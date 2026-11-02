use super::syntax::{string_literal, text, unwrap_expression};
use std::collections::HashMap;
use tree_sitter::Node;

/// A compile-time value read from an expression.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Str(String),
    Num(f64),
    Object(Vec<(String, Value)>),
    Array(Vec<Value>),
    /// Anything that cannot be determined statically.
    Unknown,
}

impl Value {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_u32(&self) -> Option<u32> {
        match self {
            Value::Num(n) if *n >= 0.0 && n.fract() == 0.0 && *n <= u32::MAX as f64 => {
                Some(*n as u32)
            }
            _ => None,
        }
    }

    /// Elements of an array value.
    pub fn as_array(&self) -> &[Value] {
        match self {
            Value::Array(items) => items,
            _ => &[],
        }
    }

    /// Property of an object value.
    pub fn get(&self, key: &str) -> &Value {
        static UNKNOWN: Value = Value::Unknown;
        match self {
            Value::Object(pairs) => pairs
                .iter()
                .find(|(name, _)| name == key)
                .map(|(_, value)| value)
                .unwrap_or(&UNKNOWN),
            _ => &UNKNOWN,
        }
    }

    pub fn is_unknown(&self) -> bool {
        matches!(self, Value::Unknown)
    }
}

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
