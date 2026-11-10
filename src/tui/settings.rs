//! The settings screen: the language and the logo, with the logo turning above
//! so the choice can be seen (and turned by hand) before leaving.

use super::logo::{ART_ROWS, Logo, Spin};
use super::ui::{ACCENT, DIM, FG, MUTED, focused_panel, key_hints, row_bar};
use crate::config::Settings;
use crate::i18n::Lang;
use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, MouseButton, MouseEventKind};
use ratatui::layout::{Constraint, Flex, Layout, Position, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Row {
    Language,
    Logo,
}

const ROWS: [Row; 2] = [Row::Language, Row::Logo];

/// What the screen asks the application to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Back,
    /// Switch to the other language.
    Language,
    /// Pick the next logo, or the previous one with `back`.
    Logo {
        back: bool,
    },
}

#[derive(Default)]
pub struct SettingsScreen {
    pub selected: usize,
    /// How far the animation of the logo has gone.
    pub angle: f32,
    pub spin: Spin,
    /// The rows of the settings, filled in by the renderer.
    pub list_area: Rect,
    /// Where the logo was drawn, filled in by the renderer.
    pub logo_area: Rect,
}

impl SettingsScreen {
    pub fn new() -> SettingsScreen {
        SettingsScreen::default()
    }

    pub fn tick(&mut self) {
        if self.spin.held() {
            return;
        }
        self.angle = (self.angle + 0.042) % std::f32::consts::TAU;
        self.spin.coast();
    }

    fn select(&mut self, index: usize) {
        self.selected = index.min(ROWS.len() - 1);
    }

    fn change(&self, back: bool) -> Action {
        match ROWS[self.selected] {
            Row::Language => Action::Language,
            Row::Logo => Action::Logo { back },
        }
    }

    pub fn handle_key(&mut self, key: KeyCode) -> Option<Action> {
        match key {
            KeyCode::Esc | KeyCode::Char('q' | 'b' | 'm') => return Some(Action::Back),
            KeyCode::Down | KeyCode::Char('j') => self.select(self.selected + 1),
            KeyCode::Up | KeyCode::Char('k') => self.select(self.selected.saturating_sub(1)),
            KeyCode::Enter | KeyCode::Right | KeyCode::Char('l' | ' ') | KeyCode::Tab => {
                return Some(self.change(false));
            }
            KeyCode::Left | KeyCode::Char('h') | KeyCode::BackTab => {
                return Some(self.change(true));
            }
            _ => {}
        }
        None
    }

    /// Click a row to select it and click it again to change it; drag the
    /// logo to turn it.
    pub fn handle_mouse(
        &mut self,
        kind: MouseEventKind,
        column: u16,
        row: u16,
        logo: Logo,
    ) -> Option<Action> {
        let at = Position::new(column, row);
        if logo.is_3d() && self.spin.handle_mouse(kind, at, self.logo_area) {
            return None;
        }
        match kind {
            MouseEventKind::Down(MouseButton::Left) if self.list_area.contains(at) => {
                let index = usize::from(row - self.list_area.y);
                if index >= ROWS.len() {
                    return None;
                }
                if index == self.selected {
                    return Some(self.change(false));
                }
                self.select(index);
                None
            }
            MouseEventKind::ScrollDown => {
                self.select(self.selected + 1);
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

fn language_name(lang: Lang) -> &'static str {
    match lang {
        Lang::En => "English",
        Lang::PtBr => "Português",
    }
}

pub fn render(frame: &mut Frame, screen: &mut SettingsScreen, lang: Lang, logo: Logo) {
    let t = lang.t();
    let [column] = Layout::horizontal([Constraint::Max(64)])
        .flex(Flex::Center)
        .areas(frame.area());
    let [column] = Layout::vertical([Constraint::Max(
        ART_ROWS + 1 + ROWS.len() as u16 + 2 + 2 + 2,
    )])
    .flex(Flex::Center)
    .areas(column);
    let [head, list, note, hints] = Layout::vertical([
        Constraint::Length(ART_ROWS + 1),
        Constraint::Length(ROWS.len() as u16 + 2),
        Constraint::Length(2),
        Constraint::Length(2),
    ])
    .areas(column);

    screen.logo_area =
        Rect::new(head.x + 1, head.y, head.width.saturating_sub(2), ART_ROWS).intersection(head);
    let picture = logo.lines(
        screen.logo_area,
        0.0,
        (0.0, 0.0),
        screen.angle,
        screen.spin.turn,
    );
    let picture: Vec<Line> = picture
        .into_iter()
        .map(|mut line| {
            line.spans.insert(0, Span::raw(" "));
            line
        })
        .collect();
    frame.render_widget(Paragraph::new(picture), head);

    let rows: Vec<Line> = ROWS
        .iter()
        .enumerate()
        .map(|(index, row)| {
            let on = index == screen.selected;
            let (label, value) = match row {
                Row::Language => (t.settings_language, language_name(lang)),
                Row::Logo => (t.settings_logo, logo.label(lang)),
            };
            let value = if on {
                format!("‹ {value} ›")
            } else {
                value.to_string()
            };
            Line::from(vec![
                row_bar(on),
                Span::styled(
                    format!("{label:<14}"),
                    Style::new()
                        .fg(if on { ACCENT } else { FG })
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(value, Style::new().fg(if on { FG } else { MUTED })),
            ])
        })
        .collect();
    let block = focused_panel(t.settings_title);
    screen.list_area = block.inner(list);
    frame.render_widget(Paragraph::new(rows).block(block), list);

    let mut notes = vec![Line::raw("")];
    let saved = Settings::path().map(|path| format!("{} {}", t.settings_saved_in, path.display()));
    let drag = logo.is_3d().then(|| t.settings_drag.to_string());
    let text = [drag, saved]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" • ");
    notes.push(Line::from(Span::styled(
        format!(" {text}"),
        Style::new().fg(DIM),
    )));
    frame.render_widget(Paragraph::new(notes), note);

    frame.render_widget(
        Paragraph::new(vec![
            Line::raw(""),
            key_hints(&[
                ("↑↓", t.menu_select),
                ("←→", t.key_change),
                ("Esc", t.key_back),
                ("q", t.key_quit),
            ]),
        ]),
        hints,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_move_and_change_the_rows() {
        let mut screen = SettingsScreen::new();
        assert_eq!(screen.handle_key(KeyCode::Enter), Some(Action::Language));
        screen.handle_key(KeyCode::Down);
        assert_eq!(
            screen.handle_key(KeyCode::Right),
            Some(Action::Logo { back: false })
        );
        assert_eq!(
            screen.handle_key(KeyCode::Left),
            Some(Action::Logo { back: true })
        );
        // It stops at the last row.
        screen.handle_key(KeyCode::Down);
        assert_eq!(screen.selected, ROWS.len() - 1);
        assert_eq!(screen.handle_key(KeyCode::Esc), Some(Action::Back));
    }

    #[test]
    fn the_logo_can_be_dragged_here_too() {
        let mut screen = SettingsScreen::new();
        screen.logo_area = Rect::new(5, 2, 62, ART_ROWS);
        screen.handle_mouse(MouseEventKind::Down(MouseButton::Left), 30, 5, Logo::Lock);
        screen.handle_mouse(MouseEventKind::Drag(MouseButton::Left), 35, 5, Logo::Lock);
        assert!(screen.spin.turn.0 > 0.4);
    }
}
