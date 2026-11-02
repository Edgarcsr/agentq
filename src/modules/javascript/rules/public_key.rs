//! Rules for public-key libraries: `node-forge`, `node-rsa`, `elliptic`,
//! `tweetnacl` and the `@noble` family.

use super::CallSite;
use crate::finding::{Confidence, Purpose};
use crate::modules::common::{Detection, names};

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
        "jsrsasign" => jsrsasign(site, &path),
        "openpgp" => openpgp(site, &path),
        "libsodium-wrappers" | "libsodium-wrappers-sumo" | "sodium-native" => libsodium(&path),
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
    if module.starts_with("@noble/post-quantum") {
        return post_quantum(path);
    }
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

/// `@noble/post-quantum`: `ml_kem768.keygen()`, `ml_dsa65.sign(...)`, ...
/// These are already post-quantum; reporting them shows what is migrated.
fn post_quantum(path: &[&str]) -> Vec<Detection> {
    const LIB: &str = "@noble/post-quantum";
    let [scheme, method] = path else {
        return vec![];
    };
    if scheme.starts_with("ml_kem") && matches!(*method, "keygen" | "encapsulate" | "decapsulate") {
        vec![detection(LIB, "ml-kem", Purpose::KeyExchange, None)]
    } else if scheme.starts_with("ml_dsa") && matches!(*method, "keygen" | "sign" | "verify") {
        vec![detection(LIB, "ml-dsa", Purpose::Signature, None)]
    } else if scheme.starts_with("slh_dsa") && matches!(*method, "keygen" | "sign" | "verify") {
        vec![detection(LIB, "slh-dsa", Purpose::Signature, None)]
    } else {
        vec![]
    }
}

/// `KEYUTIL.generateKeypair('RSA', 2048)` and `KEYUTIL.generateKeypair('EC', 'secp256r1')`
fn jsrsasign(site: &CallSite, path: &[&str]) -> Vec<Detection> {
    const LIB: &str = "jsrsasign";
    if path != ["KEYUTIL", "generateKeypair"] {
        return vec![];
    }
    match site.arg(0).as_str() {
        Some("RSA") => vec![detection(
            LIB,
            "rsa",
            Purpose::Unknown,
            site.arg(1).as_u32(),
        )],
        Some("EC") => {
            let bits = site.arg(1).as_str().and_then(names::curve_bits);
            vec![Detection {
                confidence: Confidence::Medium,
                ..detection(LIB, "ecdsa", Purpose::Unknown, bits)
            }]
        }
        _ => vec![],
    }
}

/// `openpgp.generateKey({ type: 'rsa', rsaBits: 4096 })` and `{ type: 'ecc', curve: ... }`.
/// Curve25519 keys are an EdDSA signing key with an X25519 encryption subkey.
fn openpgp(site: &CallSite, path: &[&str]) -> Vec<Detection> {
    const LIB: &str = "openpgp";
    if path != ["generateKey"] {
        return vec![];
    }
    let options = site.arg(0);
    let curve = options.get("curve").as_str();
    // Without an explicit curve OpenPGP.js uses Curve25519.
    let is_25519 = curve.is_none_or(|curve| curve.to_ascii_lowercase().contains("25519"));
    match options.get("type").as_str() {
        Some("rsa") => vec![detection(
            LIB,
            "rsa",
            Purpose::Unknown,
            options.get("rsaBits").as_u32(),
        )],
        Some("ecc") if is_25519 => vec![
            detection(LIB, "ed25519", Purpose::Signature, Some(256)),
            detection(LIB, "x25519", Purpose::KeyExchange, Some(256)),
        ],
        Some("ecc") => {
            let bits = curve.and_then(names::curve_bits);
            vec![Detection {
                confidence: Confidence::Medium,
                ..detection(LIB, "ecdsa", Purpose::Unknown, bits)
            }]
        }
        Some("curve25519" | "curve25519Legacy" | "ed25519") => vec![
            detection(LIB, "ed25519", Purpose::Signature, Some(256)),
            detection(LIB, "x25519", Purpose::KeyExchange, Some(256)),
        ],
        _ => vec![],
    }
}

/// libsodium signing (Ed25519) and public-key box / key exchange (X25519) functions.
fn libsodium(path: &[&str]) -> Vec<Detection> {
    const LIB: &str = "libsodium";
    let [function] = path else {
        return vec![];
    };
    match *function {
        "crypto_sign_keypair"
        | "crypto_sign_seed_keypair"
        | "crypto_sign"
        | "crypto_sign_detached"
        | "crypto_sign_open"
        | "crypto_sign_verify_detached" => {
            vec![detection(LIB, "ed25519", Purpose::Signature, Some(256))]
        }
        "crypto_box_keypair"
        | "crypto_box_seed_keypair"
        | "crypto_box_easy"
        | "crypto_box_open_easy"
        | "crypto_box_seal"
        | "crypto_box_seal_open"
        | "crypto_scalarmult"
        | "crypto_kx_keypair"
        | "crypto_kx_client_session_keys"
        | "crypto_kx_server_session_keys" => {
            vec![detection(LIB, "x25519", Purpose::KeyExchange, Some(256))]
        }
        _ => vec![],
    }
}
