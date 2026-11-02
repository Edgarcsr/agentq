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

/// Resolves `Purpose::Unknown` findings of one file from the evidence in it.
///
/// Evidence is the purpose of other findings of the same key family, plus
/// `hints`: purposes of calls such as `crypto.sign` that do not name an
/// algorithm. A purpose is only assigned when the evidence points to exactly
/// one; the finding's confidence then drops to at most `Medium` since it is an
/// inference. For elliptic curves the algorithm follows the purpose
/// (`ecdsa` for signatures, `ecdh` for key exchange).
pub fn infer_purposes(findings: &mut [Finding], hints: &[Purpose]) {
    let resolved: Vec<Option<(Purpose, &'static str)>> = findings
        .iter()
        .enumerate()
        .map(|(index, finding)| {
            if finding.purpose != Purpose::Unknown {
                return None;
            }
            let family = KeyFamily::of(&finding.algorithm)?;
            let mut evidence: Vec<Purpose> = hints.to_vec();
            evidence.extend(
                findings
                    .iter()
                    .enumerate()
                    .filter(|(other, f)| {
                        *other != index && KeyFamily::of(&f.algorithm) == Some(family)
                    })
                    .map(|(_, f)| f.purpose),
            );
            evidence.retain(|purpose| family.accepts(*purpose));
            let mut distinct: Vec<Purpose> = Vec::new();
            for purpose in evidence {
                if !distinct.contains(&purpose) {
                    distinct.push(purpose);
                }
            }
            match distinct.as_slice() {
                [purpose] => Some((*purpose, algorithm_for(family, *purpose))),
                _ => None,
            }
        })
        .collect();

    for (finding, inferred) in findings.iter_mut().zip(resolved) {
        if let Some((purpose, algorithm)) = inferred {
            finding.purpose = purpose;
            finding.algorithm = algorithm.to_string();
            finding.confidence = finding.confidence.min(Confidence::Medium);
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

    #[test]
    fn rsa_key_takes_the_purpose_of_a_sibling_finding() {
        let mut findings = vec![
            finding("rsa", Purpose::Unknown),
            finding("rsa", Purpose::Encryption),
        ];
        infer_purposes(&mut findings, &[]);
        assert_eq!(findings[0].purpose, Purpose::Encryption);
        assert_eq!(findings[0].confidence, Confidence::Medium);
        assert_eq!(findings[1].confidence, Confidence::High);
    }

    #[test]
    fn rsa_key_takes_the_purpose_of_a_hint() {
        let mut findings = vec![finding("rsa", Purpose::Unknown)];
        infer_purposes(&mut findings, &[Purpose::Signature]);
        assert_eq!(findings[0].purpose, Purpose::Signature);
    }

    #[test]
    fn conflicting_evidence_leaves_the_purpose_unknown() {
        let mut findings = vec![
            finding("rsa", Purpose::Unknown),
            finding("rsa", Purpose::Encryption),
            finding("rsa", Purpose::Signature),
        ];
        infer_purposes(&mut findings, &[]);
        assert_eq!(findings[0].purpose, Purpose::Unknown);
        assert_eq!(findings[0].confidence, Confidence::High);
    }

    #[test]
    fn no_evidence_leaves_the_purpose_unknown() {
        let mut findings = vec![finding("rsa", Purpose::Unknown)];
        infer_purposes(&mut findings, &[]);
        assert_eq!(findings[0].purpose, Purpose::Unknown);
    }

    #[test]
    fn hints_that_do_not_fit_the_key_family_are_ignored() {
        let mut findings = vec![finding("rsa", Purpose::Unknown)];
        infer_purposes(&mut findings, &[Purpose::KeyExchange]);
        assert_eq!(findings[0].purpose, Purpose::Unknown);
    }

    #[test]
    fn elliptic_curve_algorithm_follows_the_inferred_purpose() {
        let mut findings = vec![finding("ecdsa", Purpose::Unknown)];
        infer_purposes(&mut findings, &[Purpose::KeyExchange]);
        assert_eq!(findings[0].algorithm, "ecdh");
        assert_eq!(findings[0].purpose, Purpose::KeyExchange);

        let mut findings = vec![finding("ecdsa", Purpose::Unknown)];
        infer_purposes(&mut findings, &[Purpose::Signature]);
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
        infer_purposes(&mut findings, &[Purpose::Encryption]);
        assert_eq!(findings, before);
    }
}
