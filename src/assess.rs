//! Turns findings into assessments: how exposed each use is to quantum
//! attacks, how urgent a migration is, and what to move to.

use crate::finding::{Finding, Purpose, QuantumRisk};
use crate::i18n::Lang;
use crate::knowledge::{Attack, KnowledgeBase};
use serde::Serialize;

/// How soon a finding should be addressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Urgency {
    /// Already safe, or a post-quantum algorithm.
    None,
    Low,
    Medium,
    High,
}

/// A post-quantum (or stronger) algorithm that can replace the one found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Recommendation {
    pub id: String,
    pub name: String,
    pub implementations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Assessment {
    pub finding: Finding,
    pub algorithm_name: String,
    pub risk: QuantumRisk,
    pub attack: Attack,
    pub urgency: Urgency,
    /// Why the use is (or is not) a problem.
    pub explanation: String,
    pub recommendations: Vec<Recommendation>,
    /// Practical guidance for the migration.
    pub advice: Vec<String>,
}

/// Assesses `finding` against `knowledge`, writing the advice in `lang`.
/// Returns `None` when the algorithm is not in the knowledge base.
pub fn assess(finding: &Finding, knowledge: &KnowledgeBase, lang: Lang) -> Option<Assessment> {
    let algorithm = knowledge.get(&finding.algorithm)?;
    let urgency = urgency(algorithm.risk, finding.purpose);

    let mut explanation = algorithm.rationale(lang).to_string();
    if let Some(context) = purpose_context(algorithm.risk, finding.purpose, lang) {
        explanation.push(' ');
        explanation.push_str(context);
    }

    let recommendations =
        recommendations(finding, algorithm.pqc_alternatives.as_slice(), knowledge);

    let mut advice = Vec::new();
    if urgency != Urgency::None {
        advice.extend(migration_advice(algorithm.risk, finding.purpose, lang));
        let notes = algorithm.notes(lang);
        if !notes.is_empty() {
            advice.push(notes.to_string());
        }
    }
    if finding.purpose == Purpose::Unknown && algorithm.risk == QuantumRisk::Broken {
        advice.push(lang.t().advice_unknown_purpose.to_string());
    }

    Some(Assessment {
        finding: finding.clone(),
        algorithm_name: algorithm.name.clone(),
        risk: algorithm.risk,
        attack: algorithm.attack.clone(),
        urgency,
        explanation,
        recommendations,
        advice,
    })
}

/// Assesses every finding, dropping those whose algorithm is unknown.
pub fn assess_all(findings: &[Finding], knowledge: &KnowledgeBase, lang: Lang) -> Vec<Assessment> {
    findings
        .iter()
        .filter_map(|finding| assess(finding, knowledge, lang))
        .collect()
}

fn urgency(risk: QuantumRisk, purpose: Purpose) -> Urgency {
    match (risk, purpose) {
        (QuantumRisk::Safe, _) => Urgency::None,
        (QuantumRisk::Weakened, _) => Urgency::Low,
        // Recorded traffic can be decrypted later, so confidentiality comes first.
        // An unknown purpose is treated as the worse case.
        (QuantumRisk::Broken, Purpose::KeyExchange | Purpose::Encryption | Purpose::Unknown) => {
            Urgency::High
        }
        (QuantumRisk::Broken, Purpose::Signature | Purpose::Hash) => Urgency::Medium,
    }
}

fn purpose_context(risk: QuantumRisk, purpose: Purpose, lang: Lang) -> Option<&'static str> {
    let t = lang.t();
    match (risk, purpose) {
        (QuantumRisk::Broken, Purpose::KeyExchange | Purpose::Encryption) => Some(t.ctx_harvest),
        (QuantumRisk::Broken, Purpose::Signature) => Some(t.ctx_forge),
        _ => None,
    }
}

/// Alternatives that fit the purpose of the finding. When the purpose is
/// unknown, or no alternative fits it, all alternatives are listed.
fn recommendations(
    finding: &Finding,
    alternatives: &[String],
    knowledge: &KnowledgeBase,
) -> Vec<Recommendation> {
    let all: Vec<_> = alternatives
        .iter()
        .filter_map(|id| knowledge.get(id))
        .collect();
    let fitting: Vec<_> = all
        .iter()
        .copied()
        .filter(|alt| alt.purposes.contains(&finding.purpose))
        .collect();
    let chosen = if finding.purpose == Purpose::Unknown || fitting.is_empty() {
        all
    } else {
        fitting
    };
    chosen
        .into_iter()
        .map(|alt| Recommendation {
            id: alt.id.clone(),
            name: alt.name.clone(),
            implementations: alt.implementations.clone(),
        })
        .collect()
}

fn migration_advice(risk: QuantumRisk, purpose: Purpose, lang: Lang) -> Vec<String> {
    let t = lang.t();
    match (risk, purpose) {
        (QuantumRisk::Broken, Purpose::KeyExchange | Purpose::Encryption) => {
            vec![t.advice_hybrid.to_string()]
        }
        (QuantumRisk::Broken, Purpose::Signature) => vec![t.advice_signatures.to_string()],
        (QuantumRisk::Weakened, _) => vec![t.advice_symmetric.to_string()],
        _ => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::finding::Confidence;
    use std::path::PathBuf;

    fn finding(algorithm: &str, purpose: Purpose) -> Finding {
        Finding {
            file: PathBuf::from("a.js"),
            line: 1,
            column: 1,
            snippet: String::new(),
            library: "test".into(),
            algorithm: algorithm.into(),
            purpose,
            key_size: None,
            confidence: Confidence::High,
        }
    }

    fn assess_one(algorithm: &str, purpose: Purpose) -> Assessment {
        assess(
            &finding(algorithm, purpose),
            KnowledgeBase::embedded(),
            Lang::En,
        )
        .unwrap()
    }

    fn ids(assessment: &Assessment) -> Vec<&str> {
        assessment
            .recommendations
            .iter()
            .map(|r| r.id.as_str())
            .collect()
    }

    #[test]
    fn key_exchange_is_urgent_and_recommends_ml_kem_hybrid() {
        let a = assess_one("ecdh", Purpose::KeyExchange);
        assert_eq!(a.risk, QuantumRisk::Broken);
        assert_eq!(a.urgency, Urgency::High);
        assert_eq!(ids(&a), ["ml-kem"]);
        assert!(a.explanation.contains("harvest now, decrypt later"));
        assert!(a.advice.iter().any(|text| text.contains("hybrid")));
        assert!(!a.recommendations[0].implementations.is_empty());
    }

    #[test]
    fn signatures_are_medium_urgency_and_recommend_signature_schemes() {
        let a = assess_one("ecdsa", Purpose::Signature);
        assert_eq!(a.urgency, Urgency::Medium);
        assert_eq!(ids(&a), ["ml-dsa", "slh-dsa"]);
        assert!(a.explanation.contains("forge signatures"));
    }

    #[test]
    fn rsa_recommendation_depends_on_purpose() {
        assert_eq!(ids(&assess_one("rsa", Purpose::Encryption)), ["ml-kem"]);
        assert_eq!(
            ids(&assess_one("rsa", Purpose::Signature)),
            ["ml-dsa", "slh-dsa"]
        );
    }

    #[test]
    fn unknown_purpose_lists_every_alternative_and_asks_for_review() {
        let a = assess_one("rsa", Purpose::Unknown);
        assert_eq!(a.urgency, Urgency::High);
        assert_eq!(ids(&a), ["ml-kem", "ml-dsa", "slh-dsa"]);
        assert!(
            a.advice
                .iter()
                .any(|text| text.contains("could not be determined"))
        );
    }

    #[test]
    fn symmetric_and_hash_weaknesses_are_low_urgency() {
        let a = assess_one("aes-128", Purpose::Encryption);
        assert_eq!(a.risk, QuantumRisk::Weakened);
        assert_eq!(a.urgency, Urgency::Low);
        assert_eq!(ids(&a), ["aes-256"]);

        let md5 = assess_one("md5", Purpose::Hash);
        assert_eq!(md5.urgency, Urgency::Low);
        assert_eq!(ids(&md5), ["sha2-256", "sha3"]);
    }

    #[test]
    fn safe_algorithms_need_nothing() {
        for (algorithm, purpose) in [
            ("aes-256", Purpose::Encryption),
            ("sha2-256", Purpose::Hash),
            ("ml-kem", Purpose::KeyExchange),
        ] {
            let a = assess_one(algorithm, purpose);
            assert_eq!(a.urgency, Urgency::None, "{algorithm}");
            assert!(a.recommendations.is_empty(), "{algorithm}");
            assert!(a.advice.is_empty(), "{algorithm}");
        }
    }

    #[test]
    fn unknown_algorithms_are_dropped() {
        let findings = vec![
            finding("rot13", Purpose::Unknown),
            finding("rsa", Purpose::Signature),
        ];
        let assessments = assess_all(&findings, KnowledgeBase::embedded(), Lang::En);
        assert_eq!(assessments.len(), 1);
        assert_eq!(assessments[0].finding.algorithm, "rsa");
    }

    #[test]
    fn urgency_orders_from_none_to_high() {
        assert!(Urgency::None < Urgency::Low);
        assert!(Urgency::Low < Urgency::Medium);
        assert!(Urgency::Medium < Urgency::High);
    }

    #[test]
    fn advice_is_written_in_the_requested_language() {
        let finding = finding("ecdh", Purpose::KeyExchange);
        let portuguese = assess(&finding, KnowledgeBase::embedded(), Lang::PtBr).unwrap();
        assert!(
            portuguese
                .explanation
                .contains("colher agora, decifrar depois")
        );
        assert!(
            portuguese
                .advice
                .iter()
                .any(|text| text.contains("esquema híbrido"))
        );
        // Same decision whatever the language.
        let english = assess(&finding, KnowledgeBase::embedded(), Lang::En).unwrap();
        assert_eq!(portuguese.urgency, english.urgency);
        assert_eq!(portuguese.recommendations, english.recommendations);

        let unknown = assess(
            &super::tests::finding("rsa", Purpose::Unknown),
            KnowledgeBase::embedded(),
            Lang::PtBr,
        )
        .unwrap();
        assert!(
            unknown
                .advice
                .iter()
                .any(|text| text.contains("Não foi possível determinar"))
        );
    }
}
