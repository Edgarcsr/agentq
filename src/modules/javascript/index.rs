use super::bindings::Bindings;
use super::rules::{self, CallSite};
use super::syntax::{text, walk};
use super::value::eval;
use crate::analyzer::Analyzer;
use crate::detect::Language;
use crate::finding::Finding;
use crate::modules::common::Value;
use crate::parse::parse;
use crate::purpose::{Evidence, infer_purposes};
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
        // Variables that receive the result of the call behind each finding.
        let mut bound: Vec<Vec<String>> = Vec::new();
        let mut evidence: Vec<Evidence> = Vec::new();

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
            let detections = rules::detect(&site);
            let hint = rules::hint(&site);
            if detections.is_empty() && hint.is_none() {
                return;
            }
            let names = argument_names(node, source);
            if let Some(purpose) = hint {
                evidence.push(Evidence {
                    purpose,
                    library: rules::HINT_LIBRARY.to_string(),
                    algorithm: None,
                    names: names.clone(),
                    source: None,
                });
            }
            let receivers = bound_names(node, source);
            for detection in detections {
                if detection.purpose != crate::finding::Purpose::Unknown {
                    evidence.push(Evidence {
                        purpose: detection.purpose,
                        library: detection.library.to_string(),
                        algorithm: Some(detection.algorithm.to_string()),
                        names: names.clone(),
                        source: Some(findings.len()),
                    });
                }
                bound.push(receivers.clone());
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
        infer_purposes(&mut findings, &bound, &evidence);
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

/// Identifiers passed to `call`, and to the calls chained after it, as in
/// `createSign('SHA256').update(data).sign(privateKey)`.
fn argument_names(call: Node, source: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut current = call;
    loop {
        if let Some(arguments) = current.child_by_field_name("arguments") {
            walk(arguments, |node| {
                if matches!(node.kind(), "identifier" | "shorthand_property_identifier") {
                    names.push(text(node, source).to_string());
                }
            });
        }
        // `current.method(...)`: move on to the outer call of the chain.
        let outer = current
            .parent()
            .filter(|member| member.kind() == "member_expression")
            .and_then(|member| member.parent())
            .filter(|outer| outer.kind() == "call_expression");
        match outer {
            Some(outer) => current = outer,
            None => return names,
        }
    }
}

/// Variables that receive the value of `call`: `k` in `const k = call()`,
/// `a`, `b` in `const { a, privateKey: b } = await call()`, the parameters
/// after the error in a Node callback `call(..., (err, pub, priv) => ...)`,
/// and the parameters of a promise's `.then((keys) => ...)`.
fn bound_names(call: Node, source: &str) -> Vec<String> {
    let mut names = callback_parameters(call, source);

    let mut value = call;
    while let Some(parent) = value.parent() {
        match parent.kind() {
            "await_expression"
            | "parenthesized_expression"
            | "as_expression"
            | "satisfies_expression"
            | "non_null_expression" => value = parent,
            _ => break,
        }
    }
    let Some(parent) = value.parent() else {
        return names;
    };
    names.extend(then_parameters(value, source));
    let target = match parent.kind() {
        "variable_declarator" => parent.child_by_field_name("name"),
        "assignment_expression" => parent.child_by_field_name("left"),
        _ => None,
    };
    let Some(target) = target else {
        return names;
    };
    names.extend(pattern_names(target, source));
    names
}

/// Variables declared by a binding pattern. In `{ privateKey: b }` only `b` is
/// a variable; the key is a property name.
fn pattern_names(pattern: Node, source: &str) -> Vec<String> {
    let mut names = Vec::new();
    walk(pattern, |node| {
        if matches!(
            node.kind(),
            "identifier" | "shorthand_property_identifier_pattern"
        ) {
            names.push(text(node, source).to_string());
        }
    });
    names
}

/// Whether `node` is a function written inline, such as an arrow function.
fn is_function(node: Node) -> bool {
    matches!(
        node.kind(),
        "arrow_function" | "function_expression" | "function"
    )
}

/// Parameters of a function node, one pattern per parameter.
fn parameters(function: Node) -> Vec<Node> {
    if let Some(list) = function.child_by_field_name("parameters") {
        let mut cursor = list.walk();
        return list
            .named_children(&mut cursor)
            .filter(|p| p.kind() != "comment")
            .collect();
    }
    // An arrow function with a single bare parameter: `key => ...`.
    function
        .child_by_field_name("parameter")
        .into_iter()
        .collect()
}

/// For a call that takes a Node-style callback, the parameters that follow the
/// error: `publicKey` and `privateKey` in
/// `generateKeyPair('rsa', opts, (err, publicKey, privateKey) => ...)`.
fn callback_parameters(call: Node, source: &str) -> Vec<String> {
    let Some(arguments) = call.child_by_field_name("arguments") else {
        return vec![];
    };
    let mut cursor = arguments.walk();
    let Some(callback) = arguments
        .named_children(&mut cursor)
        .filter(|arg| is_function(*arg))
        .last()
    else {
        return vec![];
    };
    let params = parameters(callback);
    // With a single parameter there is no error to skip.
    let skip = usize::from(params.len() > 1);
    params
        .into_iter()
        .skip(skip)
        .flat_map(|param| pattern_names(param, source))
        .collect()
}

/// For `call.then((result) => ...)`, the names `result` is bound to.
fn then_parameters(value: Node, source: &str) -> Vec<String> {
    let Some(member) = value
        .parent()
        .filter(|member| member.kind() == "member_expression")
    else {
        return vec![];
    };
    if member.child_by_field_name("object") != Some(value)
        || member
            .child_by_field_name("property")
            .is_none_or(|property| text(property, source) != "then")
    {
        return vec![];
    }
    let Some(outer) = member
        .parent()
        .filter(|outer| outer.kind() == "call_expression")
    else {
        return vec![];
    };
    let Some(arguments) = outer.child_by_field_name("arguments") else {
        return vec![];
    };
    let Some(callback) = arguments.named_child(0).filter(|arg| is_function(*arg)) else {
        return vec![];
    };
    parameters(callback)
        .into_iter()
        .take(1)
        .flat_map(|param| pattern_names(param, source))
        .collect()
}

/// First line of the node's text, shortened to [`MAX_SNIPPET`] characters.
fn snippet(node: Node, source: &str) -> String {
    let first_line = text(node, source).lines().next().unwrap_or_default();
    first_line.chars().take(MAX_SNIPPET).collect()
}
