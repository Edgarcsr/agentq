//! Rules for `crypto-js`, which exposes hashes as `CryptoJS.SHA256(...)` or as
//! one module per algorithm (`require('crypto-js/sha256')`).

use super::CallSite;
use crate::finding::{Confidence, Purpose};
use crate::modules::common::{Detection, names};

const LIBRARY: &str = "crypto-js";

fn detection(algorithm: &'static str, purpose: Purpose) -> Detection {
    Detection {
        algorithm,
        purpose,
        key_size: None,
        confidence: Confidence::High,
        library: LIBRARY,
    }
}

fn hash(name: &str) -> Vec<Detection> {
    let (name, purpose) = match name.strip_prefix("Hmac") {
        Some(rest) => (rest, Purpose::Mac),
        None => (name, Purpose::Hash),
    };
    names::hash(name)
        .map(|id| detection(id, purpose))
        .into_iter()
        .collect()
}

pub fn detect(site: &CallSite) -> Vec<Detection> {
    let module = site.resolved.module.as_str();
    let path: Vec<&str> = site.path().iter().map(String::as_str).collect();
    match (module, path.as_slice()) {
        // import SHA256 from 'crypto-js/sha256'; SHA256(message)
        (module, []) if module.starts_with("crypto-js/") => {
            let name = module.trim_start_matches("crypto-js/");
            match name.strip_prefix("hmac-") {
                Some(rest) => hash(&format!("Hmac{rest}")),
                None => hash(name),
            }
        }
        // CryptoJS.SHA256(message)
        ("crypto-js", [function]) => hash(function),
        // CryptoJS.TripleDES.encrypt(message, key)
        ("crypto-js", ["TripleDES", "encrypt" | "decrypt"]) => {
            vec![detection("3des", Purpose::Encryption)]
        }
        _ => vec![],
    }
}
