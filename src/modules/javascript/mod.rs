//! Detection of cryptographic API usage in JavaScript and TypeScript sources.

mod bindings;
mod index;
pub mod manifest;
mod rules;
mod syntax;
mod value;

pub use index::JavaScriptAnalyzer;
