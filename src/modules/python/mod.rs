//! Detection of cryptographic API usage in Python sources.

mod bindings;
mod index;
mod rules;

pub use index::PythonAnalyzer;
