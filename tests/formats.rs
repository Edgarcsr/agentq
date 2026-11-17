use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn root(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(name)
}

fn run(folder: &str, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_agentq"))
        .env("AGENTQ_HOME", "/nonexistent/agentq-tests")
        .arg(root(folder))
        .args(args)
        .output()
        .expect("failed to run agentq")
}

fn json(folder: &str, args: &[&str]) -> Value {
    let output = run(folder, args);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("output is not JSON")
}

#[test]
fn sarif_lists_what_needs_action_with_paths_relative_to_the_folder() {
    let doc = json("samples/node-shop", &["--format", "sarif"]);
    assert_eq!(doc["version"], "2.1.0");
    let run = &doc["runs"][0];
    let results = run["results"].as_array().unwrap();

    // 27 uses, 9 of them safe and left out, plus the jsonwebtoken dependency to review.
    assert_eq!(results.len(), 27 - 9 + 1);
    assert!(results.iter().all(|r| r["properties"]["urgency"] != "none"));
    assert!(results.iter().any(|r| r["level"] == "error"));

    let uris: Vec<&str> = results
        .iter()
        .map(|r| {
            r["locations"][0]["physicalLocation"]["artifactLocation"]["uri"]
                .as_str()
                .unwrap()
        })
        .collect();
    assert!(
        uris.iter()
            .all(|uri| !uri.starts_with('/') && !uri.contains("samples"))
    );
    assert!(uris.contains(&"package.json"));
    assert!(uris.iter().any(|uri| uri.starts_with("src/")));
    assert!(
        run["originalUriBaseIds"]["%SRCROOT%"]["uri"]
            .as_str()
            .unwrap()
            .ends_with('/')
    );
}

#[test]
fn sarif_sends_algorithms_chosen_at_runtime_to_review() {
    let doc = json("samples/node-shop", &["--format", "sarif"]);
    let results = doc["runs"][0]["results"].as_array().unwrap();
    let review: Vec<_> = results
        .iter()
        .filter(|r| r["ruleId"] == "agentq/unrecognized")
        .collect();
    // jsonwebtoken in package.json.
    assert_eq!(review.len(), 1);
    assert_eq!(review[0]["level"], "note");
}

#[test]
fn cbom_is_an_inventory_that_includes_what_is_already_post_quantum() {
    let doc = json("samples/node-shop", &["--format", "cbom"]);
    assert_eq!(doc["bomFormat"], "CycloneDX");
    assert_eq!(doc["specVersion"], "1.6");
    let components = doc["components"].as_array().unwrap();
    let names: Vec<&str> = components
        .iter()
        .map(|c| c["name"].as_str().unwrap())
        .collect();
    assert!(names.iter().any(|n| n.contains("RSA")));
    assert!(names.iter().any(|n| n.contains("ML-KEM")), "{names:?}");

    let rsa = components
        .iter()
        .find(|c| c["bom-ref"] == "crypto/algorithm/rsa-2048/pke")
        .expect("RSA-2048 key transport is an asset");
    let props = &rsa["cryptoProperties"]["algorithmProperties"];
    assert_eq!(props["primitive"], "pke");
    assert_eq!(props["parameterSetIdentifier"], "2048");
    assert_eq!(props["classicalSecurityLevel"], 112);
    assert_eq!(props["nistQuantumSecurityLevel"], 0);
    assert!(rsa["evidence"]["occurrences"][0]["line"].as_u64().unwrap() > 0);
    // Every asset is listed as a dependency of the project.
    assert_eq!(
        doc["dependencies"][0]["dependsOn"]
            .as_array()
            .unwrap()
            .len(),
        components.len()
    );
}

#[test]
fn fail_on_sets_the_exit_status_for_ci() {
    // The sample has high urgency uses.
    let output = run("samples/node-shop", &["--fail-on", "high"]);
    assert_eq!(output.status.code(), Some(3));
    assert!(String::from_utf8_lossy(&output.stderr).contains("--fail-on"));
    // The report is still written.
    assert!(String::from_utf8_lossy(&output.stdout).contains("agentq report"));

    // Nothing to analyze: nothing to fail on.
    assert_eq!(
        run("tests/fixtures/mixed_languages", &["--fail-on", "low"])
            .status
            .code(),
        Some(0)
    );
    // Without the flag the exit status is 0 even with findings.
    assert_eq!(run("samples/node-shop", &[]).status.code(), Some(0));
}

#[test]
fn fail_on_follows_the_filters() {
    // Dependencies have medium or low confidence, so --min-confidence high drops them all.
    let output = run(
        "tests/fixtures/node_manifests",
        &["--fail-on", "high", "--min-confidence", "high"],
    );
    assert_eq!(output.status.code(), Some(0));
}
