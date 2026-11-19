use agentq::analysis::analyze_directory;
use agentq::finding::Confidence;
use agentq::i18n::Lang;
use std::path::Path;

fn fixture() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/node_manifests")
}

#[test]
fn manifests_are_read_and_dependencies_of_dependencies_are_found() {
    let analysis = analyze_directory(&fixture(), Lang::En);

    let manifests: Vec<_> = analysis
        .manifests
        .iter()
        .map(|p| p.strip_prefix(fixture()).unwrap().display().to_string())
        .collect();
    // `node_modules` is never read, even for its own package.json.
    assert_eq!(manifests, ["package-lock.json", "package.json"]);

    let found: Vec<_> = analysis
        .assessments
        .iter()
        .map(|a| {
            let f = &a.finding;
            (
                f.file.file_name().unwrap().to_str().unwrap().to_string(),
                f.library.clone(),
                f.algorithm.clone(),
                f.confidence,
            )
        })
        .collect();

    // elliptic is direct, so the lockfile does not repeat it; node-rsa is only in the lockfile.
    assert!(found.contains(&(
        "package.json".into(),
        "elliptic".into(),
        "ecdsa".into(),
        Confidence::Medium
    )));
    assert!(found.contains(&(
        "package.json".into(),
        "elliptic".into(),
        "ecdh".into(),
        Confidence::Medium
    )));
    assert!(found.contains(&(
        "package-lock.json".into(),
        "node-rsa".into(),
        "rsa".into(),
        Confidence::Low
    )));
    assert!(
        !found
            .iter()
            .any(|(file, lib, ..)| file == "package-lock.json" && lib == "elliptic")
    );
    assert!(!found.iter().any(|(_, lib, ..)| lib == "node-forge"));

    // A JWT library does not say which algorithm it will use: flagged for review.
    let review: Vec<_> = analysis
        .unassessed
        .iter()
        .map(|f| (f.library.as_str(), f.confidence))
        .collect();
    assert_eq!(review, [("jsonwebtoken", Confidence::Low)]);
}

#[test]
fn dependencies_get_the_same_advice_as_calls() {
    let analysis = analyze_directory(&fixture(), Lang::En);
    let rsa = analysis
        .assessments
        .iter()
        .find(|a| a.finding.library == "node-rsa")
        .expect("node-rsa is reported");
    assert!(!rsa.recommendations.is_empty());
    assert_eq!(rsa.urgency, agentq::assess::Urgency::High);
}
