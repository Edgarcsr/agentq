//! The key logo. Change the numbers in [`key`] to change its shape, and
//! [`brass`] to change its color. It lies along x, ring on the left and teeth
//! on the right, and rests tilted by [`KEY`]'s `tilt`.

use super::solid::{Solid, V, round_box};
use crate::tui::ui::{ACCENT, FG, blend};
use ratatui::style::Color;

pub const KEY: Solid = Solid {
    sdf: key,
    pocket: |_| 1.0,
    color: brass,
    tilt: 0.6,
    size: 0.46,
};

/// The key's color for a brightness from 0 to 1.
fn brass(shade: f32, glow: f32) -> Color {
    let lit = blend(Color::Rgb(72, 38, 14), ACCENT, shade.min(1.0));
    let shine = ((shade - 0.8) / 0.2).clamp(0.0, 1.0) * (0.35 + 0.25 * glow);
    blend(lit, FG, shine)
}

/// The key, centered on the origin.
pub fn key(p: V) -> f32 {
    let p = [p[0] - 0.08, p[1], p[2]];
    let bow = ((p[0] + 0.62).hypot(p[1]) - 0.25).hypot(p[2]) - 0.09;
    let shaft = {
        let x = p[0].clamp(-0.4, 0.78);
        (p[0] - x).hypot(p[1]).hypot(p[2]) - 0.07
    };
    let tooth = |at: f32, y: f32, half: f32| {
        round_box([p[0] - at, p[1] - y, p[2]], [0.055, half, 0.045], 0.02)
    };
    bow.min(shaft)
        .min(tooth(0.52, -0.15, 0.09))
        .min(tooth(0.68, -0.13, 0.07))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shaft_is_solid_and_the_ring_is_open() {
        assert!(key([0.2, 0.0, 0.0]) < 0.0, "shaft");
        assert!(key([-0.54, 0.0, 0.0]) > 0.0, "the ring is open");
        assert!(key([-0.54 - 0.25, 0.0, 0.0]) < 0.0, "ring");
    }
}
