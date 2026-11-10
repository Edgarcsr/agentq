use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// What a cryptographic operation is used for. The right post-quantum
/// replacement depends on it (e.g. ML-KEM for key exchange, ML-DSA for signatures).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Purpose {
    KeyExchange,
    Signature,
    Encryption,
    Hash,
    /// Keyed message authentication (HMAC). It does not rely on collision
    /// resistance, so a hash that is weak for digests can still be sound here.
    Mac,
    /// Deriving keys from passwords or other secrets (PBKDF2, HKDF).
    KeyDerivation,
    Unknown,
}

impl Purpose {
    pub fn label(self) -> &'static str {
        match self {
            Purpose::KeyExchange => "key exchange",
            Purpose::Signature => "signature",
            Purpose::Encryption => "encryption",
            Purpose::Hash => "hash",
            Purpose::Mac => "message authentication",
            Purpose::KeyDerivation => "key derivation",
            Purpose::Unknown => "unknown",
        }
    }
}

/// How a known quantum attack affects an algorithm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuantumRisk {
    /// Broken by Shor's algorithm in polynomial time (RSA, DH, elliptic curves).
    Broken,
    /// Security margin reduced by Grover's algorithm (symmetric ciphers, hashes).
    Weakened,
    /// Not meaningfully affected, or already a post-quantum algorithm.
    Safe,
}

/// How sure the analyzer is that a finding is a real cryptographic use.
/// Variants are ordered from least to most certain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    /// Heuristic match, e.g. an algorithm name in an unrecognized context.
    Low,
    /// Inferred from an imported library or a partially resolved value.
    Medium,
    /// Explicit call to a known cryptographic API.
    High,
}

/// One use of cryptography located in source code.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finding {
    pub file: PathBuf,
    /// 1-based line of the match.
    pub line: usize,
    /// 1-based column of the match.
    pub column: usize,
    /// Source text of the matched expression.
    pub snippet: String,
    /// Library or API that exposes the operation, e.g. `node:crypto`.
    pub library: String,
    /// Id of the algorithm in the knowledge base, e.g. `rsa`.
    pub algorithm: String,
    pub purpose: Purpose,
    /// Key or modulus size in bits, when it can be read from the code.
    pub key_size: Option<u32>,
    pub confidence: Confidence,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confidence_is_ordered_from_low_to_high() {
        assert!(Confidence::Low < Confidence::Medium);
        assert!(Confidence::Medium < Confidence::High);
    }

    #[test]
    fn enums_serialize_as_snake_case() {
        let json = serde_json::to_string(&Purpose::KeyExchange).unwrap();
        assert_eq!(json, "\"key_exchange\"");
    }
}
