use tree_sitter::Node;

/// Source text covered by `node`.
pub fn text<'s>(node: Node, source: &'s str) -> &'s str {
    &source[node.byte_range()]
}

/// Calls `visit` on every node of the subtree in source order.
///
/// Iterative so that deeply nested code (minified bundles) cannot overflow the stack.
pub fn walk<'t>(root: Node<'t>, mut visit: impl FnMut(Node<'t>)) {
    let mut cursor = root.walk();
    loop {
        visit(cursor.node());
        if cursor.goto_first_child() {
            continue;
        }
        loop {
            if cursor.goto_next_sibling() {
                break;
            }
            if !cursor.goto_parent() {
                return;
            }
        }
    }
}

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
