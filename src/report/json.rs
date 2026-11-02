use super::{Report, Summary};
use crate::assess::Assessment;
use serde::Serialize;

#[derive(Serialize)]
struct JsonReport<'a> {
    tool: &'static str,
    version: &'static str,
    knowledge_base: &'a str,
    root: &'a str,
    summary: Summary,
    findings: Vec<&'a Assessment>,
}

pub fn render(report: &Report) -> String {
    let document = JsonReport {
        tool: "agentq",
        version: env!("CARGO_PKG_VERSION"),
        knowledge_base: &report.knowledge_version,
        root: &report.root,
        summary: report.summary(),
        findings: report.sorted_assessments(),
    };
    serde_json::to_string_pretty(&document).expect("report is serializable")
}

#[cfg(test)]
mod tests {
    use super::super::fixtures;
    use super::*;

    #[test]
    fn json_report_has_summary_and_findings_most_urgent_first() {
        let files = fixtures::files();
        let json: serde_json::Value =
            serde_json::from_str(&render(&fixtures::report(&files))).unwrap();
        assert_eq!(json["tool"], "agentq");
        assert_eq!(json["root"], "project");
        assert_eq!(json["summary"]["findings"], 3);
        assert_eq!(json["findings"][0]["finding"]["algorithm"], "ecdh");
        assert_eq!(json["findings"][0]["urgency"], "high");
        assert_eq!(json["findings"][0]["finding"]["purpose"], "key_exchange");
        assert_eq!(json["findings"][0]["recommendations"][0]["id"], "ml-kem");
    }
}
