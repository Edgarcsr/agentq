use super::{Report, implementations, key_size_label, recommendation_names};
use std::fmt::Write;

/// Escapes text for use inside a table cell.
fn cell(text: &str) -> String {
    text.replace('|', "\\|").replace('`', "'")
}

pub fn render(report: &Report) -> String {
    let lang = report.lang;
    let t = lang.t();
    let summary = report.summary();
    let mut out = String::new();

    writeln!(out, "# {}\n", t.md_title).unwrap();
    writeln!(out, "- **{}:** `{}`", t.md_analyzed, report.root).unwrap();
    writeln!(out, "- **{}:** {}", t.md_files_detected, summary.files).unwrap();
    writeln!(out, "- **{}:** {}", t.md_uses_found, summary.findings).unwrap();
    writeln!(
        out,
        "- **{}:** {}\n",
        t.md_knowledge_base, report.knowledge_version
    )
    .unwrap();

    writeln!(out, "## {}\n", t.report_summary).unwrap();
    writeln!(out, "| {} | {} |\n| --- | ---: |", t.md_risk, t.md_count).unwrap();
    for (risk, count) in &summary.by_risk {
        writeln!(out, "| {} | {count} |", lang.risk_key(risk)).unwrap();
    }
    writeln!(
        out,
        "\n| {} | {} | {} |\n| --- | ---: | ---: |",
        t.md_language, t.report_files, t.md_share
    )
    .unwrap();
    for (language, share) in &summary.languages {
        writeln!(
            out,
            "| {language} | {} | {:.1}% |",
            share.files, share.percent
        )
        .unwrap();
    }

    if !report.unassessed.is_empty() {
        writeln!(out, "\n## {}\n", t.report_unassessed).unwrap();
        for finding in report.unassessed {
            writeln!(
                out,
                "- `{}:{}` `{}`",
                finding.file.display(),
                finding.line,
                finding.snippet.replace('`', "'")
            )
            .unwrap();
        }
    }

    if report.assessments.is_empty() {
        return out;
    }

    writeln!(out, "\n## {}\n", t.md_findings).unwrap();
    writeln!(
        out,
        "| {} | {} | {} | {} | {} | {} |\n\
         | --- | --- | --- | --- | --- | --- |",
        t.md_urgency, t.md_algorithm, t.md_purpose, t.md_key_size, t.md_location, t.replace_with
    )
    .unwrap();
    let sorted = report.sorted_assessments();
    for assessment in &sorted {
        let finding = &assessment.finding;
        writeln!(
            out,
            "| {} | {} | {} | {} | `{}:{}` | {} |",
            lang.urgency(assessment.urgency),
            cell(&assessment.algorithm_name),
            lang.purpose(finding.purpose),
            key_size_label(finding.key_size),
            finding.file.display(),
            finding.line,
            cell(&recommendation_names(assessment).join(", "))
        )
        .unwrap();
    }

    writeln!(out, "\n## {}\n", t.md_details).unwrap();
    for assessment in sorted {
        let finding = &assessment.finding;
        writeln!(
            out,
            "### {} {} `{}:{}:{}`\n",
            assessment.algorithm_name,
            t.md_at,
            finding.file.display(),
            finding.line,
            finding.column
        )
        .unwrap();
        writeln!(out, "```\n{}\n```\n", finding.snippet).unwrap();
        writeln!(
            out,
            "- **{}:** {}\n- **{}:** {}\n- **{}:** {} ({:?})",
            t.md_library,
            finding.library,
            t.md_confidence,
            lang.confidence(finding.confidence),
            t.md_quantum_risk,
            lang.risk(assessment.risk),
            assessment.attack
        )
        .unwrap();
        writeln!(out, "- **{}:** {}", t.why, assessment.explanation).unwrap();
        let names = recommendation_names(assessment);
        if !names.is_empty() {
            writeln!(out, "- **{}:** {}", t.replace_with, names.join(", ")).unwrap();
            let implementations = implementations(assessment);
            if !implementations.is_empty() {
                writeln!(
                    out,
                    "- **{}:** {}",
                    t.implementations,
                    implementations.join(", ")
                )
                .unwrap();
            }
        }
        for advice in &assessment.advice {
            writeln!(out, "- **{}:** {advice}", t.note).unwrap();
        }
        writeln!(out).unwrap();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::super::fixtures;
    use super::*;

    #[test]
    fn markdown_report_has_summary_table_and_details() {
        let files = fixtures::files();
        let md = render(&fixtures::report(&files));
        assert!(md.starts_with("# agentq report"));
        assert!(md.contains("| broken | 2 |"));
        assert!(md.contains("| JS/TS | 2 | 50.0% |"));
        assert!(
            md.contains("| high | ECDH (NIST curves) | key exchange | 256-bit | `src/b.ts:7` |")
        );
        assert!(md.contains("### ECDSA at `src/a.js:3:5`"));
        assert!(md.contains("- **Replace with:** ML-DSA"));
    }

    #[test]
    fn pipes_in_table_cells_are_escaped() {
        assert_eq!(cell("a | b `c`"), "a \\| b 'c'");
    }

    #[test]
    fn portuguese_markdown_translates_headings_and_columns() {
        let files = fixtures::files();
        let mut report = fixtures::report(&files);
        report.lang = crate::i18n::Lang::PtBr;
        let md = render(&report);
        for expected in [
            "# Relatório do agentq",
            "- **Analisado:** `project`",
            "## Resumo",
            "| Risco | Qtde |",
            "| quebrado | 2 |",
            "| Linguagem | Arquivos | Participação |",
            "## Achados",
            "| Urgência | Algoritmo | Propósito | Tamanho da chave | Local | Substituir por |",
            "| alta | ECDH (NIST curves) | troca de chaves | 256-bit |",
            "### ECDH (NIST curves) em `src/b.ts:7:5`",
            "- **Biblioteca:** node:crypto",
            "- **Risco quântico:** quebrado (Shor)",
        ] {
            assert!(md.contains(expected), "missing `{expected}` in:\n{md}");
        }
    }
}
