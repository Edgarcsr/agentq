use crate::analyzer::Analyzer;
use crate::detect::Language;
use crate::finding::Finding;
use crate::modules::javascript::JavaScriptAnalyzer;
use std::fs;
use std::path::PathBuf;

/// The analyzer for `language`, if one exists yet.
fn analyzer_for(language: Language) -> Option<&'static dyn Analyzer> {
    match language {
        Language::JavaScriptTypeScript => Some(&JavaScriptAnalyzer),
        Language::Python | Language::Java | Language::Go => None,
    }
}

/// Runs the analyzer of each file's language and returns all findings sorted
/// by file, line and column. Files that are not valid UTF-8 are skipped.
pub fn analyze_files(files: &[(PathBuf, Language)]) -> Vec<Finding> {
    let mut findings: Vec<Finding> = files
        .iter()
        .filter_map(|(path, language)| {
            let analyzer = analyzer_for(*language)?;
            let source = fs::read_to_string(path).ok()?;
            Some(analyzer.analyze(path, &source))
        })
        .flatten()
        .collect();
    findings.sort_by(|a, b| (&a.file, a.line, a.column).cmp(&(&b.file, b.line, b.column)));
    findings
}
