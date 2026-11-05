//! State of the terminal interface, independent of how it is drawn.

use crate::analysis::Analysis;
use crate::assess::{Assessment, Urgency};
use crate::finding::{Confidence, QuantumRisk};
use crate::report::sort_most_urgent_first;
use ratatui::crossterm::event::{KeyCode, MouseButton, MouseEventKind};
use ratatui::layout::{Position, Rect};
use ratatui::widgets::ListState;
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Work the event loop must do on behalf of the app.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// Analyze the directory again.
    Rescan,
}

/// The two pages of the results screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    /// Totals and rankings.
    Overview,
    /// The list of findings with their details.
    Findings,
}

/// Where things were drawn in the last frame, so mouse clicks can be mapped back.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Areas {
    pub tabs: Vec<(Rect, Tab)>,
    /// The rows of the findings list (inside its border).
    pub list: Rect,
    /// The text area of the details pane.
    pub details: Rect,
    /// One row per urgency level on the overview.
    pub urgency_rows: Vec<(Rect, Urgency)>,
}

/// How many visible findings fall in each risk class.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Counts {
    pub total: usize,
    pub broken: usize,
    pub weakened: usize,
    pub safe: usize,
}

pub struct App {
    /// The directory being analyzed, as given by the user.
    pub root: String,
    pub file_count: usize,
    pub knowledge_version: String,
    all: Vec<Assessment>,
    pub hide_safe: bool,
    pub min_confidence: Confidence,
    /// Index into [`App::visible`].
    pub selected: usize,
    pub list_state: ListState,
    /// Lines the details pane is scrolled down.
    pub detail_scroll: u16,
    /// Largest useful scroll of the details pane; set by the renderer.
    pub detail_max_scroll: u16,
    pub tab: Tab,
    /// Filled in by the renderer on every frame.
    pub areas: Areas,
    /// Files per language, most files first.
    languages: Vec<(String, usize)>,
    pub show_help: bool,
    pub should_quit: bool,
    /// One-line message about the last action, cleared on the next key.
    pub status: Option<String>,
}

impl App {
    pub fn new(
        root: String,
        analysis: Analysis,
        knowledge_version: String,
        hide_safe: bool,
        min_confidence: Confidence,
    ) -> App {
        App {
            root,
            file_count: analysis.files.len(),
            languages: language_counts(&analysis),
            tab: Tab::Overview,
            areas: Areas::default(),
            knowledge_version,
            all: analysis.assessments,
            hide_safe,
            min_confidence,
            selected: 0,
            list_state: ListState::default(),
            detail_scroll: 0,
            detail_max_scroll: 0,
            show_help: false,
            should_quit: false,
            status: None,
        }
    }

    /// Replaces the findings after a new analysis, keeping filters and selection.
    pub fn replace(&mut self, analysis: Analysis) {
        self.file_count = analysis.files.len();
        self.languages = language_counts(&analysis);
        self.all = analysis.assessments;
        self.clamp_selection();
        self.detail_scroll = 0;
        self.status = Some(format!(
            "Scanned again: {} use(s) in {} file(s)",
            self.all.len(),
            self.file_count
        ));
    }

    /// Findings that pass the filters, most urgent first.
    pub fn visible(&self) -> Vec<&Assessment> {
        let mut visible: Vec<&Assessment> = self
            .all
            .iter()
            .filter(|a| a.finding.confidence >= self.min_confidence)
            .filter(|a| !self.hide_safe || a.risk != QuantumRisk::Safe)
            .collect();
        sort_most_urgent_first(&mut visible);
        visible
    }

    pub fn selected_assessment(&self) -> Option<&Assessment> {
        self.visible().get(self.selected).copied()
    }

    /// Counts of the visible findings by risk.
    pub fn counts(&self) -> Counts {
        let mut counts = Counts::default();
        for assessment in self.visible() {
            counts.total += 1;
            match assessment.risk {
                QuantumRisk::Broken => counts.broken += 1,
                QuantumRisk::Weakened => counts.weakened += 1,
                QuantumRisk::Safe => counts.safe += 1,
            }
        }
        counts
    }

    /// Visible findings per urgency, from the most to the least urgent.
    pub fn urgency_counts(&self) -> [(Urgency, usize); 4] {
        let mut counts = [
            (Urgency::High, 0),
            (Urgency::Medium, 0),
            (Urgency::Low, 0),
            (Urgency::None, 0),
        ];
        for assessment in self.visible() {
            if let Some(entry) = counts.iter_mut().find(|(u, _)| *u == assessment.urgency) {
                entry.1 += 1;
            }
        }
        counts
    }

    /// The `limit` algorithms used most often among the visible findings.
    pub fn top_algorithms(&self, limit: usize) -> Vec<(String, usize)> {
        let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
        for assessment in self.visible() {
            *counts.entry(&assessment.algorithm_name).or_default() += 1;
        }
        let mut ranked: Vec<(String, usize)> = counts
            .into_iter()
            .map(|(name, count)| (name.to_string(), count))
            .collect();
        ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        ranked.truncate(limit);
        ranked
    }

    /// The `limit` files to look at first: worst urgency, then most findings.
    pub fn top_files(&self, limit: usize) -> Vec<(PathBuf, usize, Urgency)> {
        let mut files: BTreeMap<&PathBuf, (usize, Urgency)> = BTreeMap::new();
        for assessment in self.visible() {
            let entry = files
                .entry(&assessment.finding.file)
                .or_insert((0, Urgency::None));
            entry.0 += 1;
            entry.1 = entry.1.max(assessment.urgency);
        }
        let mut ranked: Vec<(PathBuf, usize, Urgency)> = files
            .into_iter()
            .map(|(file, (count, urgency))| (file.clone(), count, urgency))
            .collect();
        ranked.sort_by(|a, b| {
            b.2.cmp(&a.2)
                .then_with(|| b.1.cmp(&a.1))
                .then_with(|| a.0.cmp(&b.0))
        });
        ranked.truncate(limit);
        ranked
    }

    /// Files per language, most files first.
    pub fn languages(&self) -> &[(String, usize)] {
        &self.languages
    }

    /// Number of findings hidden by the filters.
    pub fn hidden(&self) -> usize {
        self.all.len() - self.visible().len()
    }

    pub fn handle_key(&mut self, key: KeyCode) -> Option<Command> {
        self.status = None;
        match key {
            KeyCode::Char('q') | KeyCode::Esc => {
                if self.show_help {
                    self.show_help = false;
                } else {
                    self.should_quit = true;
                }
            }
            KeyCode::Char('?') | KeyCode::F(1) => self.show_help = !self.show_help,
            _ if self.show_help => {}
            KeyCode::Char('1') => self.tab = Tab::Overview,
            KeyCode::Char('2') => self.tab = Tab::Findings,
            KeyCode::Tab | KeyCode::BackTab => self.tab = self.tab.other(),
            KeyCode::Enter if self.tab == Tab::Overview => self.tab = Tab::Findings,
            KeyCode::Down | KeyCode::Char('j') if self.tab == Tab::Findings => {
                self.select(self.selected.saturating_add(1))
            }
            KeyCode::Up | KeyCode::Char('k') if self.tab == Tab::Findings => {
                self.select(self.selected.saturating_sub(1))
            }
            KeyCode::Home | KeyCode::Char('g') if self.tab == Tab::Findings => self.select(0),
            KeyCode::End | KeyCode::Char('G') if self.tab == Tab::Findings => {
                self.select(usize::MAX)
            }
            KeyCode::PageDown | KeyCode::Char(' ') if self.tab == Tab::Findings => {
                self.scroll_details(5)
            }
            KeyCode::PageUp if self.tab == Tab::Findings => self.scroll_details(-5),
            KeyCode::Char('s') => {
                self.hide_safe = !self.hide_safe;
                self.filters_changed();
            }
            KeyCode::Char('c') => {
                self.min_confidence = match self.min_confidence {
                    Confidence::Low => Confidence::Medium,
                    Confidence::Medium => Confidence::High,
                    Confidence::High => Confidence::Low,
                };
                self.filters_changed();
            }
            KeyCode::Char('r') => return Some(Command::Rescan),
            _ => {}
        }
        None
    }

    /// Reacts to a mouse event at the given terminal cell.
    ///
    /// Click a tab to switch to it, a finding to select it, or an urgency on
    /// the overview to jump to its first finding. The wheel moves the selection
    /// over the list and scrolls the details when over them.
    pub fn handle_mouse(&mut self, kind: MouseEventKind, column: u16, row: u16) {
        self.status = None;
        let at = Position::new(column, row);
        match kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if self.show_help {
                    self.show_help = false;
                    return;
                }
                if let Some(tab) = self
                    .areas
                    .tabs
                    .iter()
                    .find(|(area, _)| area.contains(at))
                    .map(|(_, tab)| *tab)
                {
                    self.tab = tab;
                    return;
                }
                match self.tab {
                    Tab::Overview => {
                        let clicked = self
                            .areas
                            .urgency_rows
                            .iter()
                            .find(|(area, _)| area.contains(at))
                            .map(|(_, urgency)| *urgency);
                        if let Some(urgency) = clicked {
                            self.jump_to(urgency);
                        }
                    }
                    Tab::Findings => {
                        if self.areas.list.contains(at) {
                            // Each finding takes two rows.
                            let rows_down = usize::from(row - self.areas.list.y) / 2;
                            let index = self.list_state.offset() + rows_down;
                            if index < self.visible().len() {
                                self.select(index);
                            }
                        }
                    }
                }
            }
            MouseEventKind::ScrollDown | MouseEventKind::ScrollUp if !self.show_help => {
                if self.tab != Tab::Findings {
                    return;
                }
                let down = kind == MouseEventKind::ScrollDown;
                if self.areas.list.contains(at) {
                    let next = if down {
                        self.selected.saturating_add(1)
                    } else {
                        self.selected.saturating_sub(1)
                    };
                    self.select(next);
                } else if self.areas.details.contains(at) {
                    self.scroll_details(if down { 3 } else { -3 });
                }
            }
            _ => {}
        }
    }

    /// Opens the findings on the first one with this urgency.
    fn jump_to(&mut self, urgency: Urgency) {
        let first = self.visible().iter().position(|a| a.urgency == urgency);
        match first {
            Some(index) => {
                self.tab = Tab::Findings;
                self.selected = index;
                self.detail_scroll = 0;
            }
            None => self.status = Some("No finding with that urgency".to_string()),
        }
    }

    fn select(&mut self, index: usize) {
        let before = self.selected;
        let last = self.visible().len().saturating_sub(1);
        self.selected = index.min(last);
        if self.selected != before {
            self.detail_scroll = 0;
        }
    }

    fn scroll_details(&mut self, delta: i32) {
        let target =
            (i32::from(self.detail_scroll) + delta).clamp(0, i32::from(self.detail_max_scroll));
        self.detail_scroll = target as u16;
    }

    fn filters_changed(&mut self) {
        self.clamp_selection();
        self.detail_scroll = 0;
    }

    fn clamp_selection(&mut self) {
        let last = self.visible().len().saturating_sub(1);
        self.selected = self.selected.min(last);
    }
}

impl Tab {
    pub fn other(self) -> Tab {
        match self {
            Tab::Overview => Tab::Findings,
            Tab::Findings => Tab::Overview,
        }
    }
}

fn language_counts(analysis: &Analysis) -> Vec<(String, usize)> {
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for (_, language) in &analysis.files {
        *counts.entry(language.label()).or_default() += 1;
    }
    let mut ranked: Vec<(String, usize)> = counts
        .into_iter()
        .map(|(label, count)| (label.to_string(), count))
        .collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    ranked
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::fixtures;

    fn app() -> App {
        let analysis = Analysis {
            files: fixtures::files(),
            assessments: fixtures::assessments(),
        };
        let mut app = App::new(
            "project".into(),
            analysis,
            "test".into(),
            false,
            Confidence::Low,
        );
        app.tab = Tab::Findings;
        app
    }

    fn algorithms(app: &App) -> Vec<String> {
        app.visible()
            .iter()
            .map(|a| a.finding.algorithm.clone())
            .collect()
    }

    #[test]
    fn findings_are_listed_most_urgent_first() {
        assert_eq!(algorithms(&app()), ["ecdh", "ecdsa", "sha2-256"]);
    }

    #[test]
    fn selection_moves_and_stays_inside_the_list() {
        let mut app = app();
        app.handle_key(KeyCode::Up);
        assert_eq!(app.selected, 0);
        app.handle_key(KeyCode::Down);
        app.handle_key(KeyCode::Char('j'));
        app.handle_key(KeyCode::Down);
        assert_eq!(app.selected, 2);
        app.handle_key(KeyCode::Char('g'));
        assert_eq!(app.selected, 0);
        app.handle_key(KeyCode::Char('G'));
        assert_eq!(app.selected, 2);
        assert_eq!(
            app.selected_assessment().unwrap().finding.algorithm,
            "sha2-256"
        );
    }

    #[test]
    fn hiding_safe_findings_removes_them_and_keeps_selection_valid() {
        let mut app = app();
        app.handle_key(KeyCode::Char('G'));
        app.handle_key(KeyCode::Char('s'));
        assert!(app.hide_safe);
        assert_eq!(algorithms(&app), ["ecdh", "ecdsa"]);
        assert_eq!(app.selected, 1);
        assert_eq!(app.hidden(), 1);
        app.handle_key(KeyCode::Char('s'));
        assert_eq!(app.hidden(), 0);
    }

    #[test]
    fn confidence_filter_cycles_and_filters() {
        let mut app = app();
        assert_eq!(app.min_confidence, Confidence::Low);
        app.handle_key(KeyCode::Char('c'));
        assert_eq!(app.min_confidence, Confidence::Medium);
        app.handle_key(KeyCode::Char('c'));
        assert_eq!(app.min_confidence, Confidence::High);
        // All fixture findings have high confidence.
        assert_eq!(app.visible().len(), 3);
        app.handle_key(KeyCode::Char('c'));
        assert_eq!(app.min_confidence, Confidence::Low);
    }

    #[test]
    fn counts_follow_the_visible_findings() {
        let mut app = app();
        assert_eq!(
            app.counts(),
            Counts {
                total: 3,
                broken: 2,
                weakened: 0,
                safe: 1
            }
        );
        app.handle_key(KeyCode::Char('s'));
        assert_eq!(app.counts().safe, 0);
    }

    #[test]
    fn quit_help_and_rescan_keys() {
        let mut app = app();
        assert_eq!(app.handle_key(KeyCode::Char('r')), Some(Command::Rescan));
        app.handle_key(KeyCode::Char('?'));
        assert!(app.show_help);
        // While help is open, navigation is ignored and Esc only closes it.
        app.handle_key(KeyCode::Down);
        assert_eq!(app.selected, 0);
        app.handle_key(KeyCode::Esc);
        assert!(!app.show_help && !app.should_quit);
        app.handle_key(KeyCode::Char('q'));
        assert!(app.should_quit);
    }

    #[test]
    fn details_scroll_is_limited_and_resets_with_the_selection() {
        let mut app = app();
        app.detail_max_scroll = 7;
        app.handle_key(KeyCode::PageDown);
        app.handle_key(KeyCode::PageDown);
        assert_eq!(app.detail_scroll, 7);
        app.handle_key(KeyCode::PageUp);
        assert_eq!(app.detail_scroll, 2);
        app.handle_key(KeyCode::Down);
        assert_eq!(app.detail_scroll, 0);
    }

    #[test]
    fn replacing_the_analysis_keeps_filters_and_clamps_the_selection() {
        let mut app = app();
        app.handle_key(KeyCode::Char('s'));
        app.handle_key(KeyCode::Char('G'));
        app.replace(Analysis {
            files: fixtures::files(),
            assessments: vec![],
        });
        assert!(app.hide_safe);
        assert_eq!(app.selected, 0);
        assert!(app.visible().is_empty());
        assert!(app.status.as_deref().unwrap().contains("0 use(s)"));
    }

    #[test]
    fn starts_on_the_overview_and_switches_tabs() {
        let analysis = Analysis {
            files: fixtures::files(),
            assessments: fixtures::assessments(),
        };
        let mut app = App::new(
            "project".into(),
            analysis,
            "t".into(),
            false,
            Confidence::Low,
        );
        assert_eq!(app.tab, Tab::Overview);
        // Selection keys do nothing on the overview.
        app.handle_key(KeyCode::Down);
        assert_eq!(app.selected, 0);
        app.handle_key(KeyCode::Char('2'));
        assert_eq!(app.tab, Tab::Findings);
        app.handle_key(KeyCode::Tab);
        assert_eq!(app.tab, Tab::Overview);
        app.handle_key(KeyCode::Enter);
        assert_eq!(app.tab, Tab::Findings);
        app.handle_key(KeyCode::Char('1'));
        assert_eq!(app.tab, Tab::Overview);
    }

    #[test]
    fn overview_numbers_follow_the_visible_findings() {
        let mut app = app();
        assert_eq!(
            app.urgency_counts(),
            [
                (Urgency::High, 1),
                (Urgency::Medium, 1),
                (Urgency::Low, 0),
                (Urgency::None, 1)
            ]
        );
        let algorithms: Vec<_> = app.top_algorithms(5).into_iter().map(|(n, _)| n).collect();
        assert_eq!(algorithms, ["ECDH (NIST curves)", "ECDSA", "SHA-256"]);
        assert_eq!(app.top_algorithms(1).len(), 1);

        let files = app.top_files(5);
        // src/a.js holds a medium and a safe finding, src/b.ts a high one.
        assert_eq!(files[0].0, PathBuf::from("src/b.ts"));
        assert_eq!(files[0].2, Urgency::High);
        assert_eq!(
            (files[1].0.clone(), files[1].1),
            (PathBuf::from("src/a.js"), 2)
        );

        assert_eq!(
            app.languages(),
            [
                ("JS/TS".to_string(), 2),
                ("Go".to_string(), 1),
                ("Python".to_string(), 1)
            ]
        );
        app.handle_key(KeyCode::Char('s'));
        assert_eq!(app.urgency_counts()[3].1, 0);
    }
}
