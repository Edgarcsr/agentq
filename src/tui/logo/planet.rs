//! The planet logo: a dithered sphere with one orbit and an electron on it.

use super::{ART_ROWS, BAYER, Canvas, Layer, gray, mix};
use crate::tui::ui::{ACCENT, FG, blend};
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

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

/// The picture as lines: a planet with an orbit, the electron at `angle`.
/// Apart from that motion, `glow` (0 to 1) turns the orbit orange and `aim`
/// moves the light on the planet toward one side; both ease in and out.
pub fn lines(area: Rect, glow: f32, aim: (f32, f32), angle: f32) -> Vec<Line<'static>> {
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
