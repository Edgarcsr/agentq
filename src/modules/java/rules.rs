//! JCA/JCE names mapped to the existing, language-independent knowledge base.

use crate::finding::{Confidence, Purpose};
use crate::modules::common::{Detection, Value, names};

pub fn detect(class: &str, name: &str, parameter: &Value) -> Option<Detection> {
    let name = name.to_ascii_uppercase();
    let size = parameter.as_u32().filter(|size| *size > 0);
    let named = parameter.as_str().map(str::to_ascii_uppercase);
    let name = match (name.as_str(), named.as_deref()) {
        ("XDH", Some("X25519")) => "X25519",
        ("EDDSA", Some("ED25519")) => "ED25519",
        _ => &name,
    };
    let (algorithm, purpose, key_size) = match class {
        "java.security.MessageDigest" => (hash(name)?, Purpose::Hash, None),
        "javax.crypto.Mac" => (hash(name.strip_prefix("HMAC")?)?, Purpose::Mac, None),
        // `PBKDF2WithHmacSHA256`: the knowledge base assesses the underlying hash.
        "javax.crypto.SecretKeyFactory" => (
            hash(name.strip_prefix("PBKDF2WITHHMAC")?)?,
            Purpose::KeyDerivation,
            None,
        ),
        "java.security.Signature" => (signature(name)?, Purpose::Signature, curve_size(name)),
        "javax.crypto.KeyAgreement" => match name {
            "DH" | "DIFFIEHELLMAN" => ("dh", Purpose::KeyExchange, None),
            "ECDH" | "ECDHC" => ("ecdh", Purpose::KeyExchange, None),
            "X25519" => ("x25519", Purpose::KeyExchange, Some(256)),
            _ => return None,
        },
        "java.security.KeyPairGenerator" | "java.security.KeyFactory" => match name {
            "RSA" => ("rsa", Purpose::Unknown, size),
            "RSASSA-PSS" => ("rsa", Purpose::Signature, size),
            "DSA" => ("dsa", Purpose::Signature, size),
            "DH" | "DIFFIEHELLMAN" => ("dh", Purpose::KeyExchange, size),
            "EC" => (
                "ecdsa",
                Purpose::Unknown,
                size.or_else(|| parameter.as_str().and_then(names::curve_bits)),
            ),
            "ED25519" => ("ed25519", Purpose::Signature, Some(256)),
            "X25519" => ("x25519", Purpose::KeyExchange, Some(256)),
            name => post_quantum(name)?,
        },
        "javax.crypto.KEM" => match name {
            "ML-KEM" | "ML-KEM-512" | "ML-KEM-768" | "ML-KEM-1024" => {
                ("ml-kem", Purpose::KeyExchange, None)
            }
            _ => return None,
        },
        "javax.crypto.Cipher" | "javax.crypto.KeyGenerator" | "javax.crypto.spec.SecretKeySpec" => {
            let cipher = name.split('/').next()?;
            let (algorithm, bits) = match cipher {
                "RSA" if class == "javax.crypto.Cipher" => ("rsa", None),
                "AES" => (names::aes_for_length(size?)?, size),
                "AES_128" => ("aes-128", Some(128)),
                "AES_192" => ("aes-192", Some(192)),
                "AES_256" => ("aes-256", Some(256)),
                "DESEDE" | "TRIPLEDES" => ("3des", size),
                "CHACHA20" | "CHACHA20-POLY1305" => ("chacha20", Some(256)),
                _ => return None,
            };
            (algorithm, Purpose::Encryption, bits)
        }
        _ => return None,
    };
    Some(Detection {
        algorithm,
        purpose,
        key_size,
        confidence: if name == "EC" {
            Confidence::Medium
        } else {
            Confidence::High
        },
        library: if class.starts_with("javax.crypto.") {
            "javax.crypto"
        } else {
            "java.security"
        },
    })
}

/// Use exact digest names: SHA-512/256 and SHA3-224 must not inherit the
/// knowledge base's assessment of full SHA-512 or SHA3-256.
fn hash(name: &str) -> Option<&'static str> {
    match name {
        "MD5" => Some("md5"),
        "SHA" | "SHA1" | "SHA-1" => Some("sha1"),
        "SHA256" | "SHA-256" => Some("sha2-256"),
        "SHA384" | "SHA-384" => Some("sha2-384"),
        "SHA512" | "SHA-512" => Some("sha2-512"),
        "SHA3-256" | "SHA3-384" | "SHA3-512" => Some("sha3"),
        _ => None,
    }
}

fn signature(name: &str) -> Option<&'static str> {
    if let Some((_, scheme)) = name.split_once("WITH") {
        return match scheme.strip_suffix("INP1363FORMAT").unwrap_or(scheme) {
            "RSA" | "RSAANDMGF1" | "RSA/PSS" => Some("rsa"),
            "DSA" => Some("dsa"),
            "ECDSA" => Some("ecdsa"),
            _ => None,
        };
    }
    match name {
        "RSASSA-PSS" => Some("rsa"),
        "ED25519" => Some("ed25519"),
        name => post_quantum(name)
            .filter(|(_, purpose, _)| *purpose == Purpose::Signature)
            .map(|(id, _, _)| id),
    }
}

fn curve_size(name: &str) -> Option<u32> {
    (name == "ED25519").then_some(256)
}

fn post_quantum(name: &str) -> Option<(&'static str, Purpose, Option<u32>)> {
    match name {
        "ML-KEM" | "ML-KEM-512" | "ML-KEM-768" | "ML-KEM-1024" => {
            Some(("ml-kem", Purpose::KeyExchange, None))
        }
        "ML-DSA" | "ML-DSA-44" | "ML-DSA-65" | "ML-DSA-87" => {
            Some(("ml-dsa", Purpose::Signature, None))
        }
        "SLH-DSA" => Some(("slh-dsa", Purpose::Signature, None)),
        name if ["SHA2", "SHAKE"].iter().any(|hash| {
            ["128S", "128F", "192S", "192F", "256S", "256F"]
                .iter()
                .any(|set| name == format!("SLH-DSA-{hash}-{set}"))
        }) =>
        {
            Some(("slh-dsa", Purpose::Signature, None))
        }
        _ => None,
    }
}
