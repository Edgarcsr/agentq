use super::bindings::Bindings;
use super::rules::{self, Args};
use crate::analyzer::Analyzer;
use crate::detect::Language;
use crate::finding::Finding;
use crate::modules::common::Value;
use crate::modules::common::syntax::{text, walk};
use crate::parse::parse;
use crate::purpose::infer_purposes;
use std::path::Path;
use tree_sitter::Node;

/// Longest snippet kept in a finding, in characters.
const MAX_SNIPPET: usize = 120;

/// Finds cryptographic API usage in Python files.
pub struct PythonAnalyzer;

impl Analyzer for PythonAnalyzer {
    fn analyze(&self, path: &Path, source: &str) -> Vec<Finding> {
        let Some(tree) = parse(Language::Python, path, source) else {
            return vec![];
        };
        let bindings = Bindings::collect(&tree, source);
        let mut findings = Vec::new();
        let mut hints = Vec::new();

        walk(tree.root_node(), |node| {
            if node.kind() != "call" {
                return;
            }
            let Some(qualified) = node
                .child_by_field_name("function")
                .and_then(|function| bindings.resolve(function, source))
            else {
                return;
            };
            hints.extend(rules::hint(&qualified));

            let (positional, keywords) = evaluate_arguments(node, source, &bindings);
            let args = Args {
                positional: &positional,
                keywords: &keywords,
            };
            for detection in rules::detect(&qualified, &args) {
                let position = node.start_position();
                findings.push(Finding {
                    file: path.to_path_buf(),
                    line: position.row + 1,
                    column: position.column + 1,
                    snippet: snippet(node, source),
                    library: detection.library.to_string(),
                    algorithm: detection.algorithm.to_string(),
                    purpose: detection.purpose,
                    key_size: detection.key_size,
                    confidence: detection.confidence,
                });
            }
        });
        infer_purposes(&mut findings, &hints);
        findings
    }
}

fn evaluate_arguments(
    call: Node,
    source: &str,
    bindings: &Bindings,
) -> (Vec<Value>, Vec<(String, Value)>) {
    let mut positional = Vec::new();
    let mut keywords = Vec::new();
    let Some(arguments) = call.child_by_field_name("arguments") else {
        return (positional, keywords);
    };
    let mut cursor = arguments.walk();
    for arg in arguments.named_children(&mut cursor) {
        match arg.kind() {
            "comment" => {}
            "keyword_argument" => {
                if let (Some(name), Some(value)) = (
                    arg.child_by_field_name("name"),
                    arg.child_by_field_name("value"),
                ) {
                    keywords.push((text(name, source).to_string(), bindings.eval(value, source)));
                }
            }
            _ => positional.push(bindings.eval(arg, source)),
        }
    }
    (positional, keywords)
}

/// First line of the node's text, shortened to [`MAX_SNIPPET`] characters.
fn snippet(node: Node, source: &str) -> String {
    let first_line = text(node, source).lines().next().unwrap_or_default();
    first_line.chars().take(MAX_SNIPPET).collect()
}
