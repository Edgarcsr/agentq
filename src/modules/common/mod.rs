//! Types and helpers shared by the per-language analyzers.

pub mod names;

use crate::finding::{Confidence, Purpose};

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

/// What a rule recognized at a call site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Detection {
    /// Knowledge base id.
    pub algorithm: &'static str,
    pub purpose: Purpose,
    pub key_size: Option<u32>,
    pub confidence: Confidence,
    pub library: &'static str,
}
