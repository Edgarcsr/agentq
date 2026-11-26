//! The analyses of a project as they were when they ran, kept in
//! `~/.agentq/history/`: one folder per project, one file per scan.
//!
//! The history of projects would be only a list of names and numbers without
//! this: opening a project from it shows what was found back then, at once,
//! and the user decides when to scan again. Older scans stay, so the sidebar
//! can list the dates of a project and open any of them. A missing or broken
//! file is the same as no snapshot.

use crate::assess::Assessment;
use crate::config::Settings;
use crate::detect::Language;
use crate::recents::Summary;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    pub path: PathBuf,
    /// When the scan finished, in seconds since the Unix epoch.
    pub taken_at: u64,
    /// The language the advice was written in (`Lang::code`).
    pub lang: String,
    pub summary: Summary,
    /// Source languages found, most files first.
    pub languages: Vec<(Language, usize)>,
    /// Most urgent first.
    pub findings: Vec<Assessment>,
}

/// A stable 64-bit hash (FNV-1a): the file name must not change between
/// versions of Rust, which is why `DefaultHasher` is not used.
fn fnv(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// How many analyses are kept for one project; the oldest are dropped.
pub const MAX_PER_PROJECT: usize = 20;

/// The folder that holds every analysis of `project`.
fn project_dir(dir: &Path, project: &Path) -> PathBuf {
    let name: String = project
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .take(40)
        .collect();
    let hash = fnv(&project.to_string_lossy());
    dir.join(format!("{name}-{hash:016x}"))
}

fn file_in(dir: &Path, project: &Path, taken_at: u64) -> PathBuf {
    project_dir(dir, project).join(format!("{taken_at}.json"))
}

impl Snapshot {
    pub fn dir() -> Option<PathBuf> {
        Settings::dir().map(|dir| dir.join("history"))
    }

    /// When each analysis of `project` was made, newest first.
    pub fn versions(project: &Path) -> Vec<u64> {
        Snapshot::dir().map_or_else(Vec::new, |dir| Snapshot::versions_in(&dir, project))
    }

    /// The latest analysis of `project`.
    pub fn load(project: &Path) -> Option<Snapshot> {
        let dir = Snapshot::dir()?;
        let latest = *Snapshot::versions_in(&dir, project).first()?;
        Snapshot::load_from(&dir, project, latest)
    }

    /// The analysis of `project` made at `taken_at`.
    pub fn load_at(project: &Path, taken_at: u64) -> Option<Snapshot> {
        Snapshot::load_from(&Snapshot::dir()?, project, taken_at)
    }

    pub fn save(&self) -> std::io::Result<()> {
        let dir = Snapshot::dir().ok_or_else(|| std::io::Error::other("no home folder"))?;
        self.save_to(&dir)
    }

    /// Forgets every analysis of `project`.
    pub fn remove(project: &Path) {
        if let Some(dir) = Snapshot::dir() {
            let _ = std::fs::remove_dir_all(project_dir(&dir, project));
        }
    }

    /// Forgets the analysis of `project` made at `taken_at`.
    pub fn remove_at(project: &Path, taken_at: u64) {
        if let Some(dir) = Snapshot::dir() {
            Snapshot::remove_at_in(&dir, project, taken_at);
        }
    }

    fn remove_at_in(dir: &Path, project: &Path, taken_at: u64) {
        let _ = std::fs::remove_file(file_in(dir, project, taken_at));
    }

    fn versions_in(dir: &Path, project: &Path) -> Vec<u64> {
        let Ok(entries) = std::fs::read_dir(project_dir(dir, project)) else {
            return Vec::new();
        };
        let mut found: Vec<u64> = entries
            .flatten()
            .filter_map(|e| e.path().file_stem()?.to_str()?.parse().ok())
            .collect();
        found.sort_unstable_by(|a, b| b.cmp(a));
        found
    }

    fn load_from(dir: &Path, project: &Path, taken_at: u64) -> Option<Snapshot> {
        let text = std::fs::read_to_string(file_in(dir, project, taken_at)).ok()?;
        let snapshot: Snapshot = serde_json::from_str(&text).ok()?;
        // A name clash must never show another project's findings.
        (snapshot.path == project).then_some(snapshot)
    }

    fn save_to(&self, dir: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(project_dir(dir, &self.path))?;
        let text = serde_json::to_string(self).map_err(std::io::Error::other)?;
        std::fs::write(file_in(dir, &self.path, self.taken_at), text)?;
        for old in Snapshot::versions_in(dir, &self.path)
            .into_iter()
            .skip(MAX_PER_PROJECT)
        {
            let _ = std::fs::remove_file(file_in(dir, &self.path, old));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::analyze_directory;
    use crate::i18n::Lang;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("agentq-snap-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn sample(path: &Path, taken_at: u64) -> Snapshot {
        Snapshot {
            path: path.to_path_buf(),
            taken_at,
            lang: Lang::En.code().to_string(),
            summary: Summary::default(),
            languages: vec![],
            findings: vec![],
        }
    }

    #[test]
    fn an_analysis_comes_back_as_it_was_saved() {
        let project = temp("project");
        std::fs::write(
            project.join("a.js"),
            "const c = require('crypto'); c.createHash('md5'); c.generateKeyPairSync('rsa', { modulusLength: 2048 });",
        )
        .unwrap();
        let analysis = analyze_directory(&project, Lang::En);
        assert!(!analysis.assessments.is_empty());
        let snapshot = Snapshot {
            path: project.clone(),
            taken_at: 42,
            lang: Lang::En.code().to_string(),
            summary: Summary::from_analysis(&analysis),
            languages: vec![(Language::JavaScriptTypeScript, 1)],
            findings: analysis.assessments,
        };
        let store = temp("store");
        snapshot.save_to(&store).unwrap();
        assert_eq!(Snapshot::load_from(&store, &project, 42), Some(snapshot));
        let _ = std::fs::remove_dir_all(&project);
        let _ = std::fs::remove_dir_all(&store);
    }

    #[test]
    fn every_scan_is_kept_newest_first_and_the_oldest_are_dropped() {
        let store = temp("many");
        let project = Path::new("/p/app");
        for at in 1..=(MAX_PER_PROJECT as u64 + 3) {
            sample(project, at).save_to(&store).unwrap();
        }
        let versions = Snapshot::versions_in(&store, project);
        assert_eq!(versions.len(), MAX_PER_PROJECT);
        assert_eq!(versions[0], MAX_PER_PROJECT as u64 + 3);
        assert_eq!(*versions.last().unwrap(), 4);
        assert!(Snapshot::load_from(&store, project, 1).is_none());
        assert!(Snapshot::load_from(&store, project, 10).is_some());
        let _ = std::fs::remove_dir_all(&store);
    }

    #[test]
    fn one_analysis_can_be_dropped_and_the_others_stay() {
        let store = temp("drop");
        let project = Path::new("/p/app");
        for at in [10, 20, 30] {
            sample(project, at).save_to(&store).unwrap();
        }
        Snapshot::remove_at_in(&store, project, 20);
        assert_eq!(Snapshot::versions_in(&store, project), [30, 10]);
        let _ = std::fs::remove_dir_all(&store);
    }

    #[test]
    fn projects_with_the_same_name_do_not_share_a_folder() {
        let dir = Path::new("/store");
        assert_ne!(
            project_dir(dir, Path::new("/a/app")),
            project_dir(dir, Path::new("/b/app"))
        );
        assert_eq!(
            project_dir(dir, Path::new("/a/app")),
            project_dir(dir, Path::new("/a/app"))
        );
    }

    #[test]
    fn missing_or_broken_files_are_no_snapshot() {
        let store = temp("junk");
        let project = Path::new("/no/such/project");
        assert_eq!(Snapshot::load_from(&store, project, 5), None);
        assert!(Snapshot::versions_in(&store, project).is_empty());
        std::fs::create_dir_all(project_dir(&store, project)).unwrap();
        std::fs::write(file_in(&store, project, 5), "not json").unwrap();
        assert_eq!(Snapshot::load_from(&store, project, 5), None);
        let _ = std::fs::remove_dir_all(&store);
    }
}
