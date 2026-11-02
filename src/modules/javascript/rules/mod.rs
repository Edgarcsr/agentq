//! Rules that recognize cryptographic calls once the callee has been resolved
//! to a module and property path.

mod crypto_js;
mod jwt;
mod node_crypto;
mod public_key;
mod webcrypto;

use super::bindings::Resolved;
use crate::modules::common::{Detection, Value};

/// A call or `new` expression whose callee resolved to a known module.
pub struct CallSite<'a> {
    pub resolved: &'a Resolved,
    pub args: &'a [Value],
    /// Whether this is a `new X(...)` expression.
    pub is_new: bool,
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

pub fn detect(site: &CallSite) -> Vec<Detection> {
    match site.resolved.module.as_str() {
        "crypto" => node_crypto::detect(site),
        "webcrypto" => webcrypto::detect(site),
        "jsonwebtoken" | "jose" => jwt::detect(site),
        "node-forge" | "node-rsa" | "elliptic" | "tweetnacl" => public_key::detect(site),
        module if module.starts_with("@noble/") => public_key::detect(site),
        module if module == "crypto-js" || module.starts_with("crypto-js/") => {
            crypto_js::detect(site)
        }
        _ => vec![],
    }
}

/// Purpose implied by a call that creates no finding by itself.
pub fn hint(site: &CallSite) -> Option<crate::finding::Purpose> {
    match site.resolved.module.as_str() {
        "crypto" => node_crypto::hint(site),
        _ => None,
    }
}
