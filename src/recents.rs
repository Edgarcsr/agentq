//! Projects the user opened before, kept in `~/.agentq/recent.toml`.
//!
//! Each entry remembers how the last scan went, so the desktop interface can
//! show a coloured dot next to a project before it is scanned again. A missing
//! or broken file is the same as no history.

use crate::analysis::Analysis;
use crate::assess::Urgency;
use crate::config::Settings;
use crate::finding::QuantumRisk;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// How many projects the history keeps.
pub const MAX_RECENTS: usize = 20;

/// The outcome of a scan, in numbers.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Summary {
    pub files: usize,
    pub broken: usize,
    pub weakened: usize,
    pub safe: usize,
    /// Findings that should be migrated first (high urgency).
    pub urgent: usize,
    pub medium: usize,
    pub low: usize,
    /// Already safe, nothing to migrate.
    pub none: usize,
}

/// What the dot next to a project says about its last scan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Health {
    /// Never scanned.
    Unknown,
    /// Scanned, and nothing is broken by quantum attacks.
    Clean,
    /// Scanned; some cryptography is weakened but nothing is broken.
    Warning,
    /// Scanned; some cryptography is broken by quantum attacks.
    AtRisk,
}

impl Summary {
    pub fn from_analysis(analysis: &Analysis) -> Summary {
        let mut summary = Summary {
            files: analysis.files.len(),
            ..Summary::default()
        };
        for assessment in &analysis.assessments {
            match assessment.risk {
                QuantumRisk::Broken => summary.broken += 1,
                QuantumRisk::Weakened => summary.weakened += 1,
                QuantumRisk::Safe => summary.safe += 1,
            }
            match assessment.urgency {
                Urgency::High => summary.urgent += 1,
                Urgency::Medium => summary.medium += 1,
                Urgency::Low => summary.low += 1,
                Urgency::None => summary.none += 1,
            }
        }
        summary
    }

    pub fn total(&self) -> usize {
        self.broken + self.weakened + self.safe
    }

    pub fn health(&self) -> Health {
        if self.broken > 0 {
            Health::AtRisk
        } else if self.weakened > 0 {
            Health::Warning
        } else {
            Health::Clean
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recent {
    pub path: PathBuf,
    /// Seconds since the Unix epoch: when it was added or last analyzed.
    pub last_opened: u64,
    pub summary: Option<Summary>,
}

impl Recent {
    /// The folder's own name, which is what the list shows in bold.
    pub fn name(&self) -> String {
        project_name(&self.path)
    }

    pub fn health(&self) -> Health {
        self.summary.map_or(Health::Unknown, |s| s.health())
    }
}

/// What kind of project a folder looks like, from the files at its root.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Node,
    TypeScript,
    Python,
    Go,
    Rust,
    Java,
    Ruby,
    Php,
    Folder,
}

impl Kind {
    /// Looks at the root of `path` for the files that give a project away.
    pub fn detect(path: &Path) -> Kind {
        let has = |name: &str| path.join(name).exists();
        if has("package.json") {
            if has("tsconfig.json") {
                Kind::TypeScript
            } else {
                Kind::Node
            }
        } else if has("go.mod") {
            Kind::Go
        } else if has("Cargo.toml") {
            Kind::Rust
        } else if has("pyproject.toml") || has("requirements.txt") || has("setup.py") {
            Kind::Python
        } else if has("pom.xml") || has("build.gradle") || has("build.gradle.kts") {
            Kind::Java
        } else if has("Gemfile") {
            Kind::Ruby
        } else if has("composer.json") {
            Kind::Php
        } else {
            Kind::Folder
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Kind::Node => "Node.js",
            Kind::TypeScript => "TypeScript",
            Kind::Python => "Python",
            Kind::Go => "Go",
            Kind::Rust => "Rust",
            Kind::Java => "Java",
            Kind::Ruby => "Ruby",
            Kind::Php => "PHP",
            Kind::Folder => "Folder",
        }
    }
}

/// The last component of `path`, or the whole path for a root such as `/`.
pub fn project_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

/// The history, most recently opened first.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Recents {
    pub projects: Vec<Recent>,
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

impl Recents {
    pub fn path() -> Option<PathBuf> {
        Settings::dir().map(|dir| dir.join("recent.toml"))
    }

    pub fn load() -> Recents {
        Recents::path()
            .and_then(|path| std::fs::read_to_string(path).ok())
            .and_then(|text| toml::from_str(&text).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> std::io::Result<()> {
        let path = Recents::path().ok_or_else(|| std::io::Error::other("no home folder"))?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let text = toml::to_string_pretty(self).map_err(std::io::Error::other)?;
        std::fs::write(path, text)
    }

    /// Makes sure `path` is in the history. A project that is already there
    /// keeps its place: opening one must not reorder the list, which follows
    /// the dates of the analyses.
    pub fn add(&mut self, path: &Path) {
        if !self.projects.iter().any(|r| r.path == path) {
            self.touch(path);
        }
    }

    /// Moves `path` to the top, keeping what is known about its last scan.
    pub fn touch(&mut self, path: &Path) {
        let summary = self
            .projects
            .iter()
            .find(|r| r.path == path)
            .and_then(|r| r.summary);
        self.projects.retain(|r| r.path != path);
        self.projects.insert(
            0,
            Recent {
                path: path.to_path_buf(),
                last_opened: now(),
                summary,
            },
        );
        self.projects.truncate(MAX_RECENTS);
    }

    /// Records how the latest scan of `path` went.
    pub fn record(&mut self, path: &Path, summary: Summary) {
        if let Some(recent) = self.projects.iter_mut().find(|r| r.path == path) {
            recent.summary = Some(summary);
        }
    }

    pub fn remove(&mut self, path: &Path) {
        self.projects.retain(|r| r.path != path);
    }

    pub fn clear(&mut self) {
        self.projects.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn summary(broken: usize, weakened: usize) -> Summary {
        Summary {
            broken,
            weakened,
            ..Summary::default()
        }
    }

    #[test]
    fn health_follows_the_worst_finding() {
        assert_eq!(summary(0, 0).health(), Health::Clean);
        assert_eq!(summary(0, 2).health(), Health::Warning);
        assert_eq!(summary(1, 2).health(), Health::AtRisk);
    }

    #[test]
    fn touch_moves_to_the_top_and_keeps_the_summary() {
        let mut recents = Recents::default();
        recents.touch(Path::new("/a"));
        recents.record(Path::new("/a"), summary(1, 0));
        recents.touch(Path::new("/b"));
        recents.touch(Path::new("/a"));
        let paths: Vec<_> = recents.projects.iter().map(|r| r.path.clone()).collect();
        assert_eq!(paths, [PathBuf::from("/a"), PathBuf::from("/b")]);
        assert_eq!(recents.projects[0].health(), Health::AtRisk);
        assert_eq!(recents.projects[1].health(), Health::Unknown);
    }

    #[test]
    fn adding_a_known_project_keeps_its_place() {
        let mut recents = Recents::default();
        recents.touch(Path::new("/a"));
        recents.touch(Path::new("/b"));
        recents.add(Path::new("/a"));
        let paths: Vec<_> = recents.projects.iter().map(|r| r.path.clone()).collect();
        assert_eq!(paths, [PathBuf::from("/b"), PathBuf::from("/a")]);
        // A new one goes to the top.
        recents.add(Path::new("/c"));
        assert_eq!(recents.projects[0].path, PathBuf::from("/c"));
    }

    #[test]
    fn history_is_capped() {
        let mut recents = Recents::default();
        for i in 0..MAX_RECENTS + 5 {
            recents.touch(Path::new(&format!("/p{i}")));
        }
        assert_eq!(recents.projects.len(), MAX_RECENTS);
        assert_eq!(recents.projects[0].path, PathBuf::from("/p24"));
    }

    #[test]
    fn round_trips_through_toml_and_tolerates_junk() {
        let mut recents = Recents::default();
        recents.touch(Path::new("/x/y"));
        recents.record(Path::new("/x/y"), summary(2, 1));
        let text = toml::to_string_pretty(&recents).unwrap();
        assert_eq!(toml::from_str::<Recents>(&text).unwrap(), recents);
        assert_eq!(toml::from_str::<Recents>("").unwrap(), Recents::default());
    }

    #[test]
    fn kind_comes_from_root_files() {
        let dir = std::env::temp_dir().join(format!("agentq-kind-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(Kind::detect(&dir), Kind::Folder);
        std::fs::write(dir.join("package.json"), "{}").unwrap();
        assert_eq!(Kind::detect(&dir), Kind::Node);
        std::fs::write(dir.join("tsconfig.json"), "{}").unwrap();
        assert_eq!(Kind::detect(&dir), Kind::TypeScript);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn names_come_from_the_last_component() {
        assert_eq!(project_name(Path::new("/home/me/shop")), "shop");
        assert_eq!(project_name(Path::new("/")), "/");
    }
}
