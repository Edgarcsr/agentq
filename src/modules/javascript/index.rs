use super::bindings::Bindings;
use super::rules::{self, CallSite};
use super::syntax::{text, walk};
use super::value::eval;
use crate::analyzer::Analyzer;
use crate::detect::Language;
use crate::finding::Finding;
use crate::modules::common::Value;
use crate::parse::parse;
use crate::purpose::infer_purposes;
use std::path::Path;
use tree_sitter::Node;

/// Longest snippet kept in a finding, in characters.
const MAX_SNIPPET: usize = 120;

/// Finds cryptographic API usage in JavaScript and TypeScript files.
pub struct JavaScriptAnalyzer;

impl Analyzer for JavaScriptAnalyzer {
    fn analyze(&self, path: &Path, source: &str) -> Vec<Finding> {
        let Some(tree) = parse(Language::JavaScriptTypeScript, path, source) else {
            return vec![];
        };
        let bindings = Bindings::collect(&tree, source);
        let mut findings = Vec::new();
        let mut hints = Vec::new();

        walk(tree.root_node(), |node| {
            let (callee, is_new) = match node.kind() {
                "call_expression" => (node.child_by_field_name("function"), false),
                "new_expression" => (node.child_by_field_name("constructor"), true),
                _ => return,
            };
            let Some(resolved) = callee.and_then(|callee| bindings.resolve(callee, source)) else {
                return;
            };
            let args = evaluate_arguments(node, source, &bindings);
            let site = CallSite {
                resolved: &resolved,
                args: &args,
                is_new,
            };
            hints.extend(rules::hint(&site));
            for detection in rules::detect(&site) {
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

fn evaluate_arguments(call: Node, source: &str, bindings: &Bindings) -> Vec<Value> {
    let Some(arguments) = call.child_by_field_name("arguments") else {
        return vec![];
    };
    let mut cursor = arguments.walk();
    arguments
        .named_children(&mut cursor)
        .filter(|arg| arg.kind() != "comment")
        .map(|arg| eval(arg, source, bindings.consts()))
        .collect()
}

/// First line of the node's text, shortened to [`MAX_SNIPPET`] characters.
fn snippet(node: Node, source: &str) -> String {
    let first_line = text(node, source).lines().next().unwrap_or_default();
    first_line.chars().take(MAX_SNIPPET).collect()
}
