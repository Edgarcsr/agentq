//! Rules for the Node.js `crypto` module.

use super::{CallSite, Detection, names};
use crate::finding::{Confidence, Purpose};

const LIBRARY: &str = "node:crypto";

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
                Some("dh") => vec![detection("dh", Purpose::KeyExchange, None)],
                _ => vec![],
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
        "createSign" | "createVerify" => match arg(0).as_str() {
            Some(name) if name.to_ascii_lowercase().starts_with("rsa-") => {
                vec![detection("rsa", Purpose::Signature, None)]
            }
            _ => vec![],
        },
        "publicEncrypt" | "privateDecrypt" | "privateEncrypt" | "publicDecrypt" => {
            vec![detection("rsa", Purpose::Encryption, None)]
        }
        "createCipheriv" | "createDecipheriv" | "createCipher" | "createDecipher" => arg(0)
            .as_str()
            .and_then(names::symmetric)
            .map(|(id, bits)| detection(id, Purpose::Encryption, bits))
            .into_iter()
            .collect(),
        "createHash" | "createHmac" | "hash" => arg(0)
            .as_str()
            .and_then(names::hash)
            .map(|id| detection(id, Purpose::Hash, None))
            .into_iter()
            .collect(),
        _ => vec![],
    }
}
