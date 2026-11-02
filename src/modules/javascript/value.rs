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
