//! A cryptographic bill of materials (CBOM): CycloneDX 1.6 with one
//! `cryptographic-asset` component per algorithm in use and, as evidence, every
//! place in the code where it was found.
//!
//! Unlike SARIF, this is an inventory: safe and post-quantum uses are listed
//! too, because knowing what is already migrated is part of the answer.

use super::{Report, urgency_name};
use crate::finding::{Finding, Purpose, QuantumRisk};
use crate::knowledge::{Family, KnowledgeBase};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::time::{SystemTime, UNIX_EPOCH};

/// CycloneDX `primitive` and `cryptoFunctions` for an algorithm used for `purpose`.
fn primitive(
    family: Option<Family>,
    algorithm: &str,
    purpose: Purpose,
) -> (&'static str, &'static [&'static str]) {
    match purpose {
        Purpose::Signature => ("signature", &["sign", "verify"]),
        Purpose::Hash => ("hash", &["digest"]),
        Purpose::Mac => ("mac", &["tag"]),
        Purpose::KeyDerivation => ("kdf", &["keyderive"]),
        Purpose::KeyExchange => match (family, algorithm) {
            (Some(Family::Pqc), _) => ("kem", &["encapsulate", "decapsulate"]),
            // RSA key transport encrypts the session key.
            (_, "rsa") => ("pke", &["encrypt", "decrypt"]),
            _ => ("key-agree", &["keygen", "keyderive"]),
        },
        Purpose::Encryption => match (family, algorithm) {
            (_, "rsa") => ("pke", &["encrypt", "decrypt"]),
            (_, "chacha20") => ("stream-cipher", &["encrypt", "decrypt"]),
            _ => ("block-cipher", &["encrypt", "decrypt"]),
        },
        Purpose::Unknown => ("unknown", &["unknown"]),
    }
}

/// Classical security in bits, from the key size, when it is a known family.
fn classical_security(algorithm: &str, key_size: Option<u32>) -> Option<u32> {
    let bits = key_size?;
    match algorithm {
        // Integer factoring and finite fields: NIST SP 800-57 equivalences.
        "rsa" | "dh" | "dsa" => Some(match bits {
            0..=1023 => 80,
            1024..=2047 => 80,
            2048..=3071 => 112,
            3072..=7679 => 128,
            7680..=15359 => 192,
            _ => 256,
        }),
        // Elliptic curves: half the field size.
        "ecdsa" | "ecdh" | "ed25519" | "x25519" | "ed448" | "x448" => Some(bits / 2),
        id if id.starts_with("aes-") => Some(bits),
        _ => None,
    }
}

/// NIST post-quantum security level (0: broken by a quantum computer).
fn quantum_level(algorithm: &str, risk: QuantumRisk, key_size: Option<u32>) -> Option<u32> {
    match (risk, algorithm, key_size) {
        (QuantumRisk::Broken, _, _) => Some(0),
        (_, "aes-128", _) => Some(1),
        (_, "aes-192", _) => Some(3),
        (_, "aes-256", _) => Some(5),
        _ => None,
    }
}

/// An RFC 4122 version 4 UUID from the standard library's random hasher.
fn uuid() -> String {
    let state = RandomState::new();
    let mut bytes = [0u8; 16];
    for (index, chunk) in bytes.chunks_mut(8).enumerate() {
        let mut hasher = state.build_hasher();
        hasher.write_usize(index);
        chunk.copy_from_slice(&hasher.finish().to_le_bytes());
    }
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let hex: Vec<String> = bytes.iter().map(|b| format!("{b:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        hex[0..4].concat(),
        hex[4..6].concat(),
        hex[6..8].concat(),
        hex[8..10].concat(),
        hex[10..16].concat()
    )
}

/// `2026-11-17T12:30:00Z` for the current time.
fn timestamp() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let (days, rest) = (seconds / 86_400, seconds % 86_400);
    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rest / 3600,
        rest % 3600 / 60,
        rest % 60
    )
}

fn occurrence(root: &str, finding: &Finding) -> Value {
    let file = finding.file.strip_prefix(root).unwrap_or(&finding.file);
    json!({
        "location": file.to_string_lossy().replace('\\', "/"),
        "line": finding.line,
        "offset": finding.column,
        "additionalContext": format!("{}: {}", finding.library, finding.snippet),
    })
}

pub fn render(report: &Report) -> String {
    let knowledge = KnowledgeBase::embedded();
    let root = report.root.as_str();

    // One asset per algorithm, key size and use.
    type Key = (String, Option<u32>, &'static str);
    let mut assets: BTreeMap<Key, Vec<&crate::assess::Assessment>> = BTreeMap::new();
    for assessment in report.sorted_assessments() {
        let finding = &assessment.finding;
        assets
            .entry((
                finding.algorithm.clone(),
                finding.key_size,
                finding.purpose.label(),
            ))
            .or_default()
            .push(assessment);
    }

    let components: Vec<Value> = assets
        .iter()
        .map(|((algorithm, key_size, _), group)| {
            let first = group[0];
            let known = knowledge.get(algorithm);
            let (primitive, functions) = primitive(
                known.map(|a| a.family),
                algorithm,
                first.finding.purpose,
            );
            let mut properties = json!({
                "primitive": primitive,
                "cryptoFunctions": functions,
            });
            if let Some(bits) = key_size {
                properties["parameterSetIdentifier"] = json!(bits.to_string());
            }
            if let Some(level) = classical_security(algorithm, *key_size) {
                properties["classicalSecurityLevel"] = json!(level);
            }
            if let Some(level) = quantum_level(algorithm, first.risk, *key_size) {
                properties["nistQuantumSecurityLevel"] = json!(level);
            }
            let reference = match key_size {
                Some(bits) => format!("crypto/algorithm/{algorithm}-{bits}/{primitive}"),
                None => format!("crypto/algorithm/{algorithm}/{primitive}"),
            };
            let urgency = group.iter().map(|a| a.urgency).max().unwrap_or(first.urgency);
            let mut component_properties = vec![
                json!({ "name": "agentq:quantumRisk", "value": super::risk_name(first.risk) }),
                json!({ "name": "agentq:urgency", "value": urgency_name(urgency) }),
            ];
            let alternatives = super::recommendation_names(first);
            if !alternatives.is_empty() {
                component_properties.push(json!({
                    "name": "agentq:replaceWith",
                    "value": alternatives.join(", "),
                }));
            }
            json!({
                "type": "cryptographic-asset",
                "bom-ref": reference,
                "name": first.algorithm_name,
                "cryptoProperties": {
                    "assetType": "algorithm",
                    "algorithmProperties": properties,
                },
                "evidence": {
                    "occurrences": group.iter().map(|a| occurrence(root, &a.finding)).collect::<Vec<_>>(),
                },
                "properties": component_properties,
            })
        })
        .collect();

    let refs: Vec<Value> = components.iter().map(|c| c["bom-ref"].clone()).collect();
    let project = std::path::Path::new(root)
        .file_name()
        .map_or_else(|| root.to_string(), |n| n.to_string_lossy().into_owned());
    let document = json!({
        "bomFormat": "CycloneDX",
        "specVersion": "1.6",
        "serialNumber": format!("urn:uuid:{}", uuid()),
        "version": 1,
        "metadata": {
            "timestamp": timestamp(),
            "tools": { "components": [{
                "type": "application",
                "name": "agentq",
                "version": env!("CARGO_PKG_VERSION"),
            }] },
            "component": { "type": "application", "name": project, "bom-ref": "project" },
            "properties": [{ "name": "agentq:knowledgeBase", "value": report.knowledge_version }],
        },
        "components": components,
        "dependencies": [{ "ref": "project", "dependsOn": refs }],
    });
    serde_json::to_string_pretty(&document).expect("CBOM is serializable")
}

#[cfg(test)]
mod tests {
    use super::super::fixtures;
    use super::*;

    fn cbom() -> Value {
        let files = fixtures::files();
        serde_json::from_str(&render(&fixtures::report(&files))).unwrap()
    }

    #[test]
    fn it_is_a_cyclonedx_1_6_document() {
        let doc = cbom();
        assert_eq!(doc["bomFormat"], "CycloneDX");
        assert_eq!(doc["specVersion"], "1.6");
        assert!(
            doc["serialNumber"]
                .as_str()
                .unwrap()
                .starts_with("urn:uuid:")
        );
        assert_eq!(
            doc["serialNumber"].as_str().unwrap().len(),
            "urn:uuid:".len() + 36
        );
        assert!(
            doc["metadata"]["timestamp"]
                .as_str()
                .unwrap()
                .ends_with('Z')
        );
    }

    #[test]
    fn every_algorithm_is_a_cryptographic_asset_with_its_occurrences() {
        let doc = cbom();
        let components = doc["components"].as_array().unwrap();
        assert!(!components.is_empty());
        for component in components {
            assert_eq!(component["type"], "cryptographic-asset");
            assert_eq!(component["cryptoProperties"]["assetType"], "algorithm");
            assert!(
                component["bom-ref"]
                    .as_str()
                    .unwrap()
                    .starts_with("crypto/algorithm/")
            );
            assert!(
                !component["evidence"]["occurrences"]
                    .as_array()
                    .unwrap()
                    .is_empty()
            );
        }
    }

    #[test]
    fn the_primitive_follows_the_purpose() {
        assert_eq!(primitive(None, "ecdsa", Purpose::Signature).0, "signature");
        assert_eq!(primitive(None, "ecdh", Purpose::KeyExchange).0, "key-agree");
        assert_eq!(primitive(None, "rsa", Purpose::KeyExchange).0, "pke");
        assert_eq!(
            primitive(Some(Family::Pqc), "ml-kem", Purpose::KeyExchange).0,
            "kem"
        );
        assert_eq!(
            primitive(None, "aes-256", Purpose::Encryption).0,
            "block-cipher"
        );
        assert_eq!(primitive(None, "sha1", Purpose::Mac).0, "mac");
    }

    #[test]
    fn security_levels_are_filled_in_when_they_are_known() {
        assert_eq!(classical_security("rsa", Some(2048)), Some(112));
        assert_eq!(classical_security("rsa", Some(3072)), Some(128));
        assert_eq!(classical_security("ecdsa", Some(256)), Some(128));
        assert_eq!(classical_security("aes-256", Some(256)), Some(256));
        assert_eq!(classical_security("md5", None), None);
        assert_eq!(
            quantum_level("rsa", QuantumRisk::Broken, Some(2048)),
            Some(0)
        );
        assert_eq!(
            quantum_level("aes-256", QuantumRisk::Weakened, None),
            Some(5)
        );
        assert_eq!(quantum_level("sha2-256", QuantumRisk::Safe, None), None);
    }

    #[test]
    fn timestamps_and_uuids_have_the_right_shape() {
        let stamp = timestamp();
        assert_eq!(stamp.len(), 20);
        assert_eq!(&stamp[4..5], "-");
        assert_eq!(&stamp[10..11], "T");
        let id = uuid();
        assert_eq!(id.len(), 36);
        assert_eq!(&id[14..15], "4");
        assert_ne!(uuid(), uuid());
    }
}
