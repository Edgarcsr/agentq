//! SARIF 2.1.0, the format GitHub code scanning and most CI systems read.
//!
//! Only what needs action is reported (urgency low or higher); uses that are
//! already safe are an inventory matter, not an alert. Uses of an algorithm
//! missing from the knowledge base are listed as notes for manual review.

use super::{Report, implementations, key_size_label, recommendation_names, urgency_name};
use crate::assess::{Assessment, Urgency};
use crate::finding::Finding;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::Path;

const SCHEMA: &str = "https://json.schemastore.org/sarif-2.1.0.json";
const UNRECOGNIZED_RULE: &str = "agentq/unrecognized";

/// SARIF level of an urgency.
fn level(urgency: Urgency) -> &'static str {
    match urgency {
        Urgency::High => "error",
        Urgency::Medium => "warning",
        Urgency::Low | Urgency::None => "note",
    }
}

/// GitHub's `security-severity`, a CVSS-like score from 0 to 10.
fn severity(urgency: Urgency) -> &'static str {
    match urgency {
        Urgency::High => "8.0",
        Urgency::Medium => "5.0",
        Urgency::Low => "3.0",
        Urgency::None => "0.0",
    }
}

/// The path of a file as SARIF wants it: relative to the analyzed folder, with
/// forward slashes.
fn uri(root: &str, file: &Path) -> String {
    let relative = file.strip_prefix(root).unwrap_or(file);
    relative.to_string_lossy().replace('\\', "/")
}

/// A fingerprint that survives the code moving up or down, so that code
/// scanning does not report the same finding as new after an unrelated edit.
fn fingerprint(root: &str, finding: &Finding) -> String {
    // FNV-1a: stable across runs and platforms, unlike the std hasher.
    let mut hash: u64 = 0xcbf29ce484222325;
    let parts = [
        uri(root, &finding.file),
        finding.algorithm.clone(),
        format!("{:?}", finding.purpose),
        finding.snippet.trim().to_string(),
    ];
    for part in parts {
        for byte in part.bytes().chain(std::iter::once(0)) {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
    }
    format!("{hash:016x}")
}

fn location(root: &str, finding: &Finding) -> Value {
    json!({
        "physicalLocation": {
            "artifactLocation": { "uri": uri(root, &finding.file), "uriBaseId": "%SRCROOT%" },
            "region": {
                "startLine": finding.line,
                "startColumn": finding.column,
                "snippet": { "text": finding.snippet },
            },
        }
    })
}

fn message(report: &Report, assessment: &Assessment) -> String {
    let finding = &assessment.finding;
    let size = key_size_label(finding.key_size);
    let mut text = format!(
        "{} {} ({}). {}",
        assessment.algorithm_name,
        size,
        report.lang.purpose(finding.purpose),
        assessment.explanation
    )
    .replace("  ", " ");
    let alternatives = recommendation_names(assessment);
    if !alternatives.is_empty() {
        text.push_str(&format!(" Replace with: {}.", alternatives.join(", ")));
    }
    text
}

pub fn render(report: &Report) -> String {
    let root = report.root.as_str();
    let actionable: Vec<&Assessment> = report
        .sorted_assessments()
        .into_iter()
        .filter(|a| a.urgency != Urgency::None)
        .collect();

    // One rule per algorithm, as severe as its worst result.
    let mut rules: BTreeMap<String, (&Assessment, Urgency)> = BTreeMap::new();
    for assessment in &actionable {
        let entry = rules
            .entry(format!("agentq/{}", assessment.finding.algorithm))
            .or_insert((assessment, assessment.urgency));
        entry.1 = entry.1.max(assessment.urgency);
    }
    let unrecognized = !report.unassessed.is_empty();
    let mut rule_ids: Vec<String> = rules.keys().cloned().collect();
    if unrecognized {
        rule_ids.push(UNRECOGNIZED_RULE.to_string());
    }
    let index_of = |id: &str| rule_ids.iter().position(|r| r == id).unwrap_or(0);

    let mut rule_objects: Vec<Value> = rules
        .iter()
        .map(|(id, (assessment, urgency))| {
            let help = format!(
                "Replace with: {}. Implementations: {}.",
                recommendation_names(assessment).join(", "),
                implementations(assessment).join(", ")
            );
            json!({
                "id": id,
                "name": assessment.algorithm_name,
                "shortDescription": { "text": format!("{} is exposed to quantum attacks", assessment.algorithm_name) },
                "fullDescription": { "text": assessment.explanation },
                "help": { "text": help },
                "defaultConfiguration": { "level": level(*urgency) },
                "properties": {
                    "tags": ["cryptography", "post-quantum", "security"],
                    "security-severity": severity(*urgency),
                },
            })
        })
        .collect();
    if unrecognized {
        rule_objects.push(json!({
            "id": UNRECOGNIZED_RULE,
            "name": "Cryptography to review by hand",
            "shortDescription": { "text": "Cryptography whose algorithm agentq could not identify" },
            "fullDescription": { "text": "The code or a dependency uses cryptography, but the algorithm is chosen at runtime or is not in the knowledge base. Check which one is used." },
            "defaultConfiguration": { "level": "note" },
            "properties": { "tags": ["cryptography", "post-quantum"], "security-severity": "0.0" },
        }));
    }

    let mut results: Vec<Value> = actionable
        .iter()
        .map(|assessment| {
            let finding = &assessment.finding;
            let id = format!("agentq/{}", finding.algorithm);
            json!({
                "ruleId": id,
                "ruleIndex": index_of(&id),
                "level": level(assessment.urgency),
                "message": { "text": message(report, assessment) },
                "locations": [location(root, finding)],
                "partialFingerprints": { "agentq/v1": fingerprint(root, finding) },
                "properties": {
                    "algorithm": finding.algorithm,
                    "purpose": format!("{:?}", finding.purpose),
                    "urgency": urgency_name(assessment.urgency),
                    "confidence": format!("{:?}", finding.confidence),
                    "library": finding.library,
                    "keySize": finding.key_size,
                },
            })
        })
        .collect();
    for finding in report.unassessed {
        results.push(json!({
            "ruleId": UNRECOGNIZED_RULE,
            "ruleIndex": index_of(UNRECOGNIZED_RULE),
            "level": "note",
            "message": { "text": format!("Uses cryptography with an algorithm agentq could not identify ({}). Check which algorithm is used.", finding.library) },
            "locations": [location(root, finding)],
            "partialFingerprints": { "agentq/v1": fingerprint(root, finding) },
        }));
    }

    let mut run = json!({
        "tool": {
            "driver": {
                "name": "agentq",
                "version": env!("CARGO_PKG_VERSION"),
                "informationUri": "https://github.com/Edgarcsr/agentq",
                "rules": rule_objects,
            }
        },
        "results": results,
        "properties": { "knowledgeBase": report.knowledge_version },
    });
    if let Ok(absolute) = std::fs::canonicalize(root) {
        let mut base = format!("file://{}", absolute.display());
        if !base.ends_with('/') {
            base.push('/');
        }
        run["originalUriBaseIds"] = json!({ "%SRCROOT%": { "uri": base } });
    }

    let document = json!({ "$schema": SCHEMA, "version": "2.1.0", "runs": [run] });
    serde_json::to_string_pretty(&document).expect("SARIF is serializable")
}

#[cfg(test)]
mod tests {
    use super::super::fixtures;
    use super::*;

    fn sarif() -> Value {
        let files = fixtures::files();
        serde_json::from_str(&render(&fixtures::report(&files))).unwrap()
    }

    #[test]
    fn it_is_a_sarif_2_1_0_document_with_one_run() {
        let doc = sarif();
        assert_eq!(doc["version"], "2.1.0");
        assert!(doc["$schema"].as_str().unwrap().contains("sarif-2.1.0"));
        assert_eq!(doc["runs"].as_array().unwrap().len(), 1);
        assert_eq!(doc["runs"][0]["tool"]["driver"]["name"], "agentq");
    }

    #[test]
    fn results_point_at_a_relative_file_line_and_column() {
        let doc = sarif();
        let first = &doc["runs"][0]["results"][0];
        let physical = &first["locations"][0]["physicalLocation"];
        assert_eq!(physical["artifactLocation"]["uriBaseId"], "%SRCROOT%");
        assert!(physical["region"]["startLine"].as_u64().unwrap() > 0);
        assert!(physical["region"]["startColumn"].as_u64().unwrap() > 0);
        assert!(first["partialFingerprints"]["agentq/v1"].is_string());
    }

    #[test]
    fn levels_follow_urgency_and_safe_uses_are_left_out() {
        let doc = sarif();
        let results = doc["runs"][0]["results"].as_array().unwrap();
        for result in results {
            assert_ne!(result["properties"]["urgency"], "none");
            let expected = match result["properties"]["urgency"].as_str().unwrap() {
                "high" => "error",
                "medium" => "warning",
                _ => "note",
            };
            assert_eq!(result["level"], expected);
        }
        // Most urgent first.
        assert_eq!(results[0]["level"], "error");
    }

    #[test]
    fn every_result_names_an_existing_rule() {
        let doc = sarif();
        let rules = doc["runs"][0]["tool"]["driver"]["rules"]
            .as_array()
            .unwrap();
        for result in doc["runs"][0]["results"].as_array().unwrap() {
            let index = result["ruleIndex"].as_u64().unwrap() as usize;
            assert_eq!(rules[index]["id"], result["ruleId"]);
        }
        assert!(rules[0]["properties"]["security-severity"].is_string());
    }

    #[test]
    fn the_fingerprint_does_not_depend_on_the_line() {
        let mut a = fixtures::assessments()[0].finding.clone();
        let mut b = a.clone();
        a.line = 3;
        b.line = 90;
        assert_eq!(fingerprint("project", &a), fingerprint("project", &b));
        b.algorithm = "rsa".into();
        assert_ne!(fingerprint("project", &a), fingerprint("project", &b));
    }
}
