//! The scheduled scans screen: the plans in `~/.agentq/schedule.toml`, shared
//! with the desktop interface, and a small form to add one. The user types the
//! time of day the scan should run instead of picking a delay.

use super::ui::{ACCENT, DIM, FG, MUTED, focused_panel, key_hints, row_bar};
use crate::i18n::Lang;
use crate::recents::project_name;
use crate::schedule::{Repeat, Schedules, next_at, parse_time, until};
use chrono::{Duration, Local, TimeZone, Timelike};
use ratatui::Frame;
use ratatui::crossterm::event::KeyCode;
use ratatui::layout::{Constraint, Flex, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph, Wrap};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// What the screen asks the application to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Back,
    /// Plan a scan of `path` at `at` (seconds since the Unix epoch).
    Add {
        path: PathBuf,
        at: u64,
        repeat: Repeat,
    },
    /// Drop the plan with this id.
    Cancel(u64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Field {
    Project,
    Time,
    Repeat,
}

const FIELDS: [Field; 3] = [Field::Project, Field::Time, Field::Repeat];
const REPEATS: [Repeat; 3] = [Repeat::Once, Repeat::Daily, Repeat::Weekly];

/// The "new scan" form while it is open.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Form {
    project: String,
    time: String,
    repeat: Repeat,
    field: usize,
    /// Which message to show about the last attempt, if it failed.
    error: Option<Problem>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Problem {
    Time,
    Path,
}

#[derive(Default)]
pub struct Planner {
    pub selected: usize,
    form: Option<Form>,
    /// The last thing that happened, shown under the list.
    pub notice: Option<String>,
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// The next full hour as "HH:00", which the user then edits.
fn suggested_time() -> String {
    let next = Local::now() + Duration::hours(1);
    format!("{:02}:00", next.hour())
}

/// "09:30" for the local clock at `at`.
fn clock(at: u64) -> String {
    i64::try_from(at)
        .ok()
        .and_then(|at| Local.timestamp_opt(at, 0).single())
        .map_or_else(
            || "--:--".to_string(),
            |t| format!("{:02}:{:02}", t.hour(), t.minute()),
        )
}

impl Planner {
    pub fn new() -> Planner {
        Planner::default()
    }

    /// Whether the form is up, which changes what the keys mean.
    pub fn is_editing(&self) -> bool {
        self.form.is_some()
    }

    fn open_form(&mut self, here: &str) {
        self.notice = None;
        self.form = Some(Form {
            project: here.to_string(),
            time: suggested_time(),
            repeat: Repeat::Once,
            field: 1,
            error: None,
        });
    }

    pub fn handle_key(
        &mut self,
        key: KeyCode,
        schedules: &Schedules,
        here: &str,
    ) -> Option<Action> {
        if self.form.is_some() {
            return self.handle_form_key(key);
        }
        let last = schedules.items.len().saturating_sub(1);
        match key {
            KeyCode::Esc | KeyCode::Char('q' | 'b' | 'm') => return Some(Action::Back),
            KeyCode::Down | KeyCode::Char('j') => self.selected = (self.selected + 1).min(last),
            KeyCode::Up | KeyCode::Char('k') => self.selected = self.selected.saturating_sub(1),
            KeyCode::Char('n' | 'a') => self.open_form(here),
            KeyCode::Char('d' | 'x') | KeyCode::Delete | KeyCode::Backspace => {
                let id = schedules.items.get(self.selected).map(|s| s.id)?;
                self.selected = self.selected.min(last.saturating_sub(1));
                return Some(Action::Cancel(id));
            }
            _ => {}
        }
        None
    }

    fn handle_form_key(&mut self, key: KeyCode) -> Option<Action> {
        let form = self.form.as_mut()?;
        form.error = None;
        let field = FIELDS[form.field];
        match key {
            KeyCode::Esc => self.form = None,
            KeyCode::Tab | KeyCode::Down => form.field = (form.field + 1) % FIELDS.len(),
            KeyCode::BackTab | KeyCode::Up => {
                form.field = (form.field + FIELDS.len() - 1) % FIELDS.len();
            }
            KeyCode::Enter => return self.submit(),
            KeyCode::Left | KeyCode::Right if field == Field::Repeat => {
                let at = REPEATS.iter().position(|r| *r == form.repeat).unwrap_or(0);
                let step = if key == KeyCode::Right {
                    1
                } else {
                    REPEATS.len() - 1
                };
                form.repeat = REPEATS[(at + step) % REPEATS.len()];
            }
            KeyCode::Char(' ') if field == Field::Repeat => {
                let at = REPEATS.iter().position(|r| *r == form.repeat).unwrap_or(0);
                form.repeat = REPEATS[(at + 1) % REPEATS.len()];
            }
            KeyCode::Backspace => match field {
                Field::Project => {
                    form.project.pop();
                }
                Field::Time => {
                    form.time.pop();
                }
                Field::Repeat => {}
            },
            KeyCode::Char(c) => match field {
                Field::Project => form.project.push(c),
                Field::Time if (c.is_ascii_digit() || c == ':') && form.time.len() < 5 => {
                    form.time.push(c);
                }
                _ => {}
            },
            _ => {}
        }
        None
    }

    /// Checks the form and turns it into a plan.
    fn submit(&mut self) -> Option<Action> {
        let form = self.form.as_mut()?;
        let Some((hour, minute)) = parse_time(&form.time) else {
            form.error = Some(Problem::Time);
            form.field = 1;
            return None;
        };
        let Ok(path) = std::fs::canonicalize(form.project.trim()).map(|p| (p.is_dir(), p)) else {
            form.error = Some(Problem::Path);
            form.field = 0;
            return None;
        };
        if !path.0 {
            form.error = Some(Problem::Path);
            form.field = 0;
            return None;
        }
        let at = next_at(hour, minute, now())?;
        let repeat = form.repeat;
        self.form = None;
        Some(Action::Add {
            path: path.1,
            at,
            repeat,
        })
    }
}

fn repeat_label(lang: Lang, repeat: Repeat) -> &'static str {
    let t = lang.t();
    match repeat {
        Repeat::Once => t.sched_once,
        Repeat::Daily => t.sched_daily,
        Repeat::Weekly => t.sched_weekly,
    }
}

/// "09:30" in the form of a message for the status line.
pub fn added_message(lang: Lang, at: u64) -> String {
    crate::i18n::fill(lang.t().sched_added, &[("time", &clock(at))])
}

pub fn render(frame: &mut Frame, planner: &Planner, schedules: &Schedules, lang: Lang) {
    let t = lang.t();
    let [column] = Layout::horizontal([Constraint::Max(78)])
        .flex(Flex::Center)
        .areas(frame.area());
    let form_rows = if planner.form.is_some() { 9 } else { 0 };
    let [list, form, note, hints] = Layout::vertical([
        Constraint::Min(5),
        Constraint::Length(form_rows),
        Constraint::Length(2),
        Constraint::Length(2),
    ])
    .areas(column);

    let now = now();
    let width = usize::from(list.width).saturating_sub(4);
    let rows: Vec<Line> = if schedules.items.is_empty() {
        vec![Line::from(Span::styled(
            format!(" {}", t.sched_empty),
            Style::new().fg(MUTED),
        ))]
    } else {
        schedules
            .items
            .iter()
            .enumerate()
            .map(|(index, item)| {
                let on = index == planner.selected;
                let when = format!(
                    "{}  {:<8} {:<7}",
                    clock(item.at),
                    repeat_label(lang, item.repeat),
                    until(item.at, now)
                );
                let name = project_name(&item.path);
                let room = width.saturating_sub(when.chars().count() + 4);
                let detail: String = item.path.display().to_string();
                let detail = if detail.chars().count() > room {
                    let tail: String = detail
                        .chars()
                        .skip(detail.chars().count() - room.saturating_sub(1))
                        .collect();
                    format!("…{tail}")
                } else {
                    detail
                };
                Line::from(vec![
                    row_bar(on),
                    Span::styled(
                        format!("{when}  "),
                        Style::new()
                            .fg(if on { ACCENT } else { FG })
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("{name} "),
                        Style::new().fg(FG).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(detail, Style::new().fg(DIM)),
                ])
            })
            .collect()
    };
    frame.render_widget(
        Paragraph::new(rows).block(focused_panel(t.sched_title)),
        list,
    );

    if let Some(f) = &planner.form {
        frame.render_widget(Clear, form);
        let field_line = |index: usize, label: &str, value: String| {
            let on = f.field == index;
            let shown = if on { format!("{value}▏") } else { value };
            Line::from(vec![
                row_bar(on),
                Span::styled(
                    format!("{label:<16}"),
                    Style::new()
                        .fg(if on { ACCENT } else { FG })
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(shown, Style::new().fg(if on { FG } else { MUTED })),
            ])
        };
        let repeat = format!("‹ {} ›", repeat_label(lang, f.repeat));
        let mut lines = vec![
            field_line(0, t.sched_project, f.project.clone()),
            field_line(1, t.sched_time, f.time.clone()),
            field_line(2, t.sched_repeat, repeat),
        ];
        if let Some(problem) = f.error {
            let text = match problem {
                Problem::Time => t.sched_bad_time,
                Problem::Path => t.sched_bad_path,
            };
            lines.push(Line::from(Span::styled(
                format!(" {text}"),
                Style::new().fg(ratatui::style::Color::Red),
            )));
        }
        frame.render_widget(
            Paragraph::new(lines)
                .wrap(Wrap { trim: false })
                .block(focused_panel(t.sched_new_title)),
            form,
        );
    }

    if let Some(text) = &planner.notice {
        frame.render_widget(
            Paragraph::new(vec![
                Line::raw(""),
                Line::from(Span::styled(format!(" {text}"), Style::new().fg(MUTED))),
            ]),
            note,
        );
    }

    let keys: Vec<(&str, &str)> = if planner.form.is_some() {
        vec![
            ("Tab", t.key_next_field),
            ("Enter", t.key_save),
            ("Esc", t.key_cancel),
        ]
    } else {
        vec![
            ("↑↓", t.menu_select),
            ("n", t.key_new),
            ("d", t.key_delete),
            ("Esc", t.key_back),
        ]
    };
    frame.render_widget(Paragraph::new(vec![Line::raw(""), key_hints(&keys)]), hints);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schedules(n: usize) -> Schedules {
        let mut s = Schedules::default();
        for i in 0..n {
            s.add(std::path::Path::new("/tmp"), 100 + i as u64, Repeat::Once);
        }
        s
    }

    fn type_text(planner: &mut Planner, text: &str) {
        for c in text.chars() {
            planner.handle_key(KeyCode::Char(c), &Schedules::default(), "/tmp");
        }
    }

    #[test]
    fn keys_move_through_the_plans_and_stop_at_the_ends() {
        let s = schedules(3);
        let mut planner = Planner::new();
        planner.handle_key(KeyCode::Down, &s, "/tmp");
        planner.handle_key(KeyCode::Down, &s, "/tmp");
        planner.handle_key(KeyCode::Down, &s, "/tmp");
        assert_eq!(planner.selected, 2);
        planner.handle_key(KeyCode::Up, &s, "/tmp");
        assert_eq!(planner.selected, 1);
        assert_eq!(
            planner.handle_key(KeyCode::Esc, &s, "/tmp"),
            Some(Action::Back)
        );
    }

    #[test]
    fn d_drops_the_selected_plan() {
        let s = schedules(2);
        let mut planner = Planner::new();
        planner.handle_key(KeyCode::Down, &s, "/tmp");
        let id = s.items[1].id;
        assert_eq!(
            planner.handle_key(KeyCode::Char('d'), &s, "/tmp"),
            Some(Action::Cancel(id))
        );
        // Nothing to drop in an empty list.
        assert_eq!(
            Planner::new().handle_key(KeyCode::Char('d'), &Schedules::default(), "/tmp"),
            None
        );
    }

    #[test]
    fn the_time_is_typed_and_only_takes_digits_and_a_colon() {
        let mut planner = Planner::new();
        planner.handle_key(KeyCode::Char('n'), &Schedules::default(), "/tmp");
        assert!(planner.is_editing());
        for _ in 0..5 {
            planner.handle_key(KeyCode::Backspace, &Schedules::default(), "/tmp");
        }
        type_text(&mut planner, "1a4:3x099");
        assert_eq!(planner.form.as_ref().unwrap().time, "14:30");
    }

    #[test]
    fn a_bad_time_keeps_the_form_open_and_says_why() {
        let mut planner = Planner::new();
        planner.handle_key(KeyCode::Char('n'), &Schedules::default(), "/tmp");
        for _ in 0..5 {
            planner.handle_key(KeyCode::Backspace, &Schedules::default(), "/tmp");
        }
        type_text(&mut planner, "25:00");
        assert_eq!(
            planner.handle_key(KeyCode::Enter, &Schedules::default(), "/tmp"),
            None
        );
        assert_eq!(planner.form.as_ref().unwrap().error, Some(Problem::Time));
    }

    #[test]
    fn a_valid_form_becomes_a_plan_at_the_next_such_time() {
        let mut planner = Planner::new();
        planner.handle_key(KeyCode::Char('n'), &Schedules::default(), "/tmp");
        for _ in 0..5 {
            planner.handle_key(KeyCode::Backspace, &Schedules::default(), "/tmp");
        }
        type_text(&mut planner, "07:45");
        planner.handle_key(KeyCode::Tab, &Schedules::default(), "/tmp");
        planner.handle_key(KeyCode::Right, &Schedules::default(), "/tmp");
        let Some(Action::Add { path, at, repeat }) =
            planner.handle_key(KeyCode::Enter, &Schedules::default(), "/tmp")
        else {
            panic!("expected a plan");
        };
        assert!(path.is_dir());
        assert_eq!(repeat, Repeat::Daily);
        assert_eq!(clock(at), "07:45");
        assert!(at > now() && at - now() <= 86_400);
        assert!(!planner.is_editing());
    }

    #[test]
    fn a_missing_folder_is_refused() {
        let mut planner = Planner::new();
        planner.handle_key(KeyCode::Char('n'), &Schedules::default(), "/no/such/folder");
        assert_eq!(
            planner.handle_key(KeyCode::Enter, &Schedules::default(), "/x"),
            None
        );
        assert_eq!(planner.form.as_ref().unwrap().error, Some(Problem::Path));
    }
}
