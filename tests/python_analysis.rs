use quantum_agent::modules::python::{Confidence, Operation, analyze_file};
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/python_crypto")
        .join(name)
}

#[test]
fn identifies_crypto_api_candidates_and_observed_operations() {
    let findings = analyze_file(fixture("crypto_usage.py")).expect("failed to analyze fixture");

    assert_eq!(findings.len(), 4);
    assert_eq!(findings[0].line, 3);
    assert_eq!(findings[0].algorithm, "RSA");
    assert_eq!(findings[0].confidence, Confidence::Candidate);
    assert_eq!(findings[1].line, 4);
    assert_eq!(findings[1].operation, Operation::DigitalSignature);
    assert_eq!(findings[1].confidence, Confidence::Confirmed);
    assert_eq!(findings[2].line, 5);
    assert_eq!(findings[2].algorithm, "Elliptic Curve");
    assert_eq!(findings[3].line, 6);
    assert_eq!(findings[3].operation, Operation::KeyAgreement);
    assert_eq!(findings[3].confidence, Confidence::Confirmed);
}

#[test]
fn ignores_non_crypto_code_and_full_line_comments() {
    let findings = analyze_file(fixture("no_crypto.py")).expect("failed to analyze fixture");

    assert!(findings.is_empty());
}

#[test]
fn does_not_confirm_crypto_use_after_reassignment_or_partial_name_match() {
    let findings = analyze_file(fixture("ambiguous_usage.py")).expect("failed to analyze fixture");

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].line, 1);
    assert_eq!(findings[0].confidence, Confidence::Candidate);
}
