use crate::finding::Finding;
use std::path::Path;

/// Finds cryptographic API usage in the source of one file.
pub trait Analyzer {
    fn analyze(&self, path: &Path, source: &str) -> Vec<Finding>;
}
