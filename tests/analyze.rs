use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn run(args: &[&str], fixture_name: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_agentq"))
        .env("AGENTQ_HOME", "/nonexistent/agentq-tests")
        .arg(fixture(fixture_name))
        .args(args)
        .output()
        .expect("failed to run agentq")
}

fn json_report(fixture_name: &str, args: &[&str]) -> Value {
    let mut all = vec!["--format", "json"];
    all.extend_from_slice(args);
    let output = run(&all, fixture_name);
    assert!(output.status.success());
    serde_json::from_slice(&output.stdout).expect("output is not JSON")
}

/// `(algorithm, purpose, urgency, file relative to the fixture, line, column)` per finding.
fn summarize(report: &Value) -> Vec<(String, String, String, String, u64, u64)> {
    report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|assessment| {
            let finding = &assessment["finding"];
            let file = finding["file"].as_str().unwrap();
            (
                finding["algorithm"].as_str().unwrap().to_string(),
                finding["purpose"].as_str().unwrap().to_string(),
                assessment["urgency"].as_str().unwrap().to_string(),
                file.rsplit("node_crypto/").next().unwrap().to_string(),
                finding["line"].as_u64().unwrap(),
                finding["column"].as_u64().unwrap(),
            )
        })
        .collect()
}

fn row(
    algorithm: &str,
    purpose: &str,
    urgency: &str,
    file: &str,
    line: u64,
    column: u64,
) -> (String, String, String, String, u64, u64) {
    (
        algorithm.into(),
        purpose.into(),
        urgency.into(),
        file.into(),
        line,
        column,
    )
}

#[test]
fn json_report_lists_findings_most_urgent_first() {
    let report = json_report("node_crypto", &[]);
    assert_eq!(report["summary"]["files"], 2);
    assert_eq!(report["summary"]["findings"], 5);
    assert_eq!(
        summarize(&report),
        vec![
            row("rsa", "encryption", "high", "server.js", 3, 35),
            row("rsa", "encryption", "high", "server.js", 12, 10),
            row("ecdh", "key_exchange", "high", "session.ts", 5, 16),
            row("sha1", "hash", "low", "server.js", 8, 10),
            row("aes-256", "encryption", "none", "session.ts", 8, 18),
        ]
    );
}

#[test]
fn recommendations_follow_the_purpose() {
    let report = json_report("node_crypto", &[]);
    let ecdh = &report["findings"][2];
    assert_eq!(ecdh["recommendations"][0]["id"], "ml-kem");
    assert!(ecdh["advice"][0].as_str().unwrap().contains("hybrid"));
}

#[test]
fn hide_safe_drops_safe_findings() {
    let report = json_report("node_crypto", &["--hide-safe"]);
    assert_eq!(report["summary"]["findings"], 4);
}

#[test]
fn min_confidence_drops_inferred_findings() {
    // The RSA key at server.js:3 has its purpose inferred, so it is Medium.
    let report = json_report("node_crypto", &["--min-confidence", "high"]);
    let lines: Vec<u64> = summarize(&report).iter().map(|row| row.4).collect();
    assert!(!lines.contains(&3), "{lines:?}");
    assert_eq!(report["summary"]["findings"], 4);
}

#[test]
fn text_and_markdown_formats_mention_the_findings() {
    let text = String::from_utf8(run(&[], "node_crypto").stdout).unwrap();
    assert!(text.contains("[HIGH] RSA (encryption) 2048-bit"));
    assert!(text.contains("Replace with: ML-KEM"));

    let markdown = String::from_utf8(run(&["-f", "md"], "node_crypto").stdout).unwrap();
    assert!(markdown.starts_with("# agentq report"));
    assert!(markdown.contains("## Findings"));
}

#[test]
fn output_flag_writes_the_report_to_a_file() {
    let dir = std::env::temp_dir().join(format!("agentq-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let target = dir.join("report.md");
    let output = run(
        &["--format", "markdown", "--output", target.to_str().unwrap()],
        "node_crypto",
    );
    assert!(output.status.success());
    assert!(output.stdout.is_empty());
    assert!(
        std::fs::read_to_string(&target)
            .unwrap()
            .starts_with("# agentq report")
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn directories_without_crypto_report_nothing() {
    let report = json_report("python_project", &[]);
    assert_eq!(report["summary"]["findings"], 0);
}

#[test]
fn missing_directory_is_an_error() {
    let output = Command::new(env!("CARGO_BIN_EXE_agentq"))
        .env("AGENTQ_HOME", "/nonexistent/agentq-tests")
        .arg("/definitely/not/here")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("not a directory"));
}

#[test]
fn tui_without_a_terminal_explains_what_to_do() {
    // Tests capture stdout, so the interface cannot start.
    let output = run(&["--tui"], "node_crypto");
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("needs a terminal"), "{stderr}");
    assert!(output.stdout.is_empty());
}

#[test]
fn tui_cannot_be_combined_with_a_report_format() {
    let output = run(&["--tui", "--format", "json"], "node_crypto");
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("cannot be used with"));
}

#[test]
fn sample_shop_covers_every_urgency_level() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("samples/node-shop");
    let output = Command::new(env!("CARGO_BIN_EXE_agentq"))
        .env("AGENTQ_HOME", "/nonexistent/agentq-tests")
        .arg(&root)
        .args(["--format", "json"])
        .output()
        .expect("failed to run agentq");
    assert!(output.status.success());
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();

    assert_eq!(report["summary"]["files"], 10);
    assert_eq!(report["summary"]["findings"], 22);
    let urgency = &report["summary"]["by_urgency"];
    assert_eq!(urgency["high"], 4);
    assert_eq!(urgency["medium"], 6);
    assert_eq!(urgency["low"], 6);
    assert_eq!(urgency["none"], 6);
}

#[test]
fn tui_without_a_path_also_needs_a_terminal() {
    let output = Command::new(env!("CARGO_BIN_EXE_agentq"))
        .env("AGENTQ_HOME", "/nonexistent/agentq-tests")
        .arg("--tui")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("needs a terminal"));
}

#[test]
fn without_a_path_a_report_analyzes_the_current_folder() {
    let output = Command::new(env!("CARGO_BIN_EXE_agentq"))
        .env("AGENTQ_HOME", "/nonexistent/agentq-tests")
        .current_dir(fixture("node_crypto"))
        .args(["--format", "json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["summary"]["findings"], 5);
}

/// Runs the binary on the fixture with a controlled locale.
fn run_with_locale(locale: Option<&str>, args: &[&str]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_agentq"));
    command
        .env("AGENTQ_HOME", "/nonexistent/agentq-tests")
        .arg(fixture("node_crypto"))
        .args(args)
        .env_remove("LC_ALL")
        .env_remove("LC_MESSAGES")
        .env_remove("LANG");
    if let Some(locale) = locale {
        command.env("LANG", locale);
    }
    command.output().expect("failed to run agentq")
}

fn stdout_of(output: Output) -> String {
    assert!(output.status.success());
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn lang_flag_switches_the_report_to_portuguese() {
    let text = stdout_of(run_with_locale(None, &["--lang", "pt-BR"]));
    for expected in [
        "Relatório do agentq para",
        "[ALTA] RSA (cifragem)",
        "Por quê:",
        "Substituir por: ML-KEM",
        "Esquema híbrido".to_lowercase().as_str(),
    ] {
        assert!(
            text.to_lowercase().contains(&expected.to_lowercase()),
            "missing `{expected}` in:\n{text}"
        );
    }
    // The labels and the description of each algorithm are translated.
    assert!(!text.contains("Replace with:"));
    assert!(!text.contains("Security rests on integer factoring"));
    assert!(text.contains("A segurança depende da fatoração de inteiros"));
}

#[test]
fn the_language_is_detected_from_the_environment() {
    let portuguese = stdout_of(run_with_locale(Some("pt_BR.UTF-8"), &[]));
    assert!(portuguese.contains("Relatório do agentq"), "{portuguese}");

    let english = stdout_of(run_with_locale(Some("de_DE.UTF-8"), &[]));
    assert!(english.contains("agentq report for"), "{english}");
    let english = stdout_of(run_with_locale(None, &[]));
    assert!(english.contains("agentq report for"), "{english}");
}

#[test]
fn the_flag_wins_over_the_environment() {
    let english = stdout_of(run_with_locale(Some("pt_BR.UTF-8"), &["--lang", "en"]));
    assert!(english.contains("agentq report for"), "{english}");
}

#[test]
fn json_keeps_its_field_names_and_says_the_language() {
    let report: Value = serde_json::from_str(&stdout_of(run_with_locale(
        None,
        &["--format", "json", "--lang", "pt"],
    )))
    .unwrap();
    assert_eq!(report["language"], "pt-BR");
    // Machine-readable values stay the same; only the prose is translated.
    assert_eq!(report["findings"][0]["urgency"], "high");
    assert_eq!(report["findings"][0]["finding"]["purpose"], "encryption");
    assert!(
        report["findings"][0]["explanation"]
            .as_str()
            .unwrap()
            .contains("colher agora")
    );
}

#[test]
fn markdown_is_translated_too() {
    let markdown = stdout_of(run_with_locale(
        None,
        &["--format", "md", "--lang", "pt-BR"],
    ));
    assert!(markdown.starts_with("# Relatório do agentq"), "{markdown}");
    assert!(markdown.contains("## Achados"));
}

#[test]
fn an_unknown_language_is_rejected() {
    let output = run_with_locale(None, &["--lang", "klingon"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("unknown language"));
}
