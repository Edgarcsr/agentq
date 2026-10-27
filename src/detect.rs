use ignore::WalkBuilder;
use std::path::{Path, PathBuf};

/// Directories that hold dependencies or build output and are never scanned.
const SKIPPED_DIRS: &[&str] = &["node_modules", "vendor", "target", "build", "dist", ".git"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Python,
    Java,
    JavaScriptTypeScript,
    Go,
}

impl Language {
    pub fn from_extension(extension: &str) -> Option<Language> {
        match extension {
            "py" => Some(Language::Python),
            "java" => Some(Language::Java),
            "js" | "jsx" | "mjs" | "ts" | "tsx" => Some(Language::JavaScriptTypeScript),
            "go" => Some(Language::Go),
            _ => None,
        }
    }
}

/// Walks `root` and returns every source file whose extension maps to a supported language.
pub fn scan_source_files(root: &Path) -> Vec<(PathBuf, Language)> {
    WalkBuilder::new(root)
        .require_git(false)
        .filter_entry(|entry| {
            let is_skipped_dir = entry.depth() > 0
                && entry.file_type().is_some_and(|t| t.is_dir())
                && entry
                    .file_name()
                    .to_str()
                    .is_some_and(|name| SKIPPED_DIRS.contains(&name));
            !is_skipped_dir
        })
        .build()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_some_and(|t| t.is_file()))
        .filter_map(|entry| {
            let path = entry.into_path();
            let language = path
                .extension()
                .and_then(|ext| ext.to_str())
                .and_then(Language::from_extension)?;
            Some((path, language))
        })
        .collect()
}
