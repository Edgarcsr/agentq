//! The folder browser shown when no path is given: pick the project to analyze.

use super::ui::{
    ACCENT, BORDER, FG, MUTED, ON_ACCENT, focused_panel, key_hints, row_bar, selected_symbol,
};
use crate::i18n::{Lang, fill};
use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, MouseButton, MouseEventKind};
use ratatui::layout::{Constraint, Layout, Position, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    /// Analyze the folder being shown.
    Analyze,
    Parent,
    Dir,
}

/// What a folder looks like, when it is recognizable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hint {
    Node,
    Git,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub kind: EntryKind,
    pub name: String,
    pub path: PathBuf,
    pub hint: Option<Hint>,
}

/// What the browser asks the application to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Analyze(PathBuf),
    /// Back to the main menu.
    Back,
}

pub struct Browser {
    pub dir: PathBuf,
    pub entries: Vec<Entry>,
    pub selected: usize,
    pub list_state: ListState,
    pub show_hidden: bool,
    /// The folder that could not be opened last, and why.
    pub error: Option<(PathBuf, String)>,
    /// The rows of the list, filled in by the renderer.
    pub list_area: Rect,
}

/// Recognizes folders worth analyzing.
fn hint_for(dir: &Path) -> Option<Hint> {
    if dir.join("package.json").is_file() {
        Some(Hint::Node)
    } else if dir.join(".git").exists() {
        Some(Hint::Git)
    } else {
        None
    }
}

fn list_dir(dir: &Path, show_hidden: bool) -> io::Result<Vec<Entry>> {
    let mut entries = vec![Entry {
        kind: EntryKind::Analyze,
        name: "Analyze this folder".to_string(),
        path: dir.to_path_buf(),
        hint: None,
    }];
    if let Some(parent) = dir.parent() {
        entries.push(Entry {
            kind: EntryKind::Parent,
            name: "..".to_string(),
            path: parent.to_path_buf(),
            hint: None,
        });
    }

    let mut folders: Vec<Entry> = Vec::new();
    for item in fs::read_dir(dir)? {
        let Ok(item) = item else { continue };
        let name = item.file_name().to_string_lossy().into_owned();
        if !show_hidden && name.starts_with('.') {
            continue;
        }
        let path = item.path();
        // `metadata` follows symlinks, so links to folders are listed too.
        if !fs::metadata(&path).is_ok_and(|m| m.is_dir()) {
            continue;
        }
        folders.push(Entry {
            kind: EntryKind::Dir,
            hint: hint_for(&path),
            name,
            path,
        });
    }
    folders.sort_by_key(|entry| entry.name.to_lowercase());
    entries.extend(folders);
    Ok(entries)
}

impl Browser {
    /// Opens the browser on `dir` (made absolute when possible).
    pub fn new(dir: PathBuf) -> Browser {
        let dir = fs::canonicalize(&dir).unwrap_or(dir);
        let mut browser = Browser {
            entries: Vec::new(),
            dir: dir.clone(),
            selected: 0,
            list_state: ListState::default(),
            show_hidden: false,
            error: None,
            list_area: Rect::default(),
        };
        browser.open(dir);
        browser
    }

    /// Shows `dir`. If it cannot be read, stays where it was and says why.
    pub fn open(&mut self, dir: PathBuf) {
        match list_dir(&dir, self.show_hidden) {
            Ok(entries) => {
                self.dir = dir;
                self.entries = entries;
                self.selected = 0;
                self.error = None;
            }
            Err(error) => {
                self.error = Some((dir, error.to_string()));
                if self.entries.is_empty() {
                    self.entries = list_dir(&self.dir, self.show_hidden).unwrap_or_default();
                }
            }
        }
    }

    /// Moves the selection to the entry for `path`, if it is listed.
    pub fn select_path(&mut self, path: &Path) {
        if let Some(index) = self.entries.iter().position(|e| e.path == path) {
            self.selected = index;
        }
    }

    fn go_up(&mut self) {
        if let Some(parent) = self.dir.parent().map(Path::to_path_buf) {
            let came_from = self.dir.clone();
            self.open(parent);
            self.select_path(&came_from);
        }
    }

    fn select(&mut self, index: usize) {
        self.selected = index.min(self.entries.len().saturating_sub(1));
    }

    /// Does what the selected entry says: analyze, or go into a folder.
    fn activate(&mut self) -> Option<Action> {
        let entry = self.entries.get(self.selected)?.clone();
        match entry.kind {
            EntryKind::Analyze => Some(Action::Analyze(self.dir.clone())),
            EntryKind::Parent => {
                self.go_up();
                None
            }
            EntryKind::Dir => {
                self.open(entry.path);
                None
            }
        }
    }

    pub fn handle_key(&mut self, key: KeyCode) -> Option<Action> {
        self.error = None;
        match key {
            KeyCode::Char('q') | KeyCode::Esc => return Some(Action::Back),
            KeyCode::Down | KeyCode::Char('j') => self.select(self.selected.saturating_add(1)),
            KeyCode::Up | KeyCode::Char('k') => self.select(self.selected.saturating_sub(1)),
            KeyCode::Home | KeyCode::Char('g') => self.select(0),
            KeyCode::End | KeyCode::Char('G') => self.select(usize::MAX),
            KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => return self.activate(),
            KeyCode::Left | KeyCode::Backspace | KeyCode::Char('h') => self.go_up(),
            KeyCode::Char('a') => return Some(Action::Analyze(self.dir.clone())),
            KeyCode::Char('~') => {
                if let Some(home) = std::env::var_os("HOME") {
                    self.open(PathBuf::from(home));
                }
            }
            KeyCode::Char('.') => {
                self.show_hidden = !self.show_hidden;
                let keep = self.entries.get(self.selected).map(|e| e.path.clone());
                let dir = self.dir.clone();
                self.open(dir);
                if let Some(path) = keep {
                    self.select_path(&path);
                }
            }
            _ => {}
        }
        None
    }

    /// Click a folder to select it and click it again to open it; the wheel
    /// moves the selection.
    pub fn handle_mouse(&mut self, kind: MouseEventKind, column: u16, row: u16) -> Option<Action> {
        self.error = None;
        let at = Position::new(column, row);
        match kind {
            MouseEventKind::Down(MouseButton::Left) if self.list_area.contains(at) => {
                let index = self.list_state.offset() + usize::from(row - self.list_area.y);
                if index >= self.entries.len() {
                    return None;
                }
                if index == self.selected {
                    return self.activate();
                }
                self.select(index);
                None
            }
            MouseEventKind::ScrollDown => {
                self.select(self.selected.saturating_add(1));
                None
            }
            MouseEventKind::ScrollUp => {
                self.select(self.selected.saturating_sub(1));
                None
            }
            _ => None,
        }
    }
}

pub fn render(frame: &mut Frame, browser: &mut Browser, lang: Lang) {
    let t = lang.t();
    let area = frame.area();
    let [header, body, footer] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(4),
        Constraint::Length(3),
    ])
    .areas(area);

    let title = Line::from(vec![
        Span::styled(
            " agentq ",
            Style::new()
                .fg(ON_ACCENT)
                .bg(ACCENT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!("  {}", t.br_title), Style::new().fg(FG)),
    ]);
    let location = Line::from(Span::styled(
        format!(" {}", browser.dir.display()),
        Style::new().fg(MUTED),
    ));
    let block = Block::new()
        .borders(Borders::BOTTOM)
        .border_style(Style::new().fg(BORDER));
    frame.render_widget(Paragraph::new(vec![title, location]).block(block), header);

    let items: Vec<ListItem> = browser
        .entries
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let on = index == browser.selected;
            let mut line = match entry.kind {
                EntryKind::Analyze => Line::from(vec![
                    Span::styled("● ", Style::new().fg(ACCENT)),
                    Span::styled(
                        t.br_analyze,
                        Style::new().fg(FG).add_modifier(Modifier::BOLD),
                    ),
                ]),
                EntryKind::Parent => Line::from(vec![
                    Span::styled("↑ ", Style::new().fg(MUTED)),
                    Span::styled("..", Style::new().fg(FG)),
                    Span::styled(format!("  {}", t.br_parent), Style::new().fg(MUTED)),
                ]),
                EntryKind::Dir => {
                    let mut spans = vec![Span::styled(
                        format!("{}/", entry.name),
                        Style::new().fg(FG),
                    )];
                    if let Some(hint) = entry.hint {
                        let label = match hint {
                            Hint::Node => t.hint_node,
                            Hint::Git => t.hint_git,
                        };
                        spans.push(Span::styled(format!("  {label}"), Style::new().fg(ACCENT)));
                    }
                    Line::from(spans)
                }
            };
            line.spans.insert(0, row_bar(on));
            if on {
                for span in line.spans.iter_mut().skip(1) {
                    if span.style.fg == Some(FG) {
                        span.style = span.style.fg(ACCENT);
                    }
                }
            }
            ListItem::new(line)
        })
        .collect();

    let folders = browser.entries.len().saturating_sub(2);
    let block = focused_panel(&fill(t.br_folders, &[("n", &folders)]));
    browser.list_area = block.inner(body);
    let list = List::new(items)
        .block(block)
        .highlight_symbol(selected_symbol());
    browser.list_state.select(Some(browser.selected));
    frame.render_stateful_widget(list, body, &mut browser.list_state);

    let status = match &browser.error {
        Some((path, error)) => Line::from(Span::styled(
            format!(
                " {}",
                fill(
                    t.cannot_open,
                    &[("path", &path.display()), ("error", error)]
                )
            ),
            Style::new().fg(ACCENT),
        )),
        None => Line::from(Span::styled(
            format!(" {}", t.br_tip),
            Style::new().fg(MUTED),
        )),
    };
    let keys = key_hints(&[
        ("↑↓", t.br_move),
        ("Enter", t.br_open),
        ("←", t.br_back),
        ("a", t.br_analyze_here),
        (".", t.br_hidden),
        ("~", t.br_home),
        ("L", t.key_language),
        ("q", t.br_menu),
    ]);
    let click = Line::from(Span::styled(
        format!(" {}", t.br_click),
        Style::new().fg(MUTED),
    ));
    frame.render_widget(Paragraph::new(vec![status, keys, click]), footer);
}

#[cfg(test)]
pub(super) mod fixture {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    /// A throwaway folder tree, removed when dropped:
    /// `alpha/` (a Node project), `Beta/` (a git repository), `gamma/`,
    /// a hidden `.hidden/` and a plain file.
    pub struct TempTree(pub PathBuf);

    impl TempTree {
        pub fn new() -> TempTree {
            let id = COUNTER.fetch_add(1, Ordering::SeqCst);
            let root =
                std::env::temp_dir().join(format!("agentq-browser-{}-{id}", std::process::id()));
            fs::create_dir_all(root.join("alpha")).unwrap();
            fs::write(root.join("alpha/package.json"), "{}").unwrap();
            fs::create_dir_all(root.join("Beta/.git")).unwrap();
            fs::create_dir_all(root.join("gamma")).unwrap();
            fs::create_dir_all(root.join(".hidden")).unwrap();
            fs::write(root.join("file.txt"), "not a folder").unwrap();
            TempTree(root)
        }
    }

    impl Drop for TempTree {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fixture::TempTree;
    use super::*;

    fn names(browser: &Browser) -> Vec<&str> {
        browser.entries.iter().map(|e| e.name.as_str()).collect()
    }

    fn browser() -> (TempTree, Browser) {
        let tree = TempTree::new();
        let browser = Browser::new(tree.0.clone());
        (tree, browser)
    }

    #[test]
    fn lists_folders_sorted_ignoring_case_with_hints() {
        let (_tree, browser) = browser();
        assert_eq!(
            names(&browser),
            ["Analyze this folder", "..", "alpha", "Beta", "gamma"]
        );
        assert_eq!(browser.entries[0].kind, EntryKind::Analyze);
        assert_eq!(browser.entries[1].kind, EntryKind::Parent);
        let hints: Vec<_> = browser.entries[2..].iter().map(|e| e.hint).collect();
        assert_eq!(hints, [Some(Hint::Node), Some(Hint::Git), None]);
    }

    #[test]
    fn hidden_folders_can_be_shown_without_losing_the_selection() {
        let (_tree, mut browser) = browser();
        browser.handle_key(KeyCode::Char('G'));
        assert_eq!(browser.entries[browser.selected].name, "gamma");
        browser.handle_key(KeyCode::Char('.'));
        assert!(names(&browser).contains(&".hidden"));
        assert_eq!(browser.entries[browser.selected].name, "gamma");
        browser.handle_key(KeyCode::Char('.'));
        assert!(!names(&browser).contains(&".hidden"));
    }

    #[test]
    fn enter_goes_into_a_folder_and_left_comes_back_to_it() {
        let (_tree, mut browser) = browser();
        let top = browser.dir.clone();
        browser.handle_key(KeyCode::Down);
        browser.handle_key(KeyCode::Down);
        assert_eq!(browser.entries[browser.selected].name, "alpha");
        assert_eq!(browser.handle_key(KeyCode::Enter), None);
        assert_eq!(browser.dir, top.join("alpha"));
        assert_eq!(names(&browser), ["Analyze this folder", ".."]);

        browser.handle_key(KeyCode::Backspace);
        assert_eq!(browser.dir, top);
        assert_eq!(browser.entries[browser.selected].name, "alpha");
    }

    #[test]
    fn the_parent_entry_goes_up() {
        let (_tree, mut browser) = browser();
        let top = browser.dir.clone();
        browser.handle_key(KeyCode::Down);
        browser.handle_key(KeyCode::Enter);
        assert_eq!(Some(browser.dir.as_path()), top.parent());
    }

    #[test]
    fn analyze_returns_the_folder_being_shown() {
        let (_tree, mut browser) = browser();
        let top = browser.dir.clone();
        assert_eq!(
            browser.handle_key(KeyCode::Enter),
            Some(Action::Analyze(top.clone()))
        );
        browser.handle_key(KeyCode::Down);
        browser.handle_key(KeyCode::Down);
        assert_eq!(
            browser.handle_key(KeyCode::Char('a')),
            Some(Action::Analyze(top))
        );
    }

    #[test]
    fn selection_stays_inside_the_list_and_q_quits() {
        let (_tree, mut browser) = browser();
        browser.handle_key(KeyCode::Up);
        assert_eq!(browser.selected, 0);
        for _ in 0..20 {
            browser.handle_key(KeyCode::Char('j'));
        }
        assert_eq!(browser.selected, browser.entries.len() - 1);
        browser.handle_key(KeyCode::Char('g'));
        assert_eq!(browser.selected, 0);
        assert_eq!(browser.handle_key(KeyCode::Char('q')), Some(Action::Back));
    }

    #[test]
    fn click_selects_and_a_second_click_opens() {
        let (_tree, mut browser) = browser();
        let top = browser.dir.clone();
        browser.list_area = Rect::new(2, 5, 40, 10);
        let click = |browser: &mut Browser, row: u16| {
            browser.handle_mouse(MouseEventKind::Down(MouseButton::Left), 5, row)
        };
        // Row 7 is the third entry: "alpha".
        assert_eq!(click(&mut browser, 7), None);
        assert_eq!(browser.selected, 2);
        assert_eq!(browser.dir, top);
        assert_eq!(click(&mut browser, 7), None);
        assert_eq!(browser.dir, top.join("alpha"));
        // Outside the list, and below the last entry, nothing happens.
        browser.list_area = Rect::new(2, 5, 40, 10);
        assert_eq!(click(&mut browser, 0), None);
        assert_eq!(click(&mut browser, 14), None);
    }

    #[test]
    fn clicking_the_analyze_row_twice_analyzes() {
        let (_tree, mut browser) = browser();
        let top = browser.dir.clone();
        browser.list_area = Rect::new(2, 5, 40, 10);
        browser.select(2);
        let click = |browser: &mut Browser| {
            browser.handle_mouse(MouseEventKind::Down(MouseButton::Left), 5, 5)
        };
        assert_eq!(click(&mut browser), None);
        assert_eq!(click(&mut browser), Some(Action::Analyze(top)));
    }

    #[test]
    fn the_wheel_moves_the_selection() {
        let (_tree, mut browser) = browser();
        browser.handle_mouse(MouseEventKind::ScrollDown, 0, 0);
        browser.handle_mouse(MouseEventKind::ScrollDown, 0, 0);
        assert_eq!(browser.selected, 2);
        browser.handle_mouse(MouseEventKind::ScrollUp, 0, 0);
        assert_eq!(browser.selected, 1);
    }

    #[test]
    fn an_unreadable_folder_keeps_the_current_one_and_says_why() {
        let (tree, mut browser) = browser();
        let top = browser.dir.clone();
        browser.open(tree.0.join("does-not-exist"));
        assert_eq!(browser.dir, top);
        let (path, _) = browser.error.clone().expect("an error is recorded");
        assert!(path.ends_with("does-not-exist"));
        assert_eq!(names(&browser).len(), 5);
        browser.handle_key(KeyCode::Down);
        assert!(browser.error.is_none());
    }
}
