//! Rules for public-key libraries: `node-forge`, `node-rsa`, `elliptic`,
//! `tweetnacl` and the `@noble` family.

use super::{CallSite, Detection, names};
use crate::finding::{Confidence, Purpose};

fn detection(
    library: &'static str,
    algorithm: &'static str,
    purpose: Purpose,
    key_size: Option<u32>,
) -> Detection {
    Detection {
        algorithm,
        purpose,
        key_size,
        confidence: Confidence::High,
        library,
    }
}

pub fn detect(site: &CallSite) -> Vec<Detection> {
    let module = site.resolved.module.as_str();
    let path: Vec<&str> = site.path().iter().map(String::as_str).collect();
    match module {
        "node-forge" => forge(site, &path),
        "node-rsa" => node_rsa(site, &path),
        "elliptic" => elliptic(site, &path),
        "tweetnacl" => tweetnacl(&path),
        _ => noble(module, &path),
    }
}

/// `forge.pki.rsa.generateKeyPair(2048)`, `forge.md.sha1.create()`, ...
fn forge(site: &CallSite, path: &[&str]) -> Vec<Detection> {
    const LIB: &str = "node-forge";
    match path {
        ["pki", "rsa", "generateKeyPair"] | ["rsa", "generateKeyPair"] => {
            // Either `generateKeyPair(2048)` or `generateKeyPair({ bits: 2048 })`.
            let bits = site
                .arg(0)
                .as_u32()
                .or_else(|| site.arg(0).get("bits").as_u32());
            vec![detection(LIB, "rsa", Purpose::Unknown, bits)]
        }
        ["pki", "ed25519", "generateKeyPair"] | ["ed25519", "generateKeyPair"] => {
            vec![detection(LIB, "ed25519", Purpose::Signature, Some(256))]
        }
        ["md", name, "create"] | [name, "create"] => names::hash(name)
            .map(|id| detection(LIB, id, Purpose::Hash, None))
            .into_iter()
            .collect(),
        _ => vec![],
    }
}

/// `new NodeRSA({ b: 2048 })`
fn node_rsa(site: &CallSite, path: &[&str]) -> Vec<Detection> {
    if site.is_new && path.is_empty() {
        let bits = site.arg(0).get("b").as_u32();
        vec![detection("node-rsa", "rsa", Purpose::Unknown, bits)]
    } else {
        vec![]
    }
}

/// `new elliptic.ec('secp256k1')` and `new elliptic.eddsa('ed25519')`
fn elliptic(site: &CallSite, path: &[&str]) -> Vec<Detection> {
    if !site.is_new {
        return vec![];
    }
    match path {
        ["ec"] => {
            let bits = site.arg(0).as_str().and_then(names::curve_bits);
            // The same curve object signs (ECDSA) and agrees on keys (ECDH).
            vec![Detection {
                confidence: Confidence::Medium,
                ..detection("elliptic", "ecdsa", Purpose::Unknown, bits)
            }]
        }
        ["eddsa"] => vec![detection(
            "elliptic",
            "ed25519",
            Purpose::Signature,
            Some(256),
        )],
        _ => vec![],
    }
}

/// `nacl.sign.keyPair()`, `nacl.box(...)`, `nacl.scalarMult(...)`
fn tweetnacl(path: &[&str]) -> Vec<Detection> {
    const LIB: &str = "tweetnacl";
    match path {
        ["sign"]
        | ["sign", "keyPair"]
        | ["sign", "keyPair", "fromSeed" | "fromSecretKey"]
        | ["sign", "detached"]
        | ["sign", "detached", "verify"]
        | ["sign", "open"] => {
            vec![detection(LIB, "ed25519", Purpose::Signature, Some(256))]
        }
        ["box"]
        | ["box", "keyPair"]
        | ["box", "keyPair", "fromSecretKey"]
        | ["box", "open"]
        | ["box", "before"]
        | ["scalarMult"]
        | ["scalarMult", "base"] => {
            vec![detection(LIB, "x25519", Purpose::KeyExchange, Some(256))]
        }
        _ => vec![],
    }
}

/// `@noble/curves` (`secp256k1.sign`, `x25519.getSharedSecret`, ...) and the
/// single-curve packages `@noble/ed25519` and `@noble/secp256k1`.
fn noble(module: &str, path: &[&str]) -> Vec<Detection> {
    let library = if module.starts_with("@noble/curves") {
        "@noble/curves"
    } else if module == "@noble/ed25519" {
        "@noble/ed25519"
    } else if module == "@noble/secp256k1" {
        "@noble/secp256k1"
    } else {
        return vec![];
    };

    // The single-curve packages export their functions directly.
    let (curve, method) = match (library, path) {
        ("@noble/ed25519", [method]) => ("ed25519", *method),
        ("@noble/secp256k1", [method]) => ("secp256k1", *method),
        ("@noble/curves", [curve, method]) => (*curve, *method),
        _ => return vec![],
    };

    let bits = match curve {
        "secp256k1" | "p256" | "ed25519" | "x25519" => Some(256),
        "p384" => Some(384),
        "p521" => Some(521),
        _ => return vec![],
    };
    let signs = matches!(method, "sign" | "signAsync" | "verify" | "verifyAsync");
    match (curve, method) {
        ("ed25519", _) if signs => vec![detection(library, "ed25519", Purpose::Signature, bits)],
        ("x25519", "getSharedSecret") => {
            vec![detection(library, "x25519", Purpose::KeyExchange, bits)]
        }
        ("secp256k1" | "p256" | "p384" | "p521", _) if signs => {
            vec![detection(library, "ecdsa", Purpose::Signature, bits)]
        }
        ("secp256k1" | "p256" | "p384" | "p521", "getSharedSecret") => {
            vec![detection(library, "ecdh", Purpose::KeyExchange, bits)]
        }
        _ => vec![],
    }
}
