//! The main menu: where the interface starts when no project is given, and
//! where every other screen can come back to.

use super::logo::{ART_ROWS, Logo, Spin};
use super::ui::{ACCENT, DIM, FG, MUTED, ON_ACCENT, focused_panel, key_hints, row_bar};
use crate::i18n::Lang;
use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, MouseButton, MouseEventKind};
use ratatui::layout::{Constraint, Flex, Layout, Position, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Item {
    Browse,
    Here,
    Settings,
    Quit,
}

const ITEMS: [Item; 4] = [Item::Browse, Item::Here, Item::Settings, Item::Quit];

/// What the menu asks the application to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Browse,
    AnalyzeHere,
    Settings,
    SwitchLanguage,
    Quit,
}

#[derive(Default)]
pub struct Menu {
    /// Which picture is drawn at the top; see the `logo` module.
    pub logo: Logo,
    pub selected: usize,
    /// The rows of the items, filled in by the renderer.
    pub list_area: Rect,
    /// Where the logo was drawn, filled in by the renderer.
    pub logo_area: Rect,
    /// The last place the pointer was seen.
    pub pointer: Option<Position>,
    /// How far the animation has gone; moves on every tick.
    pub angle: f32,
    /// How much the pointer lights the picture (0 to 1); eases toward its target.
    pub glow: f32,
    /// Which way the pointer is from the picture, kept while the glow fades.
    pub aim: (f32, f32),
    /// The user's hand on a 3D logo: how far it was turned and dragged.
    pub spin: Spin,
}

fn near(area: Rect, p: Position) -> bool {
    Rect::new(
        area.x.saturating_sub(10),
        area.y.saturating_sub(3),
        area.width + 20,
        area.height + 6,
    )
    .contains(p)
}

impl Menu {
    pub fn new(logo: Logo) -> Menu {
        Menu {
            logo,
            ..Menu::default()
        }
    }

    /// One step of the animation. The glow and the light ease toward where the
    /// pointer is, so entering and leaving are gradual.
    pub fn tick(&mut self) {
        let area = self.logo_area;
        let hovering = self.pointer.filter(|p| near(area, *p));
        if let Some(p) = hovering {
            let half = (f32::from(area.width) / 2.0, f32::from(area.height) / 2.0);
            let target = (
                ((f32::from(p.x) - f32::from(area.x) - half.0) / half.0).clamp(-1.0, 1.0),
                ((f32::from(p.y) - f32::from(area.y) - half.1) / half.1).clamp(-1.0, 1.0),
            );
            self.aim.0 += (target.0 - self.aim.0) * 0.12;
            self.aim.1 += (target.1 - self.aim.1) * 0.12;
        }
        let goal = if hovering.is_some() { 1.0 } else { 0.0 };
        self.glow += (goal - self.glow) * 0.05;
        if (goal - self.glow).abs() < 0.01 {
            self.glow = goal;
        }
        if self.spin.held() {
            // Held in the hand: it only follows the pointer.
            return;
        }
        self.angle = (self.angle + 0.042 + 0.058 * self.glow) % std::f32::consts::TAU;
        self.spin.coast();
    }

    fn select(&mut self, index: usize) {
        self.selected = index.min(ITEMS.len() - 1);
    }

    fn activate(&self) -> Action {
        match ITEMS[self.selected] {
            Item::Browse => Action::Browse,
            Item::Here => Action::AnalyzeHere,
            Item::Settings => Action::Settings,
            Item::Quit => Action::Quit,
        }
    }

    pub fn handle_key(&mut self, key: KeyCode) -> Option<Action> {
        match key {
            KeyCode::Char('q') | KeyCode::Esc => return Some(Action::Quit),
            KeyCode::Down | KeyCode::Char('j') => self.select(self.selected + 1),
            KeyCode::Up | KeyCode::Char('k') => self.select(self.selected.saturating_sub(1)),
            KeyCode::Home | KeyCode::Char('g') => self.select(0),
            KeyCode::End | KeyCode::Char('G') => self.select(usize::MAX),
            KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => return Some(self.activate()),
            KeyCode::Char('b') => return Some(Action::Browse),
            KeyCode::Char('a') => return Some(Action::AnalyzeHere),
            KeyCode::Char('s') => return Some(Action::Settings),
            KeyCode::Char('L') => return Some(Action::SwitchLanguage),
            _ => {}
        }
        None
    }

    /// Click an item to select it and click it again to run it.
    pub fn handle_mouse(&mut self, kind: MouseEventKind, column: u16, row: u16) -> Option<Action> {
        let at = Position::new(column, row);
        if self.logo.is_3d() && self.spin.handle_mouse(kind, at, self.logo_area) {
            self.pointer = Some(at);
            return None;
        }
        match kind {
            MouseEventKind::Down(MouseButton::Left) if self.list_area.contains(at) => {
                let index = usize::from(row - self.list_area.y);
                if index >= ITEMS.len() {
                    return None;
                }
                if index == self.selected {
                    return Some(self.activate());
                }
                self.select(index);
                None
            }
            MouseEventKind::Moved => {
                self.pointer = Some(at);
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

/// Keeps the end of `text` when it does not fit `width` columns.
fn shorten(text: &str, width: usize) -> String {
    let count = text.chars().count();
    if count <= width {
        return text.to_string();
    }
    let tail: String = text.chars().skip(count - width.saturating_sub(1)).collect();
    format!("…{tail}")
}

pub fn render(frame: &mut Frame, menu: &mut Menu, lang: Lang, here: &str) {
    let t = lang.t();
    let area = frame.area();
    let [column] = Layout::horizontal([Constraint::Max(64)])
        .flex(Flex::Center)
        .areas(area);
    let [column] = Layout::vertical([Constraint::Max(23)])
        .flex(Flex::Center)
        .areas(column);
    let [head, list, hints] = Layout::vertical([
        Constraint::Length(13),
        Constraint::Length(ITEMS.len() as u16 + 2),
        Constraint::Length(2),
    ])
    .areas(column);

    menu.logo_area =
        Rect::new(head.x + 1, head.y, head.width.saturating_sub(2), ART_ROWS).intersection(head);
    let mut lines: Vec<Line> = menu.logo.lines(
        menu.logo_area,
        menu.glow,
        menu.aim,
        menu.angle,
        menu.spin.turn,
    );
    lines.push(Line::raw(""));
    lines.push(Line::from(Span::styled(
        " agentq ",
        Style::new()
            .fg(ON_ACCENT)
            .bg(ACCENT)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::styled(
        t.menu_tagline,
        Style::new().fg(MUTED),
    )));
    let lines: Vec<Line> = lines
        .into_iter()
        .map(|mut line| {
            line.spans.insert(0, Span::raw(" "));
            line
        })
        .collect();
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), head);

    // Room left for the detail: border, padding, bar and the label column.
    let room = usize::from(list.width).saturating_sub(2 + 2 + 2 + 28);
    let rows: Vec<Line> = ITEMS
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let on = index == menu.selected;
            let (label, detail) = match item {
                Item::Browse => (t.menu_browse.to_string(), t.menu_browse_hint.to_string()),
                Item::Here => (t.menu_here.to_string(), shorten(here, room)),
                Item::Settings => (
                    t.menu_settings.to_string(),
                    t.menu_settings_hint.to_string(),
                ),
                Item::Quit => (t.menu_quit.to_string(), String::new()),
            };
            Line::from(vec![
                row_bar(on),
                Span::styled(
                    format!("{label:<28}"),
                    Style::new()
                        .fg(if on { ACCENT } else { FG })
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(detail, Style::new().fg(if on { MUTED } else { DIM })),
            ])
        })
        .collect();
    let block = focused_panel("menu");
    menu.list_area = block.inner(list);
    frame.render_widget(Paragraph::new(rows).block(block), list);

    frame.render_widget(
        Paragraph::new(vec![
            Line::raw(""),
            key_hints(&[
                ("↑↓", t.menu_select),
                ("Enter", t.menu_choose),
                ("shift+l", t.key_language),
                ("q", t.key_quit),
            ]),
        ]),
        hints,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(lines: &[Line]) -> String {
        lines
            .iter()
            .map(|l| {
                l.spans
                    .iter()
                    .map(|s| s.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn every_logo_has_the_size_it_says() {
        for logo in Logo::ALL {
            let lines = logo.lines(
                Rect::new(0, 0, 62, ART_ROWS),
                0.0,
                (0.0, 0.0),
                0.0,
                (0.0, 0.0),
            );
            assert_eq!(lines.len(), usize::from(ART_ROWS), "{}", logo.name());
            assert!(lines.iter().all(|l| l.spans.len() == 62), "{}", logo.name());
        }
    }

    #[test]
    fn every_logo_is_drawn_and_moves() {
        let area = Rect::new(0, 0, 62, ART_ROWS);
        for logo in Logo::ALL {
            let a = text(&logo.lines(area, 0.0, (0.0, 0.0), 0.0, (0.0, 0.0)));
            let b = text(&logo.lines(area, 0.0, (0.0, 0.0), 1.3, (0.0, 0.0)));
            assert!(
                a.chars().any(|c| ('\u{2801}'..='\u{28ff}').contains(&c)),
                "{}",
                logo.name()
            );
            assert_ne!(a, b, "{}", logo.name());
        }
    }

    #[test]
    fn the_planet_and_its_electron_are_drawn() {
        let picture = text(&Logo::Planet.lines(
            Rect::new(0, 0, 62, ART_ROWS),
            0.0,
            (0.0, 0.0),
            0.0,
            (0.0, 0.0),
        ));
        assert!(
            picture
                .chars()
                .any(|c| ('\u{2801}'..='\u{28ff}').contains(&c)),
            "{picture}"
        );
    }

    #[test]
    fn the_electron_moves_with_the_ticks() {
        let area = Rect::new(5, 2, 62, ART_ROWS);
        let mut menu = Menu::new(Logo::Planet);
        menu.logo_area = area;
        menu.tick();
        assert!(menu.angle > 0.0);
        assert_ne!(
            text(&Logo::Planet.lines(area, 0.0, (0.0, 0.0), 0.0, (0.0, 0.0))),
            text(&Logo::Planet.lines(area, 0.0, (0.0, 0.0), 1.0, (0.0, 0.0)))
        );
    }

    #[test]
    fn the_glow_rises_under_the_pointer_and_fades_slowly_after() {
        let mut menu = Menu::new(Logo::Planet);
        menu.logo_area = Rect::new(5, 2, 62, ART_ROWS);
        menu.pointer = Some(Position::new(20, 5));
        for _ in 0..80 {
            menu.tick();
        }
        assert!(menu.glow > 0.95, "{}", menu.glow);
        menu.pointer = Some(Position::new(0, 60));
        menu.tick();
        // One step later it has not dropped to nothing.
        assert!(menu.glow > 0.8 && menu.glow < 1.0, "{}", menu.glow);
        for _ in 0..150 {
            menu.tick();
        }
        assert_eq!(menu.glow, 0.0);
    }

    #[test]
    fn the_orbit_is_orange_only_with_a_glow() {
        let area = Rect::new(5, 2, 62, ART_ROWS);
        let accent = |glow| {
            Logo::Planet
                .lines(area, glow, (0.0, 0.0), 0.0, (0.0, 0.0))
                .iter()
                .flat_map(|l| l.spans.iter().map(|s| s.style.fg))
                .filter(|c| *c == Some(ACCENT))
                .count()
        };
        assert_eq!(accent(0.0), 0);
        assert!(accent(1.0) > 0);
    }

    #[test]
    fn dragging_the_logo_turns_it_and_it_keeps_going_a_little() {
        let mut menu = Menu::new(Logo::Lock);
        menu.logo_area = Rect::new(5, 2, 62, ART_ROWS);
        let down = MouseEventKind::Down(MouseButton::Left);
        let drag = MouseEventKind::Drag(MouseButton::Left);
        menu.handle_mouse(down, 30, 5);
        menu.handle_mouse(drag, 40, 7);
        assert!(
            menu.spin.turn.0 > 1.0 && menu.spin.turn.1 > 0.3,
            "{:?}",
            menu.spin.turn
        );
        // Held, the logo does not move by itself.
        let held = (menu.angle, menu.spin.turn);
        menu.tick();
        assert_eq!((menu.angle, menu.spin.turn), held);
        menu.handle_mouse(MouseEventKind::Up(MouseButton::Left), 40, 7);
        menu.tick();
        assert!(
            menu.spin.turn.0 > held.1.0,
            "it carries on after being let go"
        );
        for _ in 0..200 {
            menu.tick();
        }
        assert!(menu.spin.momentum.0.abs() < 0.001);
    }

    #[test]
    fn the_flat_logo_cannot_be_dragged() {
        let mut menu = Menu::new(Logo::Planet);
        menu.logo_area = Rect::new(5, 2, 62, ART_ROWS);
        menu.handle_mouse(MouseEventKind::Down(MouseButton::Left), 30, 5);
        assert!(menu.spin.drag.is_none());
    }

    #[test]
    fn turning_by_hand_changes_the_picture() {
        let area = Rect::new(0, 0, 62, ART_ROWS);
        assert_ne!(
            text(&Logo::Key.lines(area, 0.0, (0.0, 0.0), 0.0, (0.0, 0.0))),
            text(&Logo::Key.lines(area, 0.0, (0.0, 0.0), 0.0, (1.0, 0.6)))
        );
    }
}
