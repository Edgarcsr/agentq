//! Detection of cryptographic API usage in JavaScript and TypeScript sources.

mod bindings;
mod index;
mod syntax;
mod value;

pub use index::JavaScriptAnalyzer;
