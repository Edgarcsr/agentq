use super::{Report, implementations, key_size_label, recommendation_names, urgency_name};
use std::fmt::Write;

/// Escapes text for use inside a table cell.
fn cell(text: &str) -> String {
    text.replace('|', "\\|").replace('`', "'")
}

pub fn render(report: &Report) -> String {
    let summary = report.summary();
    let mut out = String::new();

    writeln!(out, "# agentq report\n").unwrap();
    writeln!(out, "- **Analyzed:** `{}`", report.root).unwrap();
    writeln!(out, "- **Files detected:** {}", summary.files).unwrap();
    writeln!(out, "- **Cryptographic uses found:** {}", summary.findings).unwrap();
    writeln!(out, "- **Knowledge base:** {}\n", report.knowledge_version).unwrap();

    writeln!(out, "## Summary\n").unwrap();
    writeln!(out, "| Risk | Count |\n| --- | ---: |").unwrap();
    for (risk, count) in &summary.by_risk {
        writeln!(out, "| {risk} | {count} |").unwrap();
    }
    writeln!(out, "\n| Language | Files | Share |\n| --- | ---: | ---: |").unwrap();
    for (language, share) in &summary.languages {
        writeln!(
            out,
            "| {language} | {} | {:.1}% |",
            share.files, share.percent
        )
        .unwrap();
    }

    if report.assessments.is_empty() {
        return out;
    }

    writeln!(out, "\n## Findings\n").unwrap();
    writeln!(
        out,
        "| Urgency | Algorithm | Purpose | Key size | Location | Replace with |\n\
         | --- | --- | --- | --- | --- | --- |"
    )
    .unwrap();
    let sorted = report.sorted_assessments();
    for assessment in &sorted {
        let finding = &assessment.finding;
        writeln!(
            out,
            "| {} | {} | {} | {} | `{}:{}` | {} |",
            urgency_name(assessment.urgency),
            cell(&assessment.algorithm_name),
            finding.purpose.label(),
            key_size_label(finding.key_size),
            finding.file.display(),
            finding.line,
            cell(&recommendation_names(assessment).join(", "))
        )
        .unwrap();
    }

    writeln!(out, "\n## Details\n").unwrap();
    for assessment in sorted {
        let finding = &assessment.finding;
        writeln!(
            out,
            "### {} at `{}:{}:{}`\n",
            assessment.algorithm_name,
            finding.file.display(),
            finding.line,
            finding.column
        )
        .unwrap();
        writeln!(out, "```\n{}\n```\n", finding.snippet).unwrap();
        writeln!(
            out,
            "- **Library:** {}\n- **Confidence:** {:?}\n- **Quantum risk:** {} ({:?})",
            finding.library,
            finding.confidence,
            super::risk_name(assessment.risk),
            assessment.attack
        )
        .unwrap();
        writeln!(out, "- **Why:** {}", assessment.explanation).unwrap();
        let names = recommendation_names(assessment);
        if !names.is_empty() {
            writeln!(out, "- **Replace with:** {}", names.join(", ")).unwrap();
            let implementations = implementations(assessment);
            if !implementations.is_empty() {
                writeln!(out, "- **Implementations:** {}", implementations.join(", ")).unwrap();
            }
        }
        for advice in &assessment.advice {
            writeln!(out, "- **Note:** {advice}").unwrap();
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
}
