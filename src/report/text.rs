use super::{Report, implementations, key_size_label, recommendation_names};
use crate::assess::Urgency;
use crate::i18n::fill;
use std::fmt::Write;

pub fn render(report: &Report) -> String {
    let lang = report.lang;
    let t = lang.t();
    let summary = report.summary();
    let mut out = String::new();

    writeln!(out, "{}", fill(t.report_title, &[("root", &report.root)])).unwrap();
    writeln!(
        out,
        "{}\n",
        fill(t.report_kb, &[("version", &report.knowledge_version)])
    )
    .unwrap();

    writeln!(out, "{}", t.report_files).unwrap();
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
        "\n{}\n",
        fill(
            t.report_files_detected,
            &[("n", &report.files.len()), ("root", &report.root)]
        )
    )
    .unwrap();

    writeln!(out, "{}", t.report_summary).unwrap();
    writeln!(
        out,
        "  {}",
        fill(t.report_uses_found, &[("n", &summary.findings)])
    )
    .unwrap();
    for (risk, count) in &summary.by_risk {
        writeln!(out, "  {count:>4} {}", lang.risk_key(risk)).unwrap();
    }
    if summary.findings > 0 {
        let urgent = summary.by_urgency.get("high").copied().unwrap_or(0);
        writeln!(
            out,
            "  {}",
            fill(t.report_need_urgent, &[("n", &format!("{urgent:>4}"))])
        )
        .unwrap();
    }

    if !report.unassessed.is_empty() {
        writeln!(out, "\n{}", t.report_unassessed).unwrap();
        for finding in report.unassessed {
            writeln!(
                out,
                "  {}:{}:{}  {}",
                finding.file.display(),
                finding.line,
                finding.column,
                finding.snippet
            )
            .unwrap();
        }
    }

    if report.assessments.is_empty() {
        return out;
    }
    writeln!(out, "\n{}", t.report_findings_ranked).unwrap();
    for assessment in report.sorted_assessments() {
        let finding = &assessment.finding;
        writeln!(
            out,
            "\n[{}] {} ({}) {} {}",
            lang.urgency(assessment.urgency).to_uppercase(),
            assessment.algorithm_name,
            lang.purpose(finding.purpose),
            key_size_label(finding.key_size),
            format_args!(
                "{}:{}:{}  ({}: {})",
                finding.file.display(),
                finding.line,
                finding.column,
                t.confidence_label,
                lang.confidence(finding.confidence)
            )
        )
        .unwrap();
        writeln!(out, "    {}", finding.snippet).unwrap();
        if assessment.urgency == Urgency::None {
            continue;
        }
        writeln!(out, "    {}: {}", t.why, assessment.explanation).unwrap();
        let names = recommendation_names(assessment);
        if !names.is_empty() {
            writeln!(out, "    {}: {}", t.replace_with, names.join(", ")).unwrap();
            let implementations = implementations(assessment);
            if !implementations.is_empty() {
                writeln!(
                    out,
                    "    {}: {}",
                    t.implementations,
                    implementations.join(", ")
                )
                .unwrap();
            }
        }
        for advice in &assessment.advice {
            writeln!(out, "    {}: {advice}", t.note).unwrap();
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
            lang: crate::i18n::Lang::En,
            unassessed: &[],
        };
        let text = render(&report);
        assert!(text.contains("0 cryptographic use(s) found"));
        assert!(!text.contains("Findings"));
    }

    #[test]
    fn portuguese_report_translates_labels_and_advice() {
        let files = fixtures::files();
        let mut report = fixtures::report(&files);
        report.lang = crate::i18n::Lang::PtBr;
        report.assessments = crate::assess::assess_all(
            &fixtures::assessments()
                .iter()
                .map(|a| a.finding.clone())
                .collect::<Vec<_>>(),
            crate::knowledge::KnowledgeBase::embedded(),
            crate::i18n::Lang::PtBr,
        );
        let text = render(&report);
        for expected in [
            "Relatório do agentq para project",
            "base de conhecimento:",
            "4 arquivo(s) detectado(s) em project",
            "3 uso(s) de criptografia encontrado(s)",
            "quebrado",
            "precisam de atenção urgente",
            "Achados (mais urgentes primeiro)",
            "[ALTA] ECDH (NIST curves) (troca de chaves) 256-bit",
            "confiança: Alta",
            "Por quê:",
            "Substituir por: ML-KEM (CRYSTALS-Kyber)",
            "Implementações:",
            "Nota:",
            "[NENHUMA]",
        ] {
            assert!(text.contains(expected), "missing `{expected}` in:\n{text}");
        }
        assert!(!text.contains("Replace with"), "{text}");
    }
}
