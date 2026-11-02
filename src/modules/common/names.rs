//! Maps algorithm names as spelled in code to knowledge base ids.

use crate::finding::Purpose;

/// Hash function named by `name` (`sha-256`, `RSA-SHA1`, `sha3-512`, ...).
pub fn hash(name: &str) -> Option<&'static str> {
    let lower = name.to_ascii_lowercase();
    let compact: String = lower
        .strip_prefix("rsa-")
        .unwrap_or(&lower)
        .chars()
        .filter(|c| *c != '-' && *c != '_')
        .collect();
    if compact.starts_with("md5") {
        Some("md5")
    } else if compact == "sha1" {
        Some("sha1")
    } else if compact == "sha256" {
        Some("sha2-256")
    } else if compact == "sha384" {
        Some("sha2-384")
    } else if compact.starts_with("sha512") {
        Some("sha2-512")
    } else if compact.starts_with("sha3") {
        Some("sha3")
    } else {
        None
    }
}

/// Symmetric cipher named by `name` (`aes-256-gcm`, `des-ede3-cbc`, ...),
/// with its key size in bits when the name states it.
pub fn symmetric(name: &str) -> Option<(&'static str, Option<u32>)> {
    let lower = name.to_ascii_lowercase();
    if lower.starts_with("aes-128") || lower.starts_with("aes128") {
        Some(("aes-128", Some(128)))
    } else if lower.starts_with("aes-192") || lower.starts_with("aes192") {
        Some(("aes-192", Some(192)))
    } else if lower.starts_with("aes-256") || lower.starts_with("aes256") {
        Some(("aes-256", Some(256)))
    } else if lower.starts_with("des-ede") || lower.starts_with("des3") {
        Some(("3des", None))
    } else if lower.starts_with("chacha20") {
        Some(("chacha20", Some(256)))
    } else {
        None
    }
}

/// AES variant for a key length in bits.
pub fn aes_for_length(bits: u32) -> Option<&'static str> {
    match bits {
        128 => Some("aes-128"),
        192 => Some("aes-192"),
        256 => Some("aes-256"),
        _ => None,
    }
}

/// Algorithm behind a JOSE `alg` identifier (RFC 7518), as `(id, purpose, key size)`.
/// HMAC-based identifiers (`HS256`) and key wrapping with symmetric keys are not
/// public-key cryptography and are ignored.
pub fn jose_algorithm(name: &str) -> Option<(&'static str, Purpose, Option<u32>)> {
    match name {
        "RS256" | "RS384" | "RS512" | "PS256" | "PS384" | "PS512" => {
            Some(("rsa", Purpose::Signature, None))
        }
        "ES256" | "ES256K" => Some(("ecdsa", Purpose::Signature, Some(256))),
        "ES384" => Some(("ecdsa", Purpose::Signature, Some(384))),
        "ES512" => Some(("ecdsa", Purpose::Signature, Some(521))),
        "EdDSA" | "Ed25519" => Some(("ed25519", Purpose::Signature, Some(256))),
        "RSA-OAEP" | "RSA-OAEP-256" | "RSA-OAEP-384" | "RSA-OAEP-512" | "RSA1_5" => {
            Some(("rsa", Purpose::Encryption, None))
        }
        _ if name.starts_with("ECDH-ES") => Some(("ecdh", Purpose::KeyExchange, None)),
        _ => None,
    }
}

/// Field size in bits of a named elliptic curve.
pub fn curve_bits(name: &str) -> Option<u32> {
    match name.to_ascii_lowercase().as_str() {
        "prime256v1" | "secp256r1" | "p-256" | "p256" | "secp256k1" | "curve25519" | "x25519"
        | "ed25519" | "nistp256" => Some(256),
        "secp384r1" | "p-384" | "p384" | "nistp384" => Some(384),
        "secp521r1" | "p-521" | "p521" | "nistp521" => Some(521),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_names_in_all_spellings() {
        assert_eq!(hash("md5"), Some("md5"));
        assert_eq!(hash("SHA1"), Some("sha1"));
        assert_eq!(hash("sha-1"), Some("sha1"));
        assert_eq!(hash("RSA-SHA256"), Some("sha2-256"));
        assert_eq!(hash("SHA-384"), Some("sha2-384"));
        assert_eq!(hash("sha512-256"), Some("sha2-512"));
        assert_eq!(hash("sha3-256"), Some("sha3"));
        assert_eq!(hash("whirlpool"), None);
    }

    #[test]
    fn cipher_names() {
        assert_eq!(symmetric("aes-256-gcm"), Some(("aes-256", Some(256))));
        assert_eq!(symmetric("AES128"), Some(("aes-128", Some(128))));
        assert_eq!(symmetric("des-ede3-cbc"), Some(("3des", None)));
        assert_eq!(
            symmetric("chacha20-poly1305"),
            Some(("chacha20", Some(256)))
        );
        assert_eq!(symmetric("rc4"), None);
    }

    #[test]
    fn jose_identifiers() {
        assert_eq!(
            jose_algorithm("RS256"),
            Some(("rsa", Purpose::Signature, None))
        );
        assert_eq!(
            jose_algorithm("ES512"),
            Some(("ecdsa", Purpose::Signature, Some(521)))
        );
        assert_eq!(
            jose_algorithm("ECDH-ES+A256KW"),
            Some(("ecdh", Purpose::KeyExchange, None))
        );
        assert_eq!(
            jose_algorithm("RSA-OAEP-256"),
            Some(("rsa", Purpose::Encryption, None))
        );
        assert_eq!(jose_algorithm("HS256"), None);
        assert_eq!(jose_algorithm("none"), None);
    }

    #[test]
    fn curve_sizes() {
        assert_eq!(curve_bits("prime256v1"), Some(256));
        assert_eq!(curve_bits("P-384"), Some(384));
        assert_eq!(curve_bits("secp521r1"), Some(521));
        assert_eq!(curve_bits("unknown"), None);
    }
}
