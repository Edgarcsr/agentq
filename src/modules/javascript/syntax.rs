use tree_sitter::Node;

pub use crate::modules::common::syntax::{text, walk};

/// Strips wrappers that do not change what an expression evaluates to:
/// parentheses, `await`, and TypeScript assertions (`as`, `satisfies`, `!`, `<T>x`).
pub fn unwrap_expression(mut node: Node) -> Node {
    while matches!(
        node.kind(),
        "parenthesized_expression"
            | "await_expression"
            | "as_expression"
            | "satisfies_expression"
            | "non_null_expression"
            | "type_assertion"
    ) {
        match node.named_child(if node.kind() == "type_assertion" {
            1
        } else {
            0
        }) {
            Some(inner) => node = inner,
            None => break,
        }
    }
    node
}

/// Contents of a string literal, or of a template literal without substitutions.
pub fn string_literal(node: Node, source: &str) -> Option<String> {
    match node.kind() {
        "string" => {
            let raw = text(node, source);
            Some(raw[1..raw.len().saturating_sub(1).max(1)].to_string())
        }
        "template_string" => {
            let mut cursor = node.walk();
            let has_substitution = node
                .named_children(&mut cursor)
                .any(|child| child.kind() == "template_substitution");
            if has_substitution {
                return None;
            }
            let raw = text(node, source);
            Some(raw[1..raw.len().saturating_sub(1).max(1)].to_string())
        }
        _ => None,
    }
}
