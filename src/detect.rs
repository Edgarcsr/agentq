use ignore::WalkBuilder;
use std::path::{Path, PathBuf};

/// Directories that hold dependencies or build output and are never scanned.
const SKIPPED_DIRS: &[&str] = &[
    "node_modules",
    "vendor",
    "target",
    "build",
    "dist",
    ".git",
    ".next",
    ".nuxt",
    "coverage",
    "bower_components",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
            "js" | "jsx" | "mjs" | "cjs" | "ts" | "tsx" | "mts" | "cts" => {
                Some(Language::JavaScriptTypeScript)
            }
            "go" => Some(Language::Go),
            _ => None,
        }
    }

    /// Short human-readable name used in the CLI output.
    pub fn label(self) -> &'static str {
        match self {
            Language::Python => "Python",
            Language::Java => "Java",
            Language::JavaScriptTypeScript => "JS/TS",
            Language::Go => "Go",
        }
    }
}

/// Every file under `root` that the scan looks at: `.gitignore` is respected
/// and dependency and build folders are skipped.
fn walk_files(root: &Path) -> impl Iterator<Item = PathBuf> {
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
        .map(|entry| entry.into_path())
}

/// Walks `root` and returns every source file whose extension maps to a supported language.
pub fn scan_source_files(root: &Path) -> Vec<(PathBuf, Language)> {
    walk_files(root)
        .filter_map(|path| {
            let language = path
                .extension()
                .and_then(|ext| ext.to_str())
                .and_then(Language::from_extension)?;
            Some((path, language))
        })
        .collect()
}

/// Walks `root` and returns the dependency manifests and lockfiles of the
/// projects in it (`package.json`, `package-lock.json`, `yarn.lock`, ...).
pub fn scan_manifests(root: &Path) -> Vec<PathBuf> {
    walk_files(root)
        .filter(|path| crate::modules::javascript::manifest::is_manifest(path))
        .collect()
}
