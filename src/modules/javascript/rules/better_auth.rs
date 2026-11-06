//! Rules for Better Auth (`better-auth`).
//!
//! The `jwt` plugin signs tokens with a key pair published through JWKS. The
//! algorithm comes from `jwks.keyPairConfig.alg` and is EdDSA (Ed25519) when
//! the option is left out. Password hashing (scrypt) and session tokens
//! (HMAC) are symmetric, so they create no finding.

use super::CallSite;
use crate::finding::Confidence;
use crate::modules::common::{Detection, Value, names};

const LIBRARY: &str = "better-auth";
/// Algorithm the `jwt` plugin uses when none is configured.
const DEFAULT_ALG: &str = "EdDSA";

pub fn detect(site: &CallSite) -> Vec<Detection> {
    let [function] = site.path() else {
        return vec![];
    };
    if function != "jwt" {
        return vec![];
    }
    let config = site.arg(0).get("jwks").get("keyPairConfig");
    let alg = match config.get("alg") {
        Value::Str(alg) => alg.as_str(),
        // Written out, but not readable statically: say nothing rather than guess.
        _ if !site.args.is_empty() && has_option(site.arg(0), "jwks") => return vec![],
        _ => DEFAULT_ALG,
    };
    names::jose_algorithm(alg)
        .map(|(algorithm, purpose, key_size)| Detection {
            algorithm,
            purpose,
            key_size,
            confidence: Confidence::High,
            library: LIBRARY,
        })
        .into_iter()
        .collect()
}

/// Whether an object literal sets `key`, or the options are not a literal.
fn has_option(options: &Value, key: &str) -> bool {
    match options {
        Value::Object(pairs) => pairs.iter().any(|(name, _)| name == key),
        Value::Unknown => true,
        _ => false,
    }
}
