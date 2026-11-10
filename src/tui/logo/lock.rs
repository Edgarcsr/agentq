//! The padlock logo. Change the numbers in [`padlock`] to change its shape
//! (the units are such that the padlock is about 1.4 tall), and [`steel`] to
//! change its color.

use super::gray;
use super::solid::{Solid, V, round_box};
use crate::tui::ui::{ACCENT, blend};
use ratatui::style::Color;

pub const PADLOCK: Solid = Solid {
    sdf: padlock,
    pocket,
    color: steel,
    tilt: 0.0,
    size: 0.50,
};

/// The padlock's color for a brightness from 0 to 1.
fn steel(shade: f32, glow: f32) -> Color {
    blend(gray(60 + (shade * 185.0) as u8), ACCENT, glow * 0.22)
}

/// The keyhole: a pocket cut into the front face.
fn keyhole(p: V) -> f32 {
    let round = p[0].hypot(p[1] + 0.12) - 0.09;
    let slot = (p[0].abs() - 0.035).max((p[1] + 0.25).abs() - 0.13);
    round.min(slot).max((p[2] + 0.27).abs() - 0.12)
}

/// How much light the keyhole swallows at a point.
fn pocket(p: V) -> f32 {
    if keyhole(p) < 0.02 { 0.3 } else { 1.0 }
}

/// The padlock, facing -z, centered on the origin.
pub fn padlock(p: V) -> f32 {
    let body = round_box([p[0], p[1] + 0.22, p[2]], [0.62, 0.48, 0.27], 0.14);
    let body = body.max(-keyhole(p));

    // The shackle: an arch with two legs that sink into the body.
    let (rise, radius, leg, thickness) = (0.26, 0.38, 0.12, 0.085);
    let (x, y) = (p[0], p[1] - rise);
    let path = if y > 0.0 {
        (x.hypot(y) - radius).abs()
    } else {
        (x.abs() - radius).hypot((-y - leg).max(0.0))
    };
    let shackle = path.hypot(p[2]) - thickness;
    body.min(shackle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_padlock_is_solid_where_it_should_be() {
        assert!(padlock([0.0, -0.4, 0.0]) < 0.0, "body");
        assert!(padlock([1.2, 0.0, 0.0]) > 0.0, "outside");
        assert!(padlock([0.0, 0.26 + 0.38, 0.0]) < 0.0, "shackle");
        assert!(padlock([0.0, 0.5, 0.0]) > 0.0, "the shackle is open");
    }
}
