//! Rules that recognize cryptographic calls once the callee has been resolved
//! to a module and property path.

mod names;
mod node_crypto;
mod webcrypto;

use super::bindings::Resolved;
use super::value::Value;
use crate::finding::{Confidence, Purpose};

/// A call expression whose callee resolved to a known module.
pub struct CallSite<'a> {
    pub resolved: &'a Resolved,
    pub args: &'a [Value],
}

impl CallSite<'_> {
    pub fn path(&self) -> &[String] {
        &self.resolved.path
    }

    /// The `index`-th argument, or an unknown value when absent.
    pub fn arg(&self, index: usize) -> &Value {
        static UNKNOWN: Value = Value::Unknown;
        self.args.get(index).unwrap_or(&UNKNOWN)
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

pub fn detect(site: &CallSite) -> Vec<Detection> {
    match site.resolved.module.as_str() {
        "crypto" => node_crypto::detect(site),
        "webcrypto" => webcrypto::detect(site),
        _ => vec![],
    }
}
