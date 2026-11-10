//! Rules keyed by the fully qualified name of the called function.

use crate::finding::{Confidence, Purpose};
use crate::modules::common::{Detection, Value, names};

/// Positional and keyword arguments of a call.
pub struct Args<'a> {
    pub positional: &'a [Value],
    pub keywords: &'a [(String, Value)],
}

impl Args<'_> {
    /// Argument passed at `index` or as keyword `name`.
    fn get(&self, index: usize, name: &str) -> &Value {
        static UNKNOWN: Value = Value::Unknown;
        self.positional
            .get(index)
            .or_else(|| {
                self.keywords
                    .iter()
                    .find(|(key, _)| key == name)
                    .map(|(_, value)| value)
            })
            .unwrap_or(&UNKNOWN)
    }
}

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

/// Key generation shared by ECDSA and ECDH: the algorithm is a guess.
fn ec_keygen(library: &'static str, key_size: Option<u32>) -> Detection {
    Detection {
        confidence: Confidence::Medium,
        ..detection(library, "ecdsa", Purpose::Unknown, key_size)
    }
}

fn hash(library: &'static str, name: &str) -> Vec<Detection> {
    names::hash(name)
        .map(|id| detection(library, id, Purpose::Hash, None))
        .into_iter()
        .collect()
}

const ASYMMETRIC: &str = "cryptography.hazmat.primitives.asymmetric.";

pub fn detect(qualified: &str, args: &Args) -> Vec<Detection> {
    const CRYPTOGRAPHY: &str = "cryptography";
    const CRYPTODOME: &str = "pycryptodome";

    if let Some(rest) = qualified.strip_prefix(ASYMMETRIC) {
        return match rest {
            "rsa.generate_private_key" => {
                let bits = args.get(1, "key_size").as_u32();
                vec![detection(CRYPTOGRAPHY, "rsa", Purpose::Unknown, bits)]
            }
            "ec.generate_private_key" => {
                let bits = args.get(0, "curve").as_str().and_then(names::curve_bits);
                vec![ec_keygen(CRYPTOGRAPHY, bits)]
            }
            "dsa.generate_private_key" => {
                let bits = args.get(0, "key_size").as_u32();
                vec![detection(CRYPTOGRAPHY, "dsa", Purpose::Signature, bits)]
            }
            "dh.generate_parameters" => {
                let bits = args.get(1, "key_size").as_u32();
                vec![detection(CRYPTOGRAPHY, "dh", Purpose::KeyExchange, bits)]
            }
            "ed25519.Ed25519PrivateKey.generate" => {
                vec![detection(
                    CRYPTOGRAPHY,
                    "ed25519",
                    Purpose::Signature,
                    Some(256),
                )]
            }
            "x25519.X25519PrivateKey.generate" => {
                vec![detection(
                    CRYPTOGRAPHY,
                    "x25519",
                    Purpose::KeyExchange,
                    Some(256),
                )]
            }
            _ => vec![],
        };
    }

    if let Some(name) = qualified.strip_prefix("cryptography.hazmat.primitives.hashes.") {
        return hash(CRYPTOGRAPHY, name);
    }
    if let Some(name) = qualified.strip_prefix("hashlib.") {
        return match name {
            "new" | "pbkdf2_hmac" => args
                .get(0, "name")
                .as_str()
                .map_or(vec![], |algorithm| hash("hashlib", algorithm)),
            _ => hash("hashlib", name),
        };
    }
    if let Some(rest) = qualified.strip_prefix("Crypto.Hash.") {
        return match rest.split_once('.') {
            Some((name, "new")) => hash(CRYPTODOME, name),
            _ => vec![],
        };
    }

    match qualified {
        "cryptography.hazmat.primitives.ciphers.algorithms.TripleDES" => {
            vec![detection(CRYPTOGRAPHY, "3des", Purpose::Encryption, None)]
        }
        "cryptography.hazmat.primitives.ciphers.algorithms.ChaCha20" => {
            vec![detection(
                CRYPTOGRAPHY,
                "chacha20",
                Purpose::Encryption,
                Some(256),
            )]
        }
        "cryptography.hazmat.primitives.ciphers.algorithms.AES128" => {
            vec![detection(
                CRYPTOGRAPHY,
                "aes-128",
                Purpose::Encryption,
                Some(128),
            )]
        }
        "cryptography.hazmat.primitives.ciphers.algorithms.AES256" => {
            vec![detection(
                CRYPTOGRAPHY,
                "aes-256",
                Purpose::Encryption,
                Some(256),
            )]
        }
        // Fernet is AES-128-CBC with an HMAC.
        "cryptography.fernet.Fernet" | "cryptography.fernet.Fernet.generate_key" => {
            vec![detection(
                CRYPTOGRAPHY,
                "aes-128",
                Purpose::Encryption,
                Some(128),
            )]
        }

        "Crypto.PublicKey.RSA.generate" => {
            let bits = args.get(0, "bits").as_u32();
            vec![detection(CRYPTODOME, "rsa", Purpose::Unknown, bits)]
        }
        "Crypto.PublicKey.DSA.generate" => {
            let bits = args.get(0, "bits").as_u32();
            vec![detection(CRYPTODOME, "dsa", Purpose::Signature, bits)]
        }
        "Crypto.PublicKey.ECC.generate" => {
            let curve = args.get(0, "curve").as_str().unwrap_or("");
            if curve.eq_ignore_ascii_case("ed25519") {
                vec![detection(
                    CRYPTODOME,
                    "ed25519",
                    Purpose::Signature,
                    Some(256),
                )]
            } else {
                vec![ec_keygen(CRYPTODOME, names::curve_bits(curve))]
            }
        }
        "Crypto.Cipher.PKCS1_OAEP.new" | "Crypto.Cipher.PKCS1_v1_5.new" => {
            vec![detection(CRYPTODOME, "rsa", Purpose::Encryption, None)]
        }
        "Crypto.Signature.pkcs1_15.new"
        | "Crypto.Signature.pss.new"
        | "Crypto.Signature.PKCS1_v1_5.new"
        | "Crypto.Signature.PKCS1_PSS.new" => {
            vec![detection(CRYPTODOME, "rsa", Purpose::Signature, None)]
        }
        // DSS covers both DSA and ECDSA keys.
        "Crypto.Signature.DSS.new" => vec![Detection {
            confidence: Confidence::Medium,
            ..detection(CRYPTODOME, "ecdsa", Purpose::Signature, None)
        }],
        "Crypto.Cipher.DES3.new" => vec![detection(CRYPTODOME, "3des", Purpose::Encryption, None)],
        "Crypto.Cipher.ChaCha20.new" => {
            vec![detection(
                CRYPTODOME,
                "chacha20",
                Purpose::Encryption,
                Some(256),
            )]
        }

        "rsa.newkeys" => {
            let bits = args.get(0, "nbits").as_u32();
            vec![detection("rsa", "rsa", Purpose::Unknown, bits)]
        }
        "rsa.encrypt" | "rsa.decrypt" => vec![detection("rsa", "rsa", Purpose::Encryption, None)],
        "rsa.sign" | "rsa.verify" => vec![detection("rsa", "rsa", Purpose::Signature, None)],

        "ecdsa.SigningKey.generate" => {
            let bits = args.get(0, "curve").as_str().and_then(names::curve_bits);
            vec![detection("ecdsa", "ecdsa", Purpose::Signature, bits)]
        }

        "jwt.encode" => jwt(args.get(2, "algorithm")),
        "jwt.decode" => args
            .get(2, "algorithms")
            .as_array()
            .iter()
            .flat_map(jwt)
            .collect(),

        "nacl.signing.SigningKey.generate" | "nacl.signing.SigningKey" => {
            vec![detection(
                "PyNaCl",
                "ed25519",
                Purpose::Signature,
                Some(256),
            )]
        }
        "nacl.public.PrivateKey.generate" | "nacl.public.Box" => {
            vec![detection(
                "PyNaCl",
                "x25519",
                Purpose::KeyExchange,
                Some(256),
            )]
        }
        _ => vec![],
    }
}

fn jwt(algorithm: &Value) -> Vec<Detection> {
    algorithm
        .as_str()
        .and_then(names::jose_algorithm)
        .map(|(id, purpose, bits)| detection("PyJWT", id, purpose, bits))
        .into_iter()
        .collect()
}

/// Purpose implied by calls that name no algorithm of their own, used to
/// infer what generated keys are for.
pub fn hint(qualified: &str) -> Option<Purpose> {
    match qualified {
        "cryptography.hazmat.primitives.asymmetric.ec.ECDH" => Some(Purpose::KeyExchange),
        "cryptography.hazmat.primitives.asymmetric.ec.ECDSA"
        | "cryptography.hazmat.primitives.asymmetric.padding.PSS"
        | "cryptography.hazmat.primitives.asymmetric.utils.Prehashed" => Some(Purpose::Signature),
        "cryptography.hazmat.primitives.asymmetric.padding.OAEP" => Some(Purpose::Encryption),
        _ => None,
    }
}
