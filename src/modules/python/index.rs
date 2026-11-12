use super::bindings::Bindings;
use super::rules::{self, Args};
use crate::analyzer::Analyzer;
use crate::detect::Language;
use crate::finding::{Finding, Purpose};
use crate::modules::common::Value;
use crate::modules::common::syntax::{text, walk};
use crate::parse::parse;
use crate::purpose::{Evidence, infer_purposes};
use std::collections::HashMap;
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
        let mut bound = Vec::new();
        let mut evidence = Vec::new();
        let mut variables = VariableState::default();

        walk(tree.root_node(), |node| {
            if node.kind() == "assignment" {
                variables.assign(node, source);
            }
            if node.kind() != "call" {
                return;
            }
            let function = node.child_by_field_name("function");
            let qualified = function.and_then(|function| bindings.resolve(function, source));

            if let Some(hint) = hint_evidence(node, source, &bindings, &variables) {
                evidence.push(hint);
            }
            if let Some((purpose, name)) = method_hint(node, source, &variables) {
                evidence.push(Evidence {
                    purpose,
                    library: "python".to_string(),
                    algorithm: None,
                    names: vec![name],
                    source: None,
                });
            }

            let detections = qualified.map_or_else(Vec::new, |qualified| {
                if is_digest_component(node, source, &bindings) {
                    return Vec::new();
                }
                let (positional, keywords) = evaluate_arguments(node, source, &bindings);
                rules::detect(
                    &qualified,
                    &Args {
                        positional: &positional,
                        keywords: &keywords,
                    },
                )
            });
            for detection in detections {
                let source_index = findings.len();
                if detection.purpose != Purpose::Unknown {
                    evidence.push(Evidence {
                        purpose: detection.purpose,
                        library: "python".to_string(),
                        algorithm: Some(detection.algorithm.to_string()),
                        names: argument_names(node, source, &variables),
                        source: Some(source_index),
                    });
                }
                bound.push(bound_names(node, source, &variables));
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

#[derive(Default)]
struct VariableState {
    versions: HashMap<(usize, String), usize>,
    aliases: HashMap<(usize, String), String>,
}

impl VariableState {
    fn identity(&self, name: &str, scope: usize) -> String {
        let key = (scope, name.to_string());
        self.aliases.get(&key).cloned().unwrap_or_else(|| {
            format!(
                "{scope}:{name}@{}",
                self.versions.get(&key).copied().unwrap_or_default()
            )
        })
    }

    fn assign(&mut self, assignment: Node, source: &str) {
        let scope = scope_id(assignment);
        let Some(left) = assignment.child_by_field_name("left") else {
            return;
        };
        let mut targets = Vec::new();
        assignment_targets(left, source, &mut targets);
        let alias = (targets.len() == 1)
            .then(|| {
                assignment_alias(
                    assignment.child_by_field_name("right")?,
                    source,
                    self,
                    scope,
                )
            })
            .flatten();
        for target in &targets {
            let key = (scope, target.clone());
            *self.versions.entry(key.clone()).or_default() += 1;
            self.aliases.remove(&key);
        }
        if let (Some(target), Some(alias)) = (targets.first(), alias) {
            self.aliases.insert((scope, target.clone()), alias);
        }
    }
}

fn assignment_targets(node: Node, source: &str, targets: &mut Vec<String>) {
    match node.kind() {
        "identifier" => targets.push(text(node, source).to_string()),
        "attribute" | "subscript" => {}
        _ => {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                assignment_targets(child, source, targets);
            }
        }
    }
}

fn assignment_alias(
    right: Node,
    source: &str,
    variables: &VariableState,
    scope: usize,
) -> Option<String> {
    if right.kind() == "identifier" {
        return Some(variables.identity(text(right, source), scope));
    }
    if right.kind() != "call" {
        return None;
    }
    let function = right.child_by_field_name("function")?;
    if function.kind() != "attribute"
        || text(function.child_by_field_name("attribute")?, source) != "public_key"
    {
        return None;
    }
    let receiver = root_identifier(function.child_by_field_name("object")?);
    (receiver.kind() == "identifier").then(|| variables.identity(text(receiver, source), scope))
}

fn argument_names(call: Node, source: &str, variables: &VariableState) -> Vec<String> {
    let Some(arguments) = call.child_by_field_name("arguments") else {
        return vec![];
    };
    let mut names = Vec::new();
    let scope = scope_id(call);
    walk(arguments, |node| {
        if node.kind() == "identifier" {
            names.push(variables.identity(text(node, source), scope));
        }
    });
    names
}

fn bound_names(call: Node, source: &str, variables: &VariableState) -> Vec<String> {
    let Some(parent) = call.parent() else {
        return vec![];
    };
    if parent.kind() != "assignment"
        || parent
            .child_by_field_name("right")
            .is_none_or(|right| right.id() != call.id())
    {
        return vec![];
    }
    let Some(left) = parent.child_by_field_name("left") else {
        return vec![];
    };
    let scope = scope_id(call);
    let mut targets = Vec::new();
    assignment_targets(left, source, &mut targets);
    targets
        .iter()
        .map(|name| variables.identity(name, scope))
        .collect()
}

fn method_hint(call: Node, source: &str, variables: &VariableState) -> Option<(Purpose, String)> {
    let function = call.child_by_field_name("function")?;
    if function.kind() != "attribute" {
        return None;
    }
    let method = function.child_by_field_name("attribute")?;
    let purpose = match text(method, source) {
        "sign" | "verify" => Purpose::Signature,
        "encrypt" | "decrypt" => Purpose::Encryption,
        "exchange" => Purpose::KeyExchange,
        _ => return None,
    };
    let scope = scope_id(call);
    let receiver = receiver_identity(
        function.child_by_field_name("object")?,
        source,
        variables,
        scope,
    )?;
    Some((purpose, receiver))
}

fn hint_evidence(
    call: Node,
    source: &str,
    bindings: &Bindings,
    variables: &VariableState,
) -> Option<Evidence> {
    let qualified = call
        .child_by_field_name("function")
        .and_then(|function| bindings.resolve(function, source))?;
    let purpose = rules::hint(&qualified)?;
    let mut parent = call.parent();
    while let Some(node) = parent {
        if node.kind() == "call"
            && let Some((usage, name)) = method_hint(node, source, variables)
            && usage == purpose
        {
            return Some(Evidence {
                purpose,
                library: "python".to_string(),
                algorithm: None,
                names: vec![name],
                source: None,
            });
        }
        parent = node.parent();
    }
    None
}

fn is_digest_component(call: Node, source: &str, bindings: &Bindings) -> bool {
    let Some(function) = call.child_by_field_name("function") else {
        return false;
    };
    if !bindings
        .resolve(function, source)
        .is_some_and(|name| name.starts_with("cryptography.hazmat.primitives.hashes."))
    {
        return false;
    }

    let mut parent = call.parent();
    while let Some(node) = parent {
        if node.kind() == "call"
            && let Some(outer_function) = node.child_by_field_name("function")
            && let Some(qualified) = bindings.resolve(outer_function, source)
            && matches!(
                qualified.as_str(),
                "cryptography.hazmat.primitives.hmac.HMAC"
                    | "cryptography.hazmat.primitives.kdf.hkdf.HKDF"
                    | "cryptography.hazmat.primitives.kdf.hkdf.HKDFExpand"
            )
        {
            return true;
        }
        parent = node.parent();
    }
    false
}

fn root_identifier(mut node: Node) -> Node {
    while node.kind() == "attribute" {
        let Some(object) = node.child_by_field_name("object") else {
            break;
        };
        node = object;
    }
    node
}

fn receiver_identity(
    node: Node,
    source: &str,
    variables: &VariableState,
    scope: usize,
) -> Option<String> {
    if node.kind() == "identifier" {
        return Some(variables.identity(text(node, source), scope));
    }
    if node.kind() != "call" {
        return None;
    }
    let function = node.child_by_field_name("function")?;
    if function.kind() != "attribute"
        || text(function.child_by_field_name("attribute")?, source) != "public_key"
    {
        return None;
    }
    let receiver = root_identifier(function.child_by_field_name("object")?);
    (receiver.kind() == "identifier").then(|| variables.identity(text(receiver, source), scope))
}

fn scope_id(mut node: Node) -> usize {
    while let Some(parent) = node.parent() {
        if matches!(
            parent.kind(),
            "class_definition" | "function_definition" | "lambda"
        ) {
            return parent.id();
        }
        node = parent;
    }
    0
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
