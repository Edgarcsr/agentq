//! State of the terminal interface, independent of how it is drawn.

use crate::analysis::Analysis;
use crate::assess::Assessment;
use crate::finding::{Confidence, QuantumRisk};
use crate::report::sort_most_urgent_first;
use ratatui::crossterm::event::KeyCode;
use ratatui::widgets::ListState;

/// Work the event loop must do on behalf of the app.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// Analyze the directory again.
    Rescan,
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
            KeyCode::Down | KeyCode::Char('j') => self.select(self.selected.saturating_add(1)),
            KeyCode::Up | KeyCode::Char('k') => self.select(self.selected.saturating_sub(1)),
            KeyCode::Home | KeyCode::Char('g') => self.select(0),
            KeyCode::End | KeyCode::Char('G') => self.select(usize::MAX),
            KeyCode::PageDown | KeyCode::Char(' ') => self.scroll_details(5),
            KeyCode::PageUp => self.scroll_details(-5),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::fixtures;

    fn app() -> App {
        let analysis = Analysis {
            files: fixtures::files(),
            assessments: fixtures::assessments(),
        };
        App::new(
            "project".into(),
            analysis,
            "test".into(),
            false,
            Confidence::Low,
        )
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
}
