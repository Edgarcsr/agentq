use super::syntax::{string_literal, text, unwrap_expression};
use crate::modules::common::Value;
use std::collections::HashMap;
use tree_sitter::Node;

/// Evaluates literals, object literals and identifiers bound to constants.
pub fn eval(node: Node, source: &str, consts: &HashMap<String, Value>) -> Value {
    let node = unwrap_expression(node);
    if let Some(s) = string_literal(node, source) {
        return Value::Str(s);
    }
    match node.kind() {
        "number" => text(node, source)
            .replace('_', "")
            .parse::<f64>()
            .map(Value::Num)
            .unwrap_or(Value::Unknown),
        "identifier" => consts
            .get(text(node, source))
            .cloned()
            .unwrap_or(Value::Unknown),
        "template_string" => eval_template(node, source, consts),
        "binary_expression" => eval_binary(node, source, consts),
        "member_expression" => {
            let (Some(object), Some(property)) = (
                node.child_by_field_name("object"),
                node.child_by_field_name("property"),
            ) else {
                return Value::Unknown;
            };
            eval(object, source, consts)
                .get(text(property, source))
                .clone()
        }
        "array" => {
            let mut cursor = node.walk();
            let items = node
                .named_children(&mut cursor)
                .filter(|child| child.kind() != "comment")
                .map(|child| eval(child, source, consts))
                .collect();
            Value::Array(items)
        }
        "object" => {
            let mut pairs = Vec::new();
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                match child.kind() {
                    "pair" => {
                        let (Some(key), Some(value)) = (
                            child.child_by_field_name("key"),
                            child.child_by_field_name("value"),
                        ) else {
                            continue;
                        };
                        let name = match string_literal(key, source) {
                            Some(s) => s,
                            None if key.kind() == "property_identifier" => {
                                text(key, source).to_string()
                            }
                            None => continue,
                        };
                        pairs.push((name, eval(value, source, consts)));
                    }
                    "shorthand_property_identifier" => {
                        let name = text(child, source);
                        let value = consts.get(name).cloned().unwrap_or(Value::Unknown);
                        pairs.push((name.to_string(), value));
                    }
                    _ => {}
                }
            }
            Value::Object(pairs)
        }
        _ => Value::Unknown,
    }
}

/// Text of a string or number as JavaScript would concatenate it.
fn display(value: &Value) -> Option<String> {
    match value {
        Value::Str(s) => Some(s.clone()),
        Value::Num(n) if n.fract() == 0.0 && n.abs() < 1e15 => Some(format!("{}", *n as i64)),
        _ => None,
    }
}

/// `` `aes-${BITS}-gcm` `` with every substitution known.
fn eval_template(node: Node, source: &str, consts: &HashMap<String, Value>) -> Value {
    let mut result = String::new();
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        match child.kind() {
            "string_fragment" => result.push_str(text(child, source)),
            "template_substitution" => {
                let Some(inner) = child.named_child(0) else {
                    return Value::Unknown;
                };
                match display(&eval(inner, source, consts)) {
                    Some(part) => result.push_str(&part),
                    None => return Value::Unknown,
                }
            }
            _ => return Value::Unknown,
        }
    }
    Value::Str(result)
}

/// `'aes-' + BITS + '-gcm'` and numeric addition.
fn eval_binary(node: Node, source: &str, consts: &HashMap<String, Value>) -> Value {
    let (Some(left), Some(operator), Some(right)) = (
        node.child_by_field_name("left"),
        node.child_by_field_name("operator"),
        node.child_by_field_name("right"),
    ) else {
        return Value::Unknown;
    };
    if text(operator, source) != "+" {
        return Value::Unknown;
    }
    match (eval(left, source, consts), eval(right, source, consts)) {
        (Value::Num(a), Value::Num(b)) => Value::Num(a + b),
        (a @ (Value::Str(_) | Value::Num(_)), b @ (Value::Str(_) | Value::Num(_))) => {
            match (display(&a), display(&b)) {
                (Some(a), Some(b)) => Value::Str(a + &b),
                _ => Value::Unknown,
            }
        }
        _ => Value::Unknown,
    }
}
