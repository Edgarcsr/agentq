//! Rules for the Node.js `crypto` module.

use super::CallSite;
use crate::finding::{Confidence, Purpose};
use crate::modules::common::{Detection, Value, names};

const LIBRARY: &str = "node:crypto";

/// Id given to a call that names an algorithm missing from the knowledge base.
/// It is listed in the report for manual review instead of being dropped.
pub const UNRECOGNIZED: &str = "unrecognized";

/// A call to a known API whose literal algorithm name is not in the knowledge base.
fn unrecognized(purpose: Purpose) -> Detection {
    Detection {
        confidence: Confidence::Medium,
        ..detection(UNRECOGNIZED, purpose, None)
    }
}

/// The digest of a signing call. `RSA-SHA256`, `ecdsa-with-SHA1` and
/// `dsa-with-SHA1` also name the key type; a bare digest (`SHA256`) leaves it
/// to the key, so only the hash is reported.
fn signature_digest(name: &str) -> Vec<Detection> {
    let lower = name.to_ascii_lowercase();
    let (key, digest) = if let Some(rest) = lower.strip_prefix("rsa-") {
        (Some("rsa"), rest)
    } else if let Some(rest) = lower.strip_prefix("ecdsa-with-") {
        (Some("ecdsa"), rest)
    } else if let Some(rest) = lower.strip_prefix("dsa-with-") {
        (Some("dsa"), rest)
    } else if let Some(rest) = lower.strip_prefix("dsa-") {
        (Some("dsa"), rest)
    } else {
        (None, lower.as_str())
    };
    let mut detections: Vec<Detection> = key
        .map(|id| detection(id, Purpose::Signature, None))
        .into_iter()
        .collect();
    // With a key type the hash is part of the same finding; alone, it is the finding.
    if key.is_none() {
        detections.extend(names::hash(digest).map(|id| detection(id, Purpose::Hash, None)));
    }
    detections
}

fn detection(algorithm: &'static str, purpose: Purpose, key_size: Option<u32>) -> Detection {
    Detection {
        algorithm,
        purpose,
        key_size,
        confidence: Confidence::High,
        library: LIBRARY,
    }
}

/// Diffie-Hellman group sizes (RFC 2409 and RFC 3526) by Node group name.
fn modp_bits(group: &str) -> Option<u32> {
    match group {
        "modp1" => Some(768),
        "modp2" => Some(1024),
        "modp5" => Some(1536),
        "modp14" => Some(2048),
        "modp15" => Some(3072),
        "modp16" => Some(4096),
        "modp17" => Some(6144),
        "modp18" => Some(8192),
        _ => None,
    }
}

pub fn detect(site: &CallSite) -> Vec<Detection> {
    let [function] = site.path() else {
        return vec![];
    };
    let arg = |index| site.arg(index);

    match function.as_str() {
        "generateKeyPair" | "generateKeyPairSync" => {
            let options = arg(1);
            let modulus = options.get("modulusLength").as_u32();
            match arg(0).as_str() {
                // The key type alone does not say whether it will encrypt or sign.
                Some("rsa") => vec![detection("rsa", Purpose::Unknown, modulus)],
                Some("rsa-pss") => vec![detection("rsa", Purpose::Signature, modulus)],
                Some("dsa") => vec![detection("dsa", Purpose::Signature, modulus)],
                Some("ec") => {
                    let bits = options
                        .get("namedCurve")
                        .as_str()
                        .and_then(names::curve_bits);
                    // ECDSA and ECDH share key generation, so the algorithm is a guess.
                    vec![Detection {
                        confidence: Confidence::Medium,
                        ..detection("ecdsa", Purpose::Unknown, bits)
                    }]
                }
                Some("ed25519") => vec![detection("ed25519", Purpose::Signature, Some(256))],
                Some("x25519") => vec![detection("x25519", Purpose::KeyExchange, Some(256))],
                Some("ed448") => vec![detection("ed448", Purpose::Signature, Some(448))],
                Some("x448") => vec![detection("x448", Purpose::KeyExchange, Some(448))],
                Some("dh") => vec![detection("dh", Purpose::KeyExchange, None)],
                // Post-quantum key types, e.g. `ml-kem-768` or `ml-dsa-65`.
                Some(kind) if kind.starts_with("ml-kem-") => {
                    vec![detection("ml-kem", Purpose::KeyExchange, None)]
                }
                Some(kind) if kind.starts_with("ml-dsa-") => {
                    vec![detection("ml-dsa", Purpose::Signature, None)]
                }
                Some(kind) if kind.starts_with("slh-dsa-") => {
                    vec![detection("slh-dsa", Purpose::Signature, None)]
                }
                Some(_) => vec![unrecognized(Purpose::Unknown)],
                None => vec![],
            }
        }
        "generateKey" | "generateKeySync" => {
            let length = arg(1).get("length").as_u32();
            match (arg(0).as_str(), length.and_then(names::aes_for_length)) {
                (Some("aes"), Some(id)) => vec![detection(id, Purpose::Encryption, length)],
                _ => vec![],
            }
        }
        "createDiffieHellman" => {
            // A number is the prime length; a string or buffer is the prime itself.
            let bits = arg(0).as_u32().filter(|bits| *bits >= 512);
            vec![detection("dh", Purpose::KeyExchange, bits)]
        }
        "getDiffieHellman" | "createDiffieHellmanGroup" => {
            let bits = arg(0).as_str().and_then(modp_bits);
            vec![detection("dh", Purpose::KeyExchange, bits)]
        }
        "createECDH" => {
            let bits = arg(0).as_str().and_then(names::curve_bits);
            vec![detection("ecdh", Purpose::KeyExchange, bits)]
        }
        "createSign" | "createVerify" => arg(0).as_str().map(signature_digest).unwrap_or_default(),
        // sign(algorithm, data, key): the digest is null for Ed25519 and Ed448.
        "sign" | "verify" => arg(0).as_str().map(signature_digest).unwrap_or_default(),
        "publicEncrypt" | "privateDecrypt" | "privateEncrypt" | "publicDecrypt" => {
            vec![detection("rsa", Purpose::Encryption, None)]
        }
        "createCipheriv" | "createDecipheriv" | "createCipher" | "createDecipher" => arg(0)
            .as_str()
            .map(|name| match names::symmetric(name) {
                Some((id, bits)) => detection(id, Purpose::Encryption, bits),
                None => unrecognized(Purpose::Encryption),
            })
            .into_iter()
            .collect(),
        "createHash" | "hash" => named_hash(arg(0), Purpose::Hash),
        "createHmac" => named_hash(arg(0), Purpose::Mac),
        // pbkdf2(password, salt, iterations, keylen, digest)
        "pbkdf2" | "pbkdf2Sync" => named_hash(arg(4), Purpose::KeyDerivation),
        // hkdf(digest, ikm, salt, info, keylen)
        "hkdf" | "hkdfSync" => named_hash(arg(0), Purpose::KeyDerivation),
        _ => vec![],
    }
}

/// The hash named by a literal argument, used for `purpose`.
fn named_hash(name: &Value, purpose: Purpose) -> Vec<Detection> {
    name.as_str()
        .map(|name| match names::hash(name) {
            Some(id) => detection(id, purpose, None),
            None => unrecognized(purpose),
        })
        .into_iter()
        .collect()
}

/// Purpose implied by calls that take a key but do not name its algorithm, such
/// as `crypto.sign(null, data, key)`. Used to infer what generated keys are for.
pub fn hint(site: &CallSite) -> Option<Purpose> {
    let [function] = site.path() else {
        return None;
    };
    match function.as_str() {
        "sign" | "verify" | "createSign" | "createVerify" => Some(Purpose::Signature),
        "diffieHellman" => Some(Purpose::KeyExchange),
        _ => None,
    }
}
