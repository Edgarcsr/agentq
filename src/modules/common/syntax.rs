//! Tree-sitter helpers used by every language analyzer.

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
