use quantum_agent::detect::Language;
use std::path::{Path, PathBuf};
use std::process::Command;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

/// Runs the binary on one fixture repo and returns its detected files as
/// `(language label, path relative to the fixture root)`, sorted for stable comparison.
fn run_program(name: &str) -> Vec<(String, String)> {
    let root = fixture(name);
    let output = Command::new(env!("CARGO_BIN_EXE_quantum-agent"))
        .arg(&root)
        .output()
        .expect("failed to run quantum-agent");
    assert!(
        output.status.success(),
        "quantum-agent failed on {}: {}",
        name,
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("output is not UTF-8");
    let root_prefix = format!("{}/", root.display());
    let mut results: Vec<(String, String)> = stdout
        .lines()
        .filter_map(|line| {
            let (label_and_percent, path) = line.split_once("%  ")?;
            let label = label_and_percent.split_whitespace().next()?;
            let relative = path.strip_prefix(&root_prefix)?;
            Some((label.to_string(), relative.to_string()))
        })
        .collect();
    results.sort();
    results
}

fn expected(label: &str, relative: &str) -> (String, String) {
    (label.to_string(), relative.to_string())
}

#[test]
fn finds_python_files_in_nested_directories() {
    assert_eq!(
        run_program("python_project"),
        vec![expected("Python", "pkg/app.py")]
    );
}

#[test]
fn skips_node_modules() {
    assert_eq!(
        run_program("js_with_node_modules"),
        vec![expected("JS/TS", "index.ts")]
    );
}

#[test]
fn respects_gitignore_and_ignores_unsupported_extensions() {
    assert_eq!(run_program("gitignored"), vec![expected("Go", "main.go")]);
}

#[test]
fn javascript_and_typescript_share_one_language() {
    assert_eq!(
        Language::from_extension("js"),
        Language::from_extension("tsx")
    );
    assert_eq!(
        Language::from_extension("mjs"),
        Some(Language::JavaScriptTypeScript)
    );
}
