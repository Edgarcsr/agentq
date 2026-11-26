//! Small, bounded evaluator for algorithm names and key-size constants.

use super::bindings::Bindings;
use crate::modules::common::{Value, syntax::text};
use tree_sitter::Node;

pub fn arguments(call: Node) -> Vec<Node> {
    let Some(args) = call.child_by_field_name("arguments") else {
        return vec![];
    };
    let mut cursor = args.walk();
    args.named_children(&mut cursor)
        .filter(|arg| !matches!(arg.kind(), "line_comment" | "block_comment"))
        .collect()
}

pub fn eval<'t>(node: Node<'t>, source: &str, bindings: &Bindings<'t>) -> Value {
    evaluate(node, source, bindings, 0, &mut 256)
}

fn evaluate<'t>(
    node: Node<'t>,
    source: &str,
    bindings: &Bindings<'t>,
    depth: u8,
    budget: &mut u16,
) -> Value {
    if depth >= 32 || *budget == 0 {
        return Value::Unknown;
    }
    *budget -= 1;
    let Some(node) = bindings.origin(node, source) else {
        return Value::Unknown;
    };
    let mut recurse = |node| evaluate(node, source, bindings, depth + 1, budget);
    match node.kind() {
        "string_literal" => string(text(node, source)).map_or(Value::Unknown, Value::Str),
        "decimal_integer_literal"
        | "hex_integer_literal"
        | "binary_integer_literal"
        | "octal_integer_literal" => {
            let raw = text(node, source).replace('_', "");
            let raw = raw.trim_end_matches(['l', 'L']);
            let (digits, base) = if let Some(hex) =
                raw.strip_prefix("0x").or_else(|| raw.strip_prefix("0X"))
            {
                (hex, 16)
            } else if let Some(binary) = raw.strip_prefix("0b").or_else(|| raw.strip_prefix("0B")) {
                (binary, 2)
            } else if node.kind() == "octal_integer_literal" {
                (raw, 8)
            } else {
                (raw, 10)
            };
            u32::from_str_radix(digits, base).map_or(Value::Unknown, |n| Value::Num(n as f64))
        }
        "binary_expression" => {
            let (Some(left), Some(right), Some(operator)) = (
                node.child_by_field_name("left"),
                node.child_by_field_name("right"),
                node.child_by_field_name("operator"),
            ) else {
                return Value::Unknown;
            };
            match (recurse(left), recurse(right), text(operator, source)) {
                (Value::Str(a), Value::Str(b), "+") if a.len() + b.len() <= 4096 => {
                    Value::Str(a + &b)
                }
                (Value::Str(a), Value::Num(b), "+") => Value::Str(format!("{a}{b}")),
                (Value::Num(a), Value::Str(b), "+") => Value::Str(format!("{a}{b}")),
                (Value::Num(a), Value::Num(b), "+") => Value::Num(a + b),
                (Value::Num(a), Value::Num(b), "*") => Value::Num(a * b),
                _ => Value::Unknown,
            }
        }
        _ => Value::Unknown,
    }
}

/// Java escapes differ from JSON (notably octal escapes and `\s`). Text blocks
/// and Java's source-wide Unicode preprocessing are intentionally not modeled.
fn string(raw: &str) -> Option<String> {
    if raw.starts_with("\"\"\"") {
        return None;
    }
    let mut chars = raw.strip_prefix('"')?.strip_suffix('"')?.chars().peekable();
    let mut out = String::new();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        out.push(match chars.next()? {
            'b' => '\u{0008}',
            't' => '\t',
            'n' => '\n',
            'f' => '\u{000c}',
            'r' => '\r',
            's' => ' ',
            '"' => '"',
            '\'' => '\'',
            '\\' => '\\',
            first @ '0'..='7' => {
                let mut n = first.to_digit(8)?;
                for _ in 0..if first <= '3' { 2 } else { 1 } {
                    if let Some(next @ '0'..='7') = chars.peek().copied() {
                        chars.next();
                        n = n * 8 + next.to_digit(8)?;
                    } else {
                        break;
                    }
                }
                char::from_u32(n)?
            }
            _ => return None,
        });
    }
    Some(out)
}

/// Explicit numeric key sizes or supported named-parameter specifications.
pub fn parameter<'t>(node: Node<'t>, source: &str, bindings: &Bindings<'t>) -> Value {
    if node.kind() == "field_access"
        && let (Some(object), Some(field)) = (
            node.child_by_field_name("object"),
            node.child_by_field_name("field"),
        )
        && bindings.class(object, source) == Some("java.security.spec.NamedParameterSpec")
    {
        return Value::Str(text(field, source).replace('_', "-"));
    }
    let Some(origin) = bindings.origin(node, source) else {
        return Value::Unknown;
    };
    if origin.id() != node.id() {
        return parameter(origin, source, bindings);
    }
    if node.kind() == "object_creation_expression"
        && let Some(class) = node
            .child_by_field_name("type")
            .and_then(|t| bindings.class(t, source))
        && matches!(
            class,
            "java.security.spec.ECGenParameterSpec"
                | "java.security.spec.NamedParameterSpec"
                | "java.security.spec.RSAKeyGenParameterSpec"
        )
        && let Some(arg) = arguments(node).first()
    {
        return eval(*arg, source, bindings);
    }
    eval(node, source, bindings)
}
