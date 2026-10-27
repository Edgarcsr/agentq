use quantum_agent::detect::{Language, scan_source_files};
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

/// Returns the scanned files as paths relative to the fixture root, sorted for stable comparison.
fn relative_results(name: &str) -> Vec<(String, Language)> {
    let root = fixture(name);
    let mut results: Vec<(String, Language)> = scan_source_files(&root)
        .into_iter()
        .map(|(path, language)| {
            let relative = path
                .strip_prefix(&root)
                .unwrap()
                .to_string_lossy()
                .into_owned();
            (relative, language)
        })
        .collect();
    results.sort_by(|a, b| a.0.cmp(&b.0));
    results
}

#[test]
fn finds_python_files_in_nested_directories() {
    assert_eq!(
        relative_results("python_project"),
        vec![("pkg/app.py".to_string(), Language::Python)]
    );
}

#[test]
fn skips_node_modules() {
    assert_eq!(
        relative_results("js_with_node_modules"),
        vec![("index.ts".to_string(), Language::JavaScriptTypeScript)]
    );
}

#[test]
fn respects_gitignore_and_ignores_unsupported_extensions() {
    assert_eq!(
        relative_results("gitignored"),
        vec![("main.go".to_string(), Language::Go)]
    );
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
