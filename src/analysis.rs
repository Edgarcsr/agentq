use crate::analyzer::Analyzer;
use crate::assess::{Assessment, assess_all};
use crate::detect::{Language, scan_source_files};
use crate::finding::Finding;
use crate::i18n::Lang;
use crate::knowledge::KnowledgeBase;
use crate::modules::javascript::JavaScriptAnalyzer;
use std::fs;
use std::path::{Path, PathBuf};

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

/// Everything found in one directory, before any filtering.
pub struct Analysis {
    /// Source files of supported languages, sorted by path.
    pub files: Vec<(PathBuf, Language)>,
    /// Every finding, assessed against the embedded knowledge base.
    pub assessments: Vec<Assessment>,
}

/// Scans `root`, analyzes every supported file and assesses the findings,
/// writing the advice in `lang`.
pub fn analyze_directory(root: &Path, lang: Lang) -> Analysis {
    let mut files = scan_source_files(root);
    files.sort_by(|a, b| a.0.cmp(&b.0));
    let findings = analyze_files(&files);
    let assessments = assess_all(&findings, KnowledgeBase::embedded(), lang);
    Analysis { files, assessments }
}
