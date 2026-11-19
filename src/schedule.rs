//! Scans planned for later, kept in `~/.agentq/schedule.toml`.
//!
//! Any number of scans can be waiting, including several for one project. A
//! scan that repeats is moved forward when it runs; one that does not is
//! dropped. A missing or broken file is the same as nothing scheduled.

use crate::config::Settings;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Repeat {
    Once,
    Daily,
    Weekly,
}

impl Repeat {
    /// Seconds between runs, if it repeats.
    pub fn period(self) -> Option<u64> {
        match self {
            Repeat::Once => None,
            Repeat::Daily => Some(86_400),
            Repeat::Weekly => Some(7 * 86_400),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Repeat::Once => "Once",
            Repeat::Daily => "Daily",
            Repeat::Weekly => "Weekly",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Schedule {
    pub id: u64,
    pub path: PathBuf,
    /// When to run, in seconds since the Unix epoch.
    pub at: u64,
    pub repeat: Repeat,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Schedules {
    pub items: Vec<Schedule>,
}

impl Schedules {
    pub fn path() -> Option<PathBuf> {
        Settings::dir().map(|dir| dir.join("schedule.toml"))
    }

    pub fn load() -> Schedules {
        Schedules::path()
            .and_then(|path| std::fs::read_to_string(path).ok())
            .and_then(|text| toml::from_str(&text).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> std::io::Result<()> {
        let path = Schedules::path().ok_or_else(|| std::io::Error::other("no home folder"))?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let text = toml::to_string_pretty(self).map_err(std::io::Error::other)?;
        std::fs::write(path, text)
    }

    /// Plans a scan of `path` at `at`; returns its id.
    pub fn add(&mut self, path: &Path, at: u64, repeat: Repeat) -> u64 {
        let id = self.items.iter().map(|s| s.id).max().map_or(1, |m| m + 1);
        self.items.push(Schedule {
            id,
            path: path.to_path_buf(),
            at,
            repeat,
        });
        self.items.sort_by_key(|s| (s.at, s.id));
        id
    }

    pub fn cancel(&mut self, id: u64) {
        self.items.retain(|s| s.id != id);
    }

    /// The scans that should have started by `now`.
    pub fn due(&self, now: u64) -> Vec<Schedule> {
        self.items.iter().filter(|s| s.at <= now).cloned().collect()
    }

    /// Records that schedule `id` ran: drops it, or moves it past `now`.
    pub fn completed(&mut self, id: u64, now: u64) {
        let Some(item) = self.items.iter_mut().find(|s| s.id == id) else {
            return;
        };
        match item.repeat.period() {
            None => self.cancel(id),
            Some(period) => {
                // Skip runs missed while the app was closed instead of
                // running them all one after the other.
                let behind = now.saturating_sub(item.at) / period + 1;
                item.at += behind * period;
                self.items.sort_by_key(|s| (s.at, s.id));
            }
        }
    }

    /// The soonest scan planned for `path`.
    pub fn next_for(&self, path: &Path) -> Option<&Schedule> {
        self.items.iter().find(|s| s.path == path)
    }
}

/// "in 5m", "in 3h", "in 2d"; "due" when it is already time.
pub fn until(at: u64, now: u64) -> String {
    match at.saturating_sub(now) {
        0 => "due".into(),
        s @ 1..=3599 => format!("in {}m", s.div_ceil(60)),
        s @ 3600..=86_399 => format!("in {}h", s / 3600),
        s => format!("in {}d", s / 86_400),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn several_scans_can_wait_and_stay_sorted() {
        let mut s = Schedules::default();
        let a = s.add(Path::new("/a"), 300, Repeat::Once);
        let b = s.add(Path::new("/a"), 100, Repeat::Daily);
        assert_ne!(a, b);
        let order: Vec<_> = s.items.iter().map(|i| i.at).collect();
        assert_eq!(order, [100, 300]);
        assert_eq!(s.next_for(Path::new("/a")).unwrap().at, 100);
        assert!(s.next_for(Path::new("/z")).is_none());
    }

    #[test]
    fn due_returns_only_what_is_ready() {
        let mut s = Schedules::default();
        s.add(Path::new("/a"), 100, Repeat::Once);
        s.add(Path::new("/b"), 500, Repeat::Once);
        assert_eq!(s.due(99).len(), 0);
        assert_eq!(s.due(100).len(), 1);
        assert_eq!(s.due(1000).len(), 2);
    }

    #[test]
    fn one_off_scans_are_dropped_and_repeating_ones_move_on() {
        let mut s = Schedules::default();
        let once = s.add(Path::new("/a"), 100, Repeat::Once);
        let daily = s.add(Path::new("/b"), 100, Repeat::Daily);
        s.completed(once, 150);
        s.completed(daily, 150);
        assert_eq!(s.items.len(), 1);
        assert_eq!(s.items[0].at, 100 + 86_400);
    }

    #[test]
    fn missed_runs_are_skipped() {
        let mut s = Schedules::default();
        let id = s.add(Path::new("/a"), 0, Repeat::Daily);
        s.completed(id, 3 * 86_400 + 10);
        assert_eq!(s.items[0].at, 4 * 86_400);
    }

    #[test]
    fn round_trips_and_tolerates_junk() {
        let mut s = Schedules::default();
        s.add(Path::new("/a"), 42, Repeat::Weekly);
        let text = toml::to_string_pretty(&s).unwrap();
        assert_eq!(toml::from_str::<Schedules>(&text).unwrap(), s);
        assert_eq!(
            toml::from_str::<Schedules>("").unwrap(),
            Schedules::default()
        );
    }

    #[test]
    fn until_reads_naturally() {
        assert_eq!(until(10, 10), "due");
        assert_eq!(until(70, 10), "in 1m");
        assert_eq!(until(10 + 7200, 10), "in 2h");
        assert_eq!(until(10 + 3 * 86_400, 10), "in 3d");
    }
}
