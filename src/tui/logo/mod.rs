//! The picture on the main menu, drawn with braille characters.
//!
//! # Changing the logo
//!
//! Every logo lives in its own file in this folder. A flat one exposes a
//! function `lines(area, glow, aim, angle)`; a 3D one is a `Solid` that
//! `solid.rs` knows how to draw and turn. Nothing is ever
//! deleted when the logo changes, you only pick another one:
//!
//! * `planet.rs`: the sphere with an orbit and an electron (the original).
//! * `lock.rs` and `key.rs`: a 3D padlock and a 3D key, drawn by `solid.rs`.
//!
//! To pick one, either change [`DEFAULT`] below or run with
//! `AGENTQ_LOGO=planet` (or `lock`, `key`) in the environment. The choice can
//! also be made in the settings screen, which remembers it.
//!
//! To add a new logo: create `my_logo.rs` (copy `key.rs` for a 3D one), add a
//! variant to [`Logo`], and add it to [`Logo::name`], [`Logo::ALL`],
//! [`Logo::lines`] and the labels in the translations. The shared pieces ([`Canvas`], [`BAYER`], [`gray`]) are
//! here so every logo uses the same dots and dithering.

mod key;
mod lock;
mod planet;
mod solid;

use ratatui::crossterm::event::{MouseButton, MouseEventKind};
use ratatui::layout::{Position, Rect};
use ratatui::style::Color;
use ratatui::text::Line;

/// The logo shown when `AGENTQ_LOGO` is not set. Change this one line to
/// switch the logo for everybody.
pub const DEFAULT: Logo = Logo::Planet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Logo {
    Planet,
    Lock,
    Key,
}

impl Default for Logo {
    fn default() -> Logo {
        DEFAULT
    }
}

impl Logo {
    pub const ALL: [Logo; 3] = [Logo::Planet, Logo::Lock, Logo::Key];

    pub fn name(self) -> &'static str {
        match self {
            Logo::Planet => "planet",
            Logo::Lock => "lock",
            Logo::Key => "key",
        }
    }

    /// The logo called `name` (`planet`, `lock`, `key`), in any case.
    pub fn parse(name: &str) -> Option<Logo> {
        let name = name.trim().to_ascii_lowercase();
        Logo::ALL.into_iter().find(|logo| logo.name() == name)
    }

    /// The logo to start with: the one named by `AGENTQ_LOGO`, else the one
    /// saved in the settings (`saved`), else [`DEFAULT`].
    pub fn resolve(saved: Option<&str>) -> Logo {
        std::env::var("AGENTQ_LOGO")
            .ok()
            .and_then(|value| Logo::parse(&value))
            .or_else(|| saved.and_then(Logo::parse))
            .unwrap_or(DEFAULT)
    }

    /// The next logo in the list, or the previous one with `back`.
    pub fn step(self, back: bool) -> Logo {
        let at = Logo::ALL.iter().position(|logo| *logo == self).unwrap_or(0);
        let n = Logo::ALL.len();
        Logo::ALL[(at + if back { n - 1 } else { 1 }) % n]
    }

    /// The name shown to the user.
    pub fn label(self, lang: crate::i18n::Lang) -> &'static str {
        let t = lang.t();
        match self {
            Logo::Planet => t.logo_planet,
            Logo::Lock => t.logo_lock,
            Logo::Key => t.logo_key,
        }
    }

    /// The picture as lines. `angle` is the animation, `turn` is how far the
    /// user has turned a 3D logo by hand, `glow` (0 to 1) is how much the
    /// pointer lights it and `aim` is which way the pointer is.
    pub fn lines(
        self,
        area: Rect,
        glow: f32,
        aim: (f32, f32),
        angle: f32,
        turn: (f32, f32),
    ) -> Vec<Line<'static>> {
        match self {
            Logo::Planet => planet::lines(area, glow, aim, angle),
            Logo::Lock => solid::lines(&lock::PADLOCK, area, glow, aim, angle, turn),
            Logo::Key => solid::lines(&key::KEY, area, glow, aim, angle, turn),
        }
    }

    /// Whether the user can turn it with the mouse.
    pub fn is_3d(self) -> bool {
        self != Logo::Planet
    }
}

/// Height of the picture in rows. It is drawn with braille characters, which
/// hold a grid of 2 by 4 dots each, so curves come out smooth.
pub const ART_ROWS: u16 = 9;

fn mix(a: usize, b: usize) -> usize {
    let mut h = (a as u32).wrapping_mul(0x9E37_79B1) ^ (b as u32).wrapping_mul(0x85EB_CA6B);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    h as usize
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

/// How far a drag of one column turns the logo, in radians. A cell is about
/// twice as tall as it is wide, so a row turns it about twice as far.
const TURN_PER_COLUMN: f32 = 0.11;
const TURN_PER_ROW: f32 = 0.22;

/// The user's hand on a 3D logo: drag it to turn it, let go and it keeps
/// turning a little while it slows down.
#[derive(Debug, Default)]
pub struct Spin {
    /// How far it was turned: around the vertical axis and around the
    /// horizontal one, in radians.
    pub turn: (f32, f32),
    /// Where the pointer was when a drag last moved.
    pub drag: Option<Position>,
    /// How fast the turn keeps going after it is let go.
    pub momentum: (f32, f32),
}

impl Spin {
    /// Whether the logo is being held.
    pub fn held(&self) -> bool {
        self.drag.is_some()
    }

    /// One step of the animation after it is let go.
    pub fn coast(&mut self) {
        self.turn.0 += self.momentum.0;
        self.turn.1 += self.momentum.1;
        self.momentum = (self.momentum.0 * 0.93, self.momentum.1 * 0.93);
    }

    /// Follows the pointer. Returns whether the event belonged to the logo:
    /// a press on `area`, or a move or release while it is held.
    pub fn handle_mouse(&mut self, kind: MouseEventKind, at: Position, area: Rect) -> bool {
        match kind {
            MouseEventKind::Down(MouseButton::Left) if area.contains(at) => {
                self.drag = Some(at);
                self.momentum = (0.0, 0.0);
                true
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                let Some(from) = self.drag else { return false };
                let step = (
                    (f32::from(at.x) - f32::from(from.x)) * TURN_PER_COLUMN,
                    (f32::from(at.y) - f32::from(from.y)) * TURN_PER_ROW,
                );
                self.turn.0 += step.0;
                self.turn.1 += step.1;
                self.momentum = step;
                self.drag = Some(at);
                true
            }
            MouseEventKind::Up(MouseButton::Left) => self.drag.take().is_some(),
            _ => false,
        }
    }
}
