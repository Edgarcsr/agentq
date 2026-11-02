use std::path::Path;
use std::process::Command;

/// Runs the binary on a fixture and returns the lines of its findings section.
fn finding_lines(fixture: &str) -> Vec<String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(fixture);
    let output = Command::new(env!("CARGO_BIN_EXE_agentq"))
        .arg(&root)
        .output()
        .expect("failed to run agentq");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let prefix = format!("{}/", root.display());
    stdout
        .lines()
        .skip_while(|line| !line.contains("cryptographic use(s) found"))
        .skip(1)
        .map(|line| line.replace(&prefix, ""))
        .collect()
}

#[test]
fn reports_cryptographic_uses_with_location() {
    let lines = finding_lines("node_crypto");
    assert_eq!(lines.len(), 5, "{lines:#?}");

    // Sorted by file, then position: server.js first, then session.ts.
    let expected = [
        ["rsa", "encryption", "2048-bit", "server.js:3:35"],
        ["sha1", "hash", "", "server.js:8:10"],
        ["rsa", "encryption", "", "server.js:12:10"],
        ["ecdh", "key exchange", "256-bit", "session.ts:5:16"],
        ["aes-256", "encryption", "256-bit", "session.ts:8:18"],
    ];
    for (line, parts) in lines.iter().zip(expected) {
        for part in parts {
            assert!(line.contains(part), "`{line}` should contain `{part}`");
        }
    }
}

#[test]
fn directories_without_crypto_report_nothing() {
    assert!(finding_lines("python_project").is_empty());
}
