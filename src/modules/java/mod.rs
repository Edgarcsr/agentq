//! File-local analysis of Java's JCA/JCE cryptographic APIs.

mod bindings;
mod index;
mod keys;
mod rules;
mod value;

pub use index::JavaAnalyzer;
