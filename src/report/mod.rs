//! Renders assessments as text, JSON or Markdown.

mod json;
mod markdown;
mod text;

use crate::assess::{Assessment, Urgency};
use crate::detect::Language;
use crate::finding::{Finding, QuantumRisk};
use crate::i18n::Lang;
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Text,
    Json,
    Markdown,
}

/// Everything a report is built from.
pub struct Report<'a> {
    /// The directory that was analyzed, as given by the user.
    pub root: String,
    pub files: &'a [(PathBuf, Language)],
    pub assessments: Vec<Assessment>,
    /// Uses whose algorithm is not in the knowledge base, sorted by location.
    pub unassessed: &'a [Finding],
    pub knowledge_version: String,
    /// Language of the labels and of the advice.
    pub lang: Lang,
}

/// Counts shown at the top of every format.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Summary {
    pub files: usize,
    pub findings: usize,
    /// Uses of algorithms that are not in the knowledge base.
    pub unassessed: usize,
    pub by_risk: BTreeMap<String, usize>,
    pub by_urgency: BTreeMap<String, usize>,
    pub by_algorithm: BTreeMap<String, usize>,
    /// Share of detected files per language, in percent.
    pub languages: BTreeMap<String, LanguageShare>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LanguageShare {
    pub files: usize,
    pub percent: f64,
}

impl Report<'_> {
    pub fn summary(&self) -> Summary {
        let mut summary = Summary {
            files: self.files.len(),
            findings: self.assessments.len(),
            unassessed: self.unassessed.len(),
            by_risk: BTreeMap::new(),
            by_urgency: BTreeMap::new(),
            by_algorithm: BTreeMap::new(),
            languages: BTreeMap::new(),
        };
        for assessment in &self.assessments {
            *summary
                .by_risk
                .entry(risk_name(assessment.risk).into())
                .or_default() += 1;
            *summary
                .by_urgency
                .entry(urgency_name(assessment.urgency).into())
                .or_default() += 1;
            *summary
                .by_algorithm
                .entry(assessment.finding.algorithm.clone())
                .or_default() += 1;
        }
        for (_, language) in self.files {
            summary
                .languages
                .entry(language.label().into())
                .or_insert(LanguageShare {
                    files: 0,
                    percent: 0.0,
                })
                .files += 1;
        }
        for share in summary.languages.values_mut() {
            share.percent = share.files as f64 * 100.0 / self.files.len() as f64;
        }
        summary
    }

    /// Assessments ordered by urgency (highest first), then by location.
    pub fn sorted_assessments(&self) -> Vec<&Assessment> {
        let mut sorted: Vec<&Assessment> = self.assessments.iter().collect();
        sort_most_urgent_first(&mut sorted);
        sorted
    }
}

/// Orders assessments by urgency (highest first), then by file, line and column.
pub fn sort_most_urgent_first(assessments: &mut [&Assessment]) {
    assessments.sort_by(|a, b| {
        b.urgency.cmp(&a.urgency).then_with(|| {
            (&a.finding.file, a.finding.line, a.finding.column).cmp(&(
                &b.finding.file,
                b.finding.line,
                b.finding.column,
            ))
        })
    });
}

pub fn render(report: &Report, format: Format) -> String {
    match format {
        Format::Text => text::render(report),
        Format::Json => json::render(report),
        Format::Markdown => markdown::render(report),
    }
}

pub(crate) fn risk_name(risk: QuantumRisk) -> &'static str {
    match risk {
        QuantumRisk::Broken => "broken",
        QuantumRisk::Weakened => "weakened",
        QuantumRisk::Safe => "safe",
    }
}

pub(crate) fn urgency_name(urgency: Urgency) -> &'static str {
    match urgency {
        Urgency::High => "high",
        Urgency::Medium => "medium",
        Urgency::Low => "low",
        Urgency::None => "none",
    }
}

/// `2048-bit`, or an empty string when the size is not known.
pub(crate) fn key_size_label(bits: Option<u32>) -> String {
    bits.map_or(String::new(), |bits| format!("{bits}-bit"))
}

/// Names of the recommended alternatives, joined for display.
pub(crate) fn recommendation_names(assessment: &Assessment) -> Vec<&str> {
    assessment
        .recommendations
        .iter()
        .map(|recommendation| recommendation.name.as_str())
        .collect()
}

/// Implementations of all recommended alternatives, without repeats.
pub(crate) fn implementations(assessment: &Assessment) -> Vec<&str> {
    let mut seen: Vec<&str> = Vec::new();
    for recommendation in &assessment.recommendations {
        for implementation in &recommendation.implementations {
            if !seen.contains(&implementation.as_str()) {
                seen.push(implementation);
            }
        }
    }
    seen
}

#[cfg(test)]
pub(crate) mod fixtures {
    use super::*;
    use crate::assess::assess_all;
    use crate::finding::{Confidence, Finding, Purpose};
    use crate::knowledge::KnowledgeBase;

    pub fn files() -> Vec<(PathBuf, Language)> {
        vec![
            (PathBuf::from("src/a.js"), Language::JavaScriptTypeScript),
            (PathBuf::from("src/b.ts"), Language::JavaScriptTypeScript),
            (PathBuf::from("main.go"), Language::Go),
            (PathBuf::from("app.py"), Language::Python),
        ]
    }

    pub fn assessments() -> Vec<Assessment> {
        let finding = |file: &str, line, algorithm: &str, purpose, key_size| Finding {
            file: PathBuf::from(file),
            line,
            column: 5,
            snippet: format!("use({algorithm}) | tricky"),
            library: "node:crypto".into(),
            algorithm: algorithm.into(),
            purpose,
            key_size,
            confidence: Confidence::High,
        };
        assess_all(
            &[
                finding("src/a.js", 10, "sha2-256", Purpose::Hash, None),
                finding("src/a.js", 3, "ecdsa", Purpose::Signature, Some(256)),
                finding("src/b.ts", 7, "ecdh", Purpose::KeyExchange, Some(256)),
            ],
            KnowledgeBase::embedded(),
            Lang::En,
        )
    }

    pub fn report(files: &[(PathBuf, Language)]) -> Report<'_> {
        Report {
            root: "project".into(),
            files,
            assessments: assessments(),
            knowledge_version: KnowledgeBase::embedded().version.clone(),
            lang: Lang::En,
            unassessed: &[],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_counts_risk_urgency_algorithm_and_language_share() {
        let files = fixtures::files();
        let summary = fixtures::report(&files).summary();
        assert_eq!(summary.files, 4);
        assert_eq!(summary.findings, 3);
        assert_eq!(summary.by_risk["broken"], 2);
        assert_eq!(summary.by_risk["safe"], 1);
        assert_eq!(summary.by_urgency["high"], 1);
        assert_eq!(summary.by_urgency["medium"], 1);
        assert_eq!(summary.by_algorithm["ecdh"], 1);
        assert_eq!(summary.languages["JS/TS"].files, 2);
        assert_eq!(summary.languages["JS/TS"].percent, 50.0);
        assert_eq!(summary.languages["Go"].percent, 25.0);
    }

    #[test]
    fn findings_are_sorted_by_urgency_then_location() {
        let files = fixtures::files();
        let report = fixtures::report(&files);
        let order: Vec<_> = report
            .sorted_assessments()
            .iter()
            .map(|a| (a.finding.algorithm.as_str(), a.finding.line))
            .collect();
        assert_eq!(order, [("ecdh", 7), ("ecdsa", 3), ("sha2-256", 10)]);
    }
}
