//! Infers what a key is used for when the call that creates it does not say.
//!
//! `generateKeyPair('rsa')` or `ec.generate_private_key(...)` create keys that
//! can sign, encrypt or agree on secrets, and the right post-quantum
//! replacement depends on which. The rest of the file usually tells.

use crate::finding::{Confidence, Finding, Purpose};

/// Algorithms whose key generation is shared between several purposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KeyFamily {
    Rsa,
    EllipticCurve,
}

impl KeyFamily {
    fn of(algorithm: &str) -> Option<KeyFamily> {
        match algorithm {
            "rsa" => Some(KeyFamily::Rsa),
            "ecdsa" | "ecdh" => Some(KeyFamily::EllipticCurve),
            _ => None,
        }
    }

    /// Purposes a key of this family can have.
    fn accepts(self, purpose: Purpose) -> bool {
        match self {
            KeyFamily::Rsa => matches!(purpose, Purpose::Signature | Purpose::Encryption),
            KeyFamily::EllipticCurve => {
                matches!(purpose, Purpose::Signature | Purpose::KeyExchange)
            }
        }
    }
}

/// A call that shows what some key is used for: a finding with a known
/// purpose, or a call such as `crypto.sign` that names no algorithm.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Evidence {
    pub purpose: Purpose,
    /// Library of the call, as in [`Finding::library`].
    pub library: String,
    /// Algorithm of the finding the evidence comes from, if any.
    pub algorithm: Option<String>,
    /// Identifiers passed to the call, e.g. `privateKey` in `sign(null, data, privateKey)`.
    pub names: Vec<String>,
    /// Index of the finding this evidence comes from, if any.
    pub source: Option<usize>,
}

impl Evidence {
    /// Whether the evidence can be about a key of `family`.
    fn fits(&self, family: KeyFamily) -> bool {
        family.accepts(self.purpose)
            && self
                .algorithm
                .as_deref()
                .is_none_or(|algorithm| KeyFamily::of(algorithm) == Some(family))
    }

    fn mentions(&self, names: &[String]) -> bool {
        self.names.iter().any(|name| names.contains(name))
    }
}

/// Resolves `Purpose::Unknown` findings of one file from the evidence in it.
///
/// `bound[i]` holds the variables that receive the key created by finding `i`
/// (`const { privateKey } = generateKeyPairSync('rsa')` binds `privateKey`).
///
/// 1. **By variable.** Evidence whose call receives one of those variables
///    decides the purpose, whatever library the call belongs to.
/// 2. **By file.** Otherwise, evidence from the same library decides, unless
///    it is already explained by the variables of another key. This is a
///    weaker guess, kept for keys that are used without being named.
///
/// A purpose is only assigned when the evidence points to exactly one. Since
/// it is an inference, the finding's confidence drops to at most `Medium`
/// when decided by variable and to `Low` when decided by file. For elliptic curves the algorithm follows the purpose
/// (`ecdsa` for signatures, `ecdh` for key exchange).
pub fn infer_purposes(findings: &mut [Finding], bound: &[Vec<String>], evidence: &[Evidence]) {
    let names_of = |index: usize| bound.get(index).map(Vec::as_slice).unwrap_or(&[]);

    let resolved: Vec<Option<(Purpose, &'static str, Confidence)>> = findings
        .iter()
        .enumerate()
        .map(|(index, finding)| {
            if finding.purpose != Purpose::Unknown {
                return None;
            }
            let family = KeyFamily::of(&finding.algorithm)?;
            let candidates: Vec<&Evidence> = evidence
                .iter()
                .filter(|e| e.source != Some(index) && e.fits(family))
                .collect();

            let by_variable: Vec<Purpose> = candidates
                .iter()
                .filter(|e| e.mentions(names_of(index)))
                .map(|e| e.purpose)
                .collect();
            let (purposes, confidence) = if !by_variable.is_empty() {
                (by_variable, Confidence::Medium)
            } else {
                // Evidence that names the variables of another key is about that key.
                let claimed = |e: &Evidence| {
                    (0..findings.len()).any(|other| other != index && e.mentions(names_of(other)))
                };
                let by_file = candidates
                    .iter()
                    .filter(|e| e.library == finding.library && !claimed(e))
                    .map(|e| e.purpose)
                    .collect();
                (by_file, Confidence::Low)
            };

            let mut distinct: Vec<Purpose> = Vec::new();
            for purpose in purposes {
                if !distinct.contains(&purpose) {
                    distinct.push(purpose);
                }
            }
            match distinct.as_slice() {
                [purpose] => Some((*purpose, algorithm_for(family, *purpose), confidence)),
                _ => None,
            }
        })
        .collect();

    for (finding, inferred) in findings.iter_mut().zip(resolved) {
        if let Some((purpose, algorithm, confidence)) = inferred {
            finding.purpose = purpose;
            finding.algorithm = algorithm.to_string();
            finding.confidence = finding.confidence.min(confidence);
        }
    }
}

fn algorithm_for(family: KeyFamily, purpose: Purpose) -> &'static str {
    match (family, purpose) {
        (KeyFamily::Rsa, _) => "rsa",
        (KeyFamily::EllipticCurve, Purpose::KeyExchange) => "ecdh",
        (KeyFamily::EllipticCurve, _) => "ecdsa",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

    /// Evidence from every finding with a known purpose, plus `hints`, none
    /// of them naming a variable: what the analyzer sees in a file where keys
    /// are never bound to names.
    fn evidence_of(findings: &[Finding], hints: &[Purpose]) -> Vec<Evidence> {
        let from_findings = findings
            .iter()
            .enumerate()
            .filter(|(_, f)| f.purpose != Purpose::Unknown)
            .map(|(index, f)| Evidence {
                purpose: f.purpose,
                library: f.library.clone(),
                algorithm: Some(f.algorithm.clone()),
                names: vec![],
                source: Some(index),
            });
        let from_hints = hints.iter().map(|purpose| hint(*purpose, &[]));
        from_findings.chain(from_hints).collect()
    }

    fn hint(purpose: Purpose, names: &[&str]) -> Evidence {
        Evidence {
            purpose,
            library: "test".into(),
            algorithm: None,
            names: names.iter().map(|name| name.to_string()).collect(),
            source: None,
        }
    }

    fn infer(findings: &mut [Finding], hints: &[Purpose]) {
        let evidence = evidence_of(findings, hints);
        let bound = vec![vec![]; findings.len()];
        infer_purposes(findings, &bound, &evidence);
    }

    #[test]
    fn rsa_key_takes_the_purpose_of_a_sibling_finding() {
        let mut findings = vec![
            finding("rsa", Purpose::Unknown),
            finding("rsa", Purpose::Encryption),
        ];
        infer(&mut findings, &[]);
        assert_eq!(findings[0].purpose, Purpose::Encryption);
        // No variable links them, so this is only a file-level guess.
        assert_eq!(findings[0].confidence, Confidence::Low);
        assert_eq!(findings[1].confidence, Confidence::High);
    }

    #[test]
    fn rsa_key_takes_the_purpose_of_a_hint() {
        let mut findings = vec![finding("rsa", Purpose::Unknown)];
        infer(&mut findings, &[Purpose::Signature]);
        assert_eq!(findings[0].purpose, Purpose::Signature);
    }

    #[test]
    fn conflicting_evidence_leaves_the_purpose_unknown() {
        let mut findings = vec![
            finding("rsa", Purpose::Unknown),
            finding("rsa", Purpose::Encryption),
            finding("rsa", Purpose::Signature),
        ];
        infer(&mut findings, &[]);
        assert_eq!(findings[0].purpose, Purpose::Unknown);
        assert_eq!(findings[0].confidence, Confidence::High);
    }

    #[test]
    fn no_evidence_leaves_the_purpose_unknown() {
        let mut findings = vec![finding("rsa", Purpose::Unknown)];
        infer(&mut findings, &[]);
        assert_eq!(findings[0].purpose, Purpose::Unknown);
    }

    #[test]
    fn hints_that_do_not_fit_the_key_family_are_ignored() {
        let mut findings = vec![finding("rsa", Purpose::Unknown)];
        infer(&mut findings, &[Purpose::KeyExchange]);
        assert_eq!(findings[0].purpose, Purpose::Unknown);
    }

    #[test]
    fn elliptic_curve_algorithm_follows_the_inferred_purpose() {
        let mut findings = vec![finding("ecdsa", Purpose::Unknown)];
        infer(&mut findings, &[Purpose::KeyExchange]);
        assert_eq!(findings[0].algorithm, "ecdh");
        assert_eq!(findings[0].purpose, Purpose::KeyExchange);

        let mut findings = vec![finding("ecdsa", Purpose::Unknown)];
        infer(&mut findings, &[Purpose::Signature]);
        assert_eq!(findings[0].algorithm, "ecdsa");
    }

    #[test]
    fn explicit_purposes_and_other_algorithms_are_untouched() {
        let mut findings = vec![
            finding("rsa", Purpose::Signature),
            finding("sha1", Purpose::Hash),
            finding("ed25519", Purpose::Signature),
        ];
        let before = findings.clone();
        infer(&mut findings, &[Purpose::Encryption]);
        assert_eq!(findings, before);
    }

    #[test]
    fn the_call_that_receives_the_key_variable_decides() {
        // Two RSA keys in one file: one signs, the other encrypts.
        let mut findings = vec![
            finding("rsa", Purpose::Unknown),
            finding("rsa", Purpose::Unknown),
        ];
        let bound = vec![vec!["signingKey".to_string()], vec!["sealKey".to_string()]];
        let evidence = vec![
            hint(Purpose::Signature, &["data", "signingKey"]),
            hint(Purpose::Encryption, &["sealKey", "secret"]),
        ];
        infer_purposes(&mut findings, &bound, &evidence);
        assert_eq!(findings[0].purpose, Purpose::Signature);
        assert_eq!(findings[1].purpose, Purpose::Encryption);
        assert_eq!(findings[0].confidence, Confidence::Medium);
    }

    #[test]
    fn evidence_about_another_key_is_not_reused() {
        // The signature is explained by `signingKey`, so it says nothing
        // about the second key, which no call receives.
        let mut findings = vec![
            finding("rsa", Purpose::Unknown),
            finding("rsa", Purpose::Unknown),
        ];
        let bound = vec![vec!["signingKey".to_string()], vec!["other".to_string()]];
        let evidence = vec![hint(Purpose::Signature, &["signingKey"])];
        infer_purposes(&mut findings, &bound, &evidence);
        assert_eq!(findings[0].purpose, Purpose::Signature);
        assert_eq!(findings[1].purpose, Purpose::Unknown);
    }

    #[test]
    fn another_library_only_counts_through_the_variable() {
        let mut findings = vec![finding("rsa", Purpose::Unknown)];
        let mut jwt = hint(Purpose::Signature, &[]);
        jwt.library = "jsonwebtoken".into();
        jwt.algorithm = Some("rsa".into());
        infer_purposes(&mut findings, &[vec![]], std::slice::from_ref(&jwt));
        assert_eq!(findings[0].purpose, Purpose::Unknown);

        jwt.names = vec!["privateKey".into()];
        infer_purposes(&mut findings, &[vec!["privateKey".into()]], &[jwt]);
        assert_eq!(findings[0].purpose, Purpose::Signature);
    }
}
