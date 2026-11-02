//! Static knowledge about cryptographic algorithms: how quantum attacks affect
//! them and which post-quantum algorithms can replace them.

use crate::finding::{Purpose, QuantumRisk};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::OnceLock;

const EMBEDDED: &str = include_str!("algorithms.toml");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Family {
    PublicKey,
    Symmetric,
    Hash,
    Pqc,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Attack {
    Shor,
    Grover,
    None,
}

/// One algorithm and its quantum exposure.
#[derive(Debug, Clone, Deserialize)]
pub struct Algorithm {
    pub id: String,
    pub name: String,
    pub family: Family,
    pub risk: QuantumRisk,
    pub attack: Attack,
    pub purposes: Vec<Purpose>,
    /// Ids of algorithms that can replace this one.
    pub pqc_alternatives: Vec<String>,
    pub rationale: String,
    pub notes: String,
    pub references: Vec<String>,
    /// Libraries or platforms that implement this algorithm (post-quantum entries only).
    #[serde(default)]
    pub implementations: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct KnowledgeBase {
    /// Revision of the data, as `YYYY-MM`.
    pub version: String,
    #[serde(rename = "algorithm")]
    algorithms: Vec<Algorithm>,
}

impl KnowledgeBase {
    /// The knowledge base bundled into the binary.
    pub fn embedded() -> &'static KnowledgeBase {
        static KB: OnceLock<KnowledgeBase> = OnceLock::new();
        KB.get_or_init(|| {
            let kb = Self::parse(EMBEDDED).expect("embedded knowledge base is invalid");
            kb.validate()
                .expect("embedded knowledge base is inconsistent");
            kb
        })
    }

    pub fn parse(source: &str) -> Result<KnowledgeBase, toml::de::Error> {
        toml::from_str(source)
    }

    pub fn get(&self, id: &str) -> Option<&Algorithm> {
        self.algorithms.iter().find(|algorithm| algorithm.id == id)
    }

    pub fn algorithms(&self) -> &[Algorithm] {
        &self.algorithms
    }

    /// Checks that ids are unique and every alternative points to an existing entry.
    pub fn validate(&self) -> Result<(), String> {
        let mut seen = HashSet::new();
        for algorithm in &self.algorithms {
            if !seen.insert(algorithm.id.as_str()) {
                return Err(format!("duplicate algorithm id `{}`", algorithm.id));
            }
        }
        for algorithm in &self.algorithms {
            for alternative in &algorithm.pqc_alternatives {
                if !seen.contains(alternative.as_str()) {
                    return Err(format!(
                        "`{}` lists unknown alternative `{}`",
                        algorithm.id, alternative
                    ));
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_base_is_valid() {
        let kb = KnowledgeBase::embedded();
        assert!(kb.validate().is_ok());
        assert!(kb.algorithms().len() >= 20);
    }

    #[test]
    fn public_key_algorithms_are_broken_by_shor() {
        let kb = KnowledgeBase::embedded();
        for id in ["rsa", "dh", "ecdh", "x25519", "dsa", "ecdsa", "ed25519"] {
            let algorithm = kb.get(id).unwrap_or_else(|| panic!("missing `{id}`"));
            assert_eq!(algorithm.risk, QuantumRisk::Broken, "{id}");
            assert_eq!(algorithm.attack, Attack::Shor, "{id}");
            assert!(!algorithm.pqc_alternatives.is_empty(), "{id}");
        }
    }

    #[test]
    fn key_exchange_algorithms_recommend_ml_kem() {
        let kb = KnowledgeBase::embedded();
        for id in ["dh", "ecdh", "x25519"] {
            assert_eq!(kb.get(id).unwrap().pqc_alternatives, vec!["ml-kem"], "{id}");
        }
    }

    #[test]
    fn signature_algorithms_recommend_a_signature_scheme() {
        let kb = KnowledgeBase::embedded();
        for id in ["dsa", "ecdsa", "ed25519"] {
            let alternatives = &kb.get(id).unwrap().pqc_alternatives;
            assert!(alternatives.contains(&"ml-dsa".to_string()), "{id}");
        }
    }

    #[test]
    fn post_quantum_algorithms_are_safe_and_need_no_alternative() {
        let kb = KnowledgeBase::embedded();
        for id in ["ml-kem", "ml-dsa", "slh-dsa"] {
            let algorithm = kb.get(id).unwrap();
            assert_eq!(algorithm.family, Family::Pqc, "{id}");
            assert_eq!(algorithm.risk, QuantumRisk::Safe, "{id}");
            assert!(algorithm.pqc_alternatives.is_empty(), "{id}");
        }
    }

    #[test]
    fn post_quantum_algorithms_list_implementations() {
        let kb = KnowledgeBase::embedded();
        for id in ["ml-kem", "ml-dsa", "slh-dsa"] {
            assert!(!kb.get(id).unwrap().implementations.is_empty(), "{id}");
        }
    }

    #[test]
    fn validation_rejects_unknown_alternatives_and_duplicates() {
        let entry = |id: &str, alt: &str| {
            format!(
                r#"[[algorithm]]
id = "{id}"
name = "x"
family = "hash"
risk = "safe"
attack = "none"
purposes = ["hash"]
pqc_alternatives = [{alt}]
rationale = ""
notes = ""
references = []
"#
            )
        };
        let dangling = format!("version = \"t\"\n{}", entry("a", "\"missing\""));
        assert!(KnowledgeBase::parse(&dangling).unwrap().validate().is_err());

        let duplicated = format!("version = \"t\"\n{}{}", entry("a", ""), entry("a", ""));
        assert!(
            KnowledgeBase::parse(&duplicated)
                .unwrap()
                .validate()
                .is_err()
        );
    }

    #[test]
    fn lookup_of_unknown_id_returns_none() {
        assert!(KnowledgeBase::embedded().get("rot13").is_none());
    }
}
