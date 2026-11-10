//! Rules for the Web Crypto API (`crypto.subtle`), available in browsers,
//! Node.js, Deno and edge runtimes.

use super::CallSite;
use crate::finding::{Confidence, Purpose};
use crate::modules::common::Value;
use crate::modules::common::{Detection, names};

const LIBRARY: &str = "webcrypto";

fn detection(algorithm: &'static str, purpose: Purpose, key_size: Option<u32>) -> Detection {
    Detection {
        algorithm,
        purpose,
        key_size,
        confidence: Confidence::High,
        library: LIBRARY,
    }
}

/// Algorithm identifiers are either a string or an object with a `name`.
fn algorithm_name(algorithm: &Value) -> Option<&str> {
    algorithm
        .as_str()
        .or_else(|| algorithm.get("name").as_str())
}

/// Interprets one `AlgorithmIdentifier` argument.
fn from_algorithm(algorithm: &Value) -> Option<Detection> {
    let name = algorithm_name(algorithm)?.to_ascii_uppercase();
    let modulus = algorithm.get("modulusLength").as_u32();
    let curve_bits = algorithm
        .get("namedCurve")
        .as_str()
        .and_then(names::curve_bits);
    match name.as_str() {
        "RSA-OAEP" => Some(detection("rsa", Purpose::Encryption, modulus)),
        "RSASSA-PKCS1-V1_5" | "RSA-PSS" => Some(detection("rsa", Purpose::Signature, modulus)),
        "ECDSA" => Some(detection("ecdsa", Purpose::Signature, curve_bits)),
        "ECDH" => Some(detection("ecdh", Purpose::KeyExchange, curve_bits)),
        "ED25519" => Some(detection("ed25519", Purpose::Signature, Some(256))),
        "X25519" => Some(detection("x25519", Purpose::KeyExchange, Some(256))),
        "ED448" => Some(detection("ed448", Purpose::Signature, Some(448))),
        "X448" => Some(detection("x448", Purpose::KeyExchange, Some(448))),
        // The hash is a string or an object with a `name`.
        "HMAC" => algorithm_name(algorithm.get("hash"))
            .and_then(names::hash)
            .map(|id| detection(id, Purpose::Mac, None)),
        "PBKDF2" | "HKDF" => algorithm_name(algorithm.get("hash"))
            .and_then(names::hash)
            .map(|id| detection(id, Purpose::KeyDerivation, None)),
        // The key size is only known when the key is generated.
        "AES-GCM" | "AES-CBC" | "AES-CTR" | "AES-KW" => {
            let length = algorithm.get("length").as_u32()?;
            names::aes_for_length(length).map(|id| detection(id, Purpose::Encryption, Some(length)))
        }
        _ => None,
    }
}

pub fn detect(site: &CallSite) -> Vec<Detection> {
    let [subtle, function] = site.path() else {
        return vec![];
    };
    if subtle != "subtle" {
        return vec![];
    }
    // Position of the algorithm argument for each method.
    let detection = match function.as_str() {
        "generateKey" | "sign" | "verify" | "encrypt" | "decrypt" | "deriveKey" | "deriveBits" => {
            from_algorithm(site.arg(0))
        }
        "importKey" => from_algorithm(site.arg(2)),
        "wrapKey" | "unwrapKey" => from_algorithm(site.arg(3)),
        "digest" => algorithm_name(site.arg(0))
            .and_then(names::hash)
            .map(|id| self::detection(id, Purpose::Hash, None)),
        _ => None,
    };
    detection.into_iter().collect()
}
