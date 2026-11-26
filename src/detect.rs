use ignore::WalkBuilder;
use serde::{Deserialize, Serialize};
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

/// Folders of tests and their fixtures. What is in them runs in the test
/// suite, not in production, so a scan leaves them out unless asked.
const TEST_DIRS: &[&str] = &[
    "test",
    "tests",
    "__tests__",
    "spec",
    "e2e",
    "__mocks__",
    "fixtures",
    "testdata",
];

/// Whether the name of a file says it is a test: `a.test.ts`, `a.spec.js`,
/// `test_a.py`, `a_test.py`, `a_test.go`, `conftest.py`.
pub fn is_test_file(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    name.contains(".test.")
        || name.contains(".spec.")
        || name == "conftest.py"
        || (name.starts_with("test_") && name.ends_with(".py"))
        || name.ends_with("_test.py")
        || name.ends_with("_test.go")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

/// Every file under `root` that the scan looks at: `.gitignore` is respected,
/// dependency and build folders are skipped, and so are the folders of tests
/// unless `include_tests` is set.
fn walk_files(root: &Path, include_tests: bool) -> impl Iterator<Item = PathBuf> {
    WalkBuilder::new(root)
        .require_git(false)
        .filter_entry(move |entry| {
            let is_skipped_dir = entry.depth() > 0
                && entry.file_type().is_some_and(|t| t.is_dir())
                && entry.file_name().to_str().is_some_and(|name| {
                    SKIPPED_DIRS.contains(&name) || (!include_tests && TEST_DIRS.contains(&name))
                });
            !is_skipped_dir
        })
        .build()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_some_and(|t| t.is_file()))
        .map(|entry| entry.into_path())
}

/// Walks `root` and returns every source file whose extension maps to a
/// supported language, leaving out the tests.
pub fn scan_source_files(root: &Path) -> Vec<(PathBuf, Language)> {
    scan_source_files_with(root, false)
}

/// Like [`scan_source_files`], with the tests too when `include_tests` is set.
pub fn scan_source_files_with(root: &Path, include_tests: bool) -> Vec<(PathBuf, Language)> {
    walk_files(root, include_tests)
        .filter(|path| include_tests || !is_test_file(path))
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
    scan_manifests_with(root, false)
}

/// Like [`scan_manifests`], with those inside folders of tests when
/// `include_tests` is set.
pub fn scan_manifests_with(root: &Path, include_tests: bool) -> Vec<PathBuf> {
    walk_files(root, include_tests)
        .filter(|path| crate::modules::javascript::manifest::is_manifest(path))
        .collect()
}

#[cfg(test)]
mod test_filter_tests {
    use super::*;

    fn tree(name: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("agentq-testfilter-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for dir in ["src", "test", "src/__tests__", "tools/e2e"] {
            std::fs::create_dir_all(root.join(dir)).unwrap();
        }
        for file in [
            "src/app.js",
            "src/app.test.js",
            "src/app.spec.ts",
            "src/__tests__/a.js",
            "test/b.js",
            "tools/e2e/c.js",
            "tools/run.py",
            "tools/test_run.py",
            "tools/conftest.py",
        ] {
            std::fs::write(root.join(file), "").unwrap();
        }
        root
    }

    fn names(files: Vec<(PathBuf, Language)>, root: &Path) -> Vec<String> {
        let mut names: Vec<String> = files
            .into_iter()
            .map(|(p, _)| p.strip_prefix(root).unwrap().display().to_string())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn tests_are_left_out_unless_asked_for() {
        let root = tree("left-out");
        assert_eq!(
            names(scan_source_files(&root), &root),
            ["src/app.js", "tools/run.py"]
        );
        assert_eq!(names(scan_source_files_with(&root, true), &root).len(), 9);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_project_inside_a_test_folder_is_still_scanned_when_it_is_the_root() {
        let root = tree("as-root");
        let inside = root.join("test");
        assert_eq!(names(scan_source_files(&inside), &inside), ["b.js"]);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn test_files_are_told_by_their_names() {
        assert!(is_test_file(Path::new("a/b.test.tsx")));
        assert!(is_test_file(Path::new("x_test.go")));
        assert!(!is_test_file(Path::new("src/latest.js")));
        assert!(!is_test_file(Path::new("src/contest.py")));
    }
}
