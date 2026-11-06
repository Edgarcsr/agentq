//! The main menu: where the interface starts when no project is given, and
//! where every other screen can come back to.

use super::ui::{ACCENT, DIM, FG, MUTED, ON_ACCENT, focused_panel, key_hints, row_bar};
use crate::i18n::Lang;
use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, MouseButton, MouseEventKind};
use ratatui::layout::{Constraint, Flex, Layout, Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Item {
    Browse,
    Here,
    Language,
    Quit,
}

const ITEMS: [Item; 4] = [Item::Browse, Item::Here, Item::Language, Item::Quit];

/// What the menu asks the application to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Browse,
    AnalyzeHere,
    SwitchLanguage,
    Quit,
}

#[derive(Default)]
pub struct Menu {
    pub selected: usize,
    /// The rows of the items, filled in by the renderer.
    pub list_area: Rect,
    /// Where the logo was drawn, filled in by the renderer.
    pub logo_area: Rect,
    /// The last place the pointer was seen.
    pub pointer: Option<Position>,
    /// Where the electron is on its orbit; moves on every tick.
    pub angle: f32,
    /// How much the pointer lights the picture (0 to 1); eases toward its target.
    pub glow: f32,
    /// Which way the pointer is from the planet, kept while the glow fades.
    pub aim: (f32, f32),
}

/// Height of the picture in rows. It is drawn with braille characters, which
/// hold a grid of 2 by 4 dots each, so curves come out smooth.
const ART_ROWS: u16 = 9;

/// One orbit: semi-axes and tilt, in dots.
struct Orbit {
    a: f32,
    b: f32,
    tilt: f32,
}

const ORBITS: [Orbit; 1] = [Orbit {
    a: 56.0,
    b: 9.0,
    tilt: -0.2,
}];

impl Orbit {
    /// The point at angle `t` relative to the planet's center, and whether it
    /// passes in front of the planet.
    fn at(&self, t: f32) -> (f32, f32, bool) {
        let (x, y) = (self.a * t.cos(), self.b * t.sin());
        let (sin, cos) = self.tilt.sin_cos();
        (x * cos - y * sin, x * sin + y * cos, t.sin() > 0.0)
    }
}

fn mix(a: usize, b: usize) -> usize {
    let mut h = (a as u32).wrapping_mul(0x9E37_79B1) ^ (b as u32).wrapping_mul(0x85EB_CA6B);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    h as usize
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

/// Mixes two colors; `amount` 0 is `from`, 1 is `to`.
fn blend(from: Color, to: Color, amount: f32) -> Color {
    match (from, to) {
        (Color::Rgb(a, b, c), Color::Rgb(d, e, f)) => {
            let mix = |x: u8, y: u8| (f32::from(x) + (f32::from(y) - f32::from(x)) * amount) as u8;
            Color::Rgb(mix(a, d), mix(b, e), mix(c, f))
        }
        _ => to,
    }
}

fn gray(level: u8) -> Color {
    Color::Rgb(level, level, level)
}

/// What is drawn in a cell, from the back to the front.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Layer {
    Star,
    Back,
    Planet,
    Front,
    Electron,
}

/// A canvas of dots that is turned into braille characters.
struct Canvas {
    cols: usize,
    rows: usize,
    bits: Vec<u8>,
    layer: Vec<Option<Layer>>,
}

impl Canvas {
    fn new(cols: usize, rows: usize) -> Canvas {
        Canvas {
            cols,
            rows,
            bits: vec![0; cols * rows],
            layer: vec![None; cols * rows],
        }
    }

    fn dot(&mut self, x: i32, y: i32, layer: Layer) {
        if x < 0 || y < 0 {
            return;
        }
        let (x, y) = (x as usize, y as usize);
        let (col, row) = (x / 2, y / 4);
        if col >= self.cols || row >= self.rows {
            return;
        }
        const BITS: [[u8; 4]; 2] = [[0x01, 0x02, 0x04, 0x40], [0x08, 0x10, 0x20, 0x80]];
        let cell = row * self.cols + col;
        self.bits[cell] |= BITS[x % 2][y % 4];
        if self.layer[cell].is_none_or(|old| layer > old) {
            self.layer[cell] = Some(layer);
        }
    }

    fn clear(&mut self, x: i32, y: i32) {
        let (col, row) = (x as usize / 2, y as usize / 4);
        if x >= 0 && y >= 0 && col < self.cols && row < self.rows {
            const BITS: [[u8; 4]; 2] = [[0x01, 0x02, 0x04, 0x40], [0x08, 0x10, 0x20, 0x80]];
            self.bits[row * self.cols + col] &= !BITS[x as usize % 2][y as usize % 4];
        }
    }
}

/// Ordered dithering: a dot is lit when the brightness beats this threshold.
const BAYER: [[f32; 4]; 4] = [
    [0.0, 8.0, 2.0, 10.0],
    [12.0, 4.0, 14.0, 6.0],
    [3.0, 11.0, 1.0, 9.0],
    [15.0, 7.0, 13.0, 5.0],
];

/// The picture as lines: a planet with an orbit, the electron at `angle`.
/// Apart from that motion, `glow` (0 to 1) turns the orbit orange and `aim`
/// moves the light on the planet toward one side; both ease in and out.
fn art_lines(area: Rect, glow: f32, aim: (f32, f32), angle: f32) -> Vec<Line<'static>> {
    let (cols, rows) = (usize::from(area.width), usize::from(ART_ROWS));
    let mut canvas = Canvas::new(cols, rows);
    let (w, h) = (cols as f32 * 2.0, rows as f32 * 4.0);
    let (cx, cy) = (w / 2.0, h / 2.0);
    let radius = h * 0.40;
    let inside = |x: f32, y: f32| x * x + y * y < radius * radius;

    // A few stars.
    for row in 0..rows {
        for col in 0..cols {
            let n = mix(col, row);
            if n.is_multiple_of(41) {
                canvas.dot(
                    (col * 2 + n / 7 % 2) as i32,
                    (row * 4 + n / 11 % 4) as i32,
                    Layer::Star,
                );
            }
        }
    }

    let trace = |canvas: &mut Canvas, front: bool| {
        for orbit in &ORBITS {
            for step in 0..600 {
                let t = step as f32 / 600.0 * std::f32::consts::TAU;
                let (x, y, in_front) = orbit.at(t);
                if in_front != front || (!front && inside(x, y)) {
                    continue;
                }
                let layer = if front { Layer::Front } else { Layer::Back };
                canvas.dot((cx + x).round() as i32, (cy + y).round() as i32, layer);
            }
        }
    };
    trace(&mut canvas, false);

    // The planet: brightness from a light at the upper left, dithered.
    let light = (
        -0.55 + (aim.0 * 0.8 + 0.55) * glow,
        -0.45 + (aim.1 * 0.8 + 0.45) * glow,
        0.70 - 0.10 * glow,
    );
    for py in 0..(rows * 4) {
        for px in 0..(cols * 2) {
            let (x, y) = (px as f32 + 0.5 - cx, py as f32 + 0.5 - cy);
            if !inside(x, y) {
                continue;
            }
            let z = (radius * radius - x * x - y * y).sqrt();
            let shade = ((x * light.0 + y * light.1 + z * light.2) / radius).clamp(0.0, 1.0);
            let threshold = (BAYER[py % 4][px % 4] + 0.5) / 16.0;
            canvas.clear(px as i32, py as i32);
            if shade * 1.15 + 0.06 > threshold {
                canvas.dot(px as i32, py as i32, Layer::Planet);
            }
        }
    }
    trace(&mut canvas, true);

    // The electron.
    for orbit in &ORBITS {
        let t = angle + 0.9;
        let (x, y, in_front) = orbit.at(t);
        if !in_front && inside(x, y) {
            continue;
        }
        for (dx, dy) in [
            (0, 0),
            (1, 0),
            (-1, 0),
            (0, 1),
            (0, -1),
            (1, 1),
            (-1, -1),
            (1, -1),
            (-1, 1),
        ] {
            canvas.dot(
                (cx + x).round() as i32 + dx,
                (cy + y).round() as i32 + dy,
                Layer::Electron,
            );
        }
    }

    (0..rows)
        .map(|row| {
            let spans: Vec<Span> = (0..cols)
                .map(|col| {
                    let cell = row * cols + col;
                    let Some(layer) = canvas.layer[cell].filter(|_| canvas.bits[cell] != 0) else {
                        return Span::raw(" ");
                    };
                    let color = match layer {
                        Layer::Star => gray(70),
                        Layer::Back => blend(gray(84), Color::Rgb(122, 74, 46), glow),
                        Layer::Planet => {
                            let (x, y) = (col as f32 * 2.0 + 1.0 - cx, row as f32 * 4.0 + 2.0 - cy);
                            let z = (radius * radius - x * x - y * y).max(0.0).sqrt();
                            let shade = ((x * light.0 + y * light.1 + z * light.2) / radius)
                                .clamp(0.0, 1.0);
                            gray(70 + (shade * 175.0) as u8)
                        }
                        Layer::Front => blend(gray(175), ACCENT, glow),
                        Layer::Electron => blend(FG, ACCENT, glow),
                    };
                    let glyph =
                        char::from_u32(0x2800 + u32::from(canvas.bits[cell])).unwrap_or(' ');
                    Span::styled(glyph.to_string(), Style::new().fg(color))
                })
                .collect();
            Line::from(spans)
        })
        .collect()
}

impl Menu {
    pub fn new() -> Menu {
        Menu::default()
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
        self.angle = (self.angle + 0.042 + 0.058 * self.glow) % std::f32::consts::TAU;
    }

    fn select(&mut self, index: usize) {
        self.selected = index.min(ITEMS.len() - 1);
    }

    fn activate(&self) -> Action {
        match ITEMS[self.selected] {
            Item::Browse => Action::Browse,
            Item::Here => Action::AnalyzeHere,
            Item::Language => Action::SwitchLanguage,
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
            KeyCode::Char('L') => return Some(Action::SwitchLanguage),
            _ => {}
        }
        None
    }

    /// Click an item to select it and click it again to run it.
    pub fn handle_mouse(&mut self, kind: MouseEventKind, column: u16, row: u16) -> Option<Action> {
        let at = Position::new(column, row);
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

fn language_name(lang: Lang) -> &'static str {
    match lang {
        Lang::En => "English",
        Lang::PtBr => "Português",
    }
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
    let mut lines: Vec<Line> = art_lines(menu.logo_area, menu.glow, menu.aim, menu.angle);
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
                Item::Language => (
                    format!("{}: {}", t.menu_language, language_name(lang)),
                    t.menu_language_hint.to_string(),
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
                ("L", t.key_language),
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
    fn the_picture_has_the_size_it_says() {
        let lines = art_lines(Rect::new(0, 0, 62, ART_ROWS), 0.0, (0.0, 0.0), 0.0);
        assert_eq!(lines.len(), usize::from(ART_ROWS));
        assert!(lines.iter().all(|l| l.spans.len() == 62));
    }

    #[test]
    fn the_planet_and_its_electron_are_drawn() {
        let picture = text(&art_lines(
            Rect::new(0, 0, 62, ART_ROWS),
            0.0,
            (0.0, 0.0),
            0.0,
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
        let mut menu = Menu::new();
        menu.logo_area = area;
        menu.tick();
        assert!(menu.angle > 0.0);
        assert_ne!(
            text(&art_lines(area, 0.0, (0.0, 0.0), 0.0)),
            text(&art_lines(area, 0.0, (0.0, 0.0), 1.0))
        );
    }

    #[test]
    fn the_glow_rises_under_the_pointer_and_fades_slowly_after() {
        let mut menu = Menu::new();
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
            art_lines(area, glow, (0.0, 0.0), 0.0)
                .iter()
                .flat_map(|l| l.spans.iter().map(|s| s.style.fg))
                .filter(|c| *c == Some(ACCENT))
                .count()
        };
        assert_eq!(accent(0.0), 0);
        assert!(accent(1.0) > 0);
    }
}
