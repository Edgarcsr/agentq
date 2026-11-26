//! Scans planned for later, kept in `~/.agentq/schedule.toml`.
//!
//! Any number of scans can be waiting, including several for one project. A
//! scan that repeats is moved forward when it runs; one that does not is
//! dropped. A missing or broken file is the same as nothing scheduled.

use crate::config::Settings;
use chrono::{Duration, Local, TimeZone};
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

/// Reads "9:30" or "09:30" as an hour and a minute; anything else is `None`.
pub fn parse_time(text: &str) -> Option<(u32, u32)> {
    let (hour, minute) = text.trim().split_once(':')?;
    let (hour, minute) = (hour.trim().parse().ok()?, minute.trim().parse().ok()?);
    (hour < 24 && minute < 60).then_some((hour, minute))
}

/// The next moment the local clock reads `hour:minute`, strictly after `now`
/// (seconds since the Unix epoch): today if it has not passed yet, otherwise
/// tomorrow.
pub fn next_at(hour: u32, minute: u32, now: u64) -> Option<u64> {
    let local = Local.timestamp_opt(i64::try_from(now).ok()?, 0).single()?;
    (0..=2).find_map(|days| {
        let day = local.date_naive() + Duration::days(days);
        let at = Local
            .from_local_datetime(&day.and_hms_opt(hour, minute, 0)?)
            .earliest()?
            .timestamp();
        u64::try_from(at).ok().filter(|&at| at > now)
    })
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
    use chrono::Timelike;

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
    fn times_are_read_from_hh_mm() {
        assert_eq!(parse_time("09:30"), Some((9, 30)));
        assert_eq!(parse_time(" 9:05 "), Some((9, 5)));
        assert_eq!(parse_time("24:00"), None);
        assert_eq!(parse_time("12:60"), None);
        assert_eq!(parse_time("noon"), None);
        assert_eq!(parse_time("1230"), None);
    }

    #[test]
    fn next_at_is_today_when_ahead_and_tomorrow_when_past() {
        let now = 1_800_000_000;
        let local = Local.timestamp_opt(now as i64, 0).unwrap();
        let ahead = local + Duration::minutes(90);
        let behind = local - Duration::minutes(90);
        let at = |t: chrono::DateTime<Local>| next_at(t.hour(), t.minute(), now).unwrap();
        let secs = |t: chrono::DateTime<Local>| t.timestamp() as u64 - u64::from(t.second());
        assert_eq!(at(ahead), secs(ahead));
        assert_eq!(at(behind), secs(behind + Duration::days(1)));
        assert!(at(ahead) > now && at(ahead) - now <= 86_400);
        assert!(at(behind) > now && at(behind) - now <= 2 * 86_400);
    }

    #[test]
    fn until_reads_naturally() {
        assert_eq!(until(10, 10), "due");
        assert_eq!(until(70, 10), "in 1m");
        assert_eq!(until(10 + 7200, 10), "in 2h");
        assert_eq!(until(10 + 3 * 86_400, 10), "in 3d");
    }
}
