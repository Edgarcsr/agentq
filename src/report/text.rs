use super::{Report, implementations, key_size_label, recommendation_names, urgency_name};
use crate::assess::Urgency;
use std::fmt::Write;

pub fn render(report: &Report) -> String {
    let summary = report.summary();
    let mut out = String::new();

    writeln!(out, "agentq report for {}", report.root).unwrap();
    writeln!(out, "knowledge base: {}\n", report.knowledge_version).unwrap();

    writeln!(out, "Files").unwrap();
    for (path, language) in report.files {
        writeln!(
            out,
            "{:<8} {:>5.1}%  {}",
            language.label(),
            summary.languages[language.label()].percent,
            path.display()
        )
        .unwrap();
    }
    writeln!(
        out,
        "\n{} file(s) detected in {}\n",
        report.files.len(),
        report.root
    )
    .unwrap();

    writeln!(out, "Summary").unwrap();
    writeln!(out, "  {} cryptographic use(s) found", summary.findings).unwrap();
    for (risk, count) in &summary.by_risk {
        writeln!(out, "  {count:>4} {risk}").unwrap();
    }
    if summary.findings > 0 {
        let urgent = summary.by_urgency.get("high").copied().unwrap_or(0);
        writeln!(out, "  {urgent:>4} need urgent attention").unwrap();
    }

    if report.assessments.is_empty() {
        return out;
    }
    writeln!(out, "\nFindings (most urgent first)").unwrap();
    for assessment in report.sorted_assessments() {
        let finding = &assessment.finding;
        writeln!(
            out,
            "\n[{}] {} ({}) {} {}",
            urgency_name(assessment.urgency).to_uppercase(),
            assessment.algorithm_name,
            finding.purpose.label(),
            key_size_label(finding.key_size),
            format_args!(
                "{}:{}:{}  (confidence: {:?})",
                finding.file.display(),
                finding.line,
                finding.column,
                finding.confidence
            )
        )
        .unwrap();
        writeln!(out, "    {}", finding.snippet).unwrap();
        if assessment.urgency == Urgency::None {
            continue;
        }
        writeln!(out, "    Why: {}", assessment.explanation).unwrap();
        let names = recommendation_names(assessment);
        if !names.is_empty() {
            writeln!(out, "    Replace with: {}", names.join(", ")).unwrap();
            let implementations = implementations(assessment);
            if !implementations.is_empty() {
                writeln!(out, "    Implementations: {}", implementations.join(", ")).unwrap();
            }
        }
        for advice in &assessment.advice {
            writeln!(out, "    Note: {advice}").unwrap();
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::super::fixtures;
    use super::*;

    #[test]
    fn text_report_lists_files_summary_and_findings() {
        let files = fixtures::files();
        let text = render(&fixtures::report(&files));
        assert!(text.contains("agentq report for project"));
        assert!(text.contains("JS/TS     50.0%  src/a.js"));
        assert!(text.contains("3 cryptographic use(s) found"));
        assert!(text.contains("[HIGH] ECDH (NIST curves) (key exchange) 256-bit src/b.ts:7:5"));
        assert!(text.contains("Replace with: ML-KEM (CRYSTALS-Kyber)"));
        // The urgent finding comes before the safe one.
        assert!(text.find("src/b.ts:7:5").unwrap() < text.find("src/a.js:10:5").unwrap());
    }

    #[test]
    fn safe_findings_are_listed_without_advice() {
        let files = fixtures::files();
        let text = render(&fixtures::report(&files));
        let safe = &text[text.find("[NONE]").unwrap()..];
        assert!(!safe.contains("Why:"));
        assert!(!safe.contains("Replace with"));
    }

    #[test]
    fn empty_report_has_no_findings_section() {
        let report = Report {
            root: "empty".into(),
            files: &[],
            assessments: vec![],
            knowledge_version: "t".into(),
        };
        let text = render(&report);
        assert!(text.contains("0 cryptographic use(s) found"));
        assert!(!text.contains("Findings"));
    }
}
