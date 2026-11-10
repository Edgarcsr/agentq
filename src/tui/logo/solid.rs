//! Draws one 3D solid with braille dots.
//!
//! A solid is described as a signed distance function (the distance from a
//! point to its nearest surface, negative inside). Every dot of the picture
//! marches a ray through it, the surface is lit and the light is dithered, so
//! it reads well in a terminal. `lock.rs` and `key.rs` only say what their
//! shape is; the drawing and the rotation are here.
//!
//! The solid turns on two axes at once, like the famous spinning donut: it
//! spins around the vertical axis and nods around the horizontal one. On top
//! of that the user can turn it by hand (`turn`, in radians).

use super::{ART_ROWS, BAYER, Canvas, Layer};
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

pub type V = [f32; 3];

/// Everything that makes one solid different from another.
pub struct Solid {
    /// The shape, centered on the origin and facing -z (toward the viewer).
    pub sdf: fn(V) -> f32,
    /// How much light a point swallows, from 0 to 1 (1 is none). It darkens
    /// pockets such as a keyhole.
    pub pocket: fn(V) -> f32,
    /// The color for a brightness from 0 to 1, and the glow from 0 to 1.
    pub color: fn(f32, f32) -> Color,
    /// How the solid rests before it turns, around z, in radians.
    pub tilt: f32,
    /// How big a unit is, as a share of the picture height.
    pub size: f32,
}

pub fn rot_x(p: V, a: f32) -> V {
    let (s, c) = a.sin_cos();
    [p[0], p[1] * c - p[2] * s, p[1] * s + p[2] * c]
}

pub fn rot_y(p: V, a: f32) -> V {
    let (s, c) = a.sin_cos();
    [p[0] * c + p[2] * s, p[1], -p[0] * s + p[2] * c]
}

pub fn rot_z(p: V, a: f32) -> V {
    let (s, c) = a.sin_cos();
    [p[0] * c - p[1] * s, p[0] * s + p[1] * c, p[2]]
}

fn norm(v: V) -> V {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-6);
    [v[0] / len, v[1] / len, v[2] / len]
}

fn dot3(a: V, b: V) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// A box with rounded edges, centered at the origin.
pub fn round_box(p: V, half: V, radius: f32) -> f32 {
    let q = [
        p[0].abs() - half[0] + radius,
        p[1].abs() - half[1] + radius,
        p[2].abs() - half[2] + radius,
    ];
    let outside = (q[0].max(0.0).powi(2) + q[1].max(0.0).powi(2) + q[2].max(0.0).powi(2)).sqrt();
    outside + q[0].max(q[1]).max(q[2]).min(0.0) - radius
}

/// How far the solid has turned at `angle` (which goes around once per lap),
/// as (around the vertical axis, around the horizontal one).
fn pose(angle: f32, turn: (f32, f32)) -> (f32, f32) {
    (angle + turn.0, 0.5 * angle.sin() + turn.1)
}

/// The surface normal at `p`.
fn normal(sdf: &dyn Fn(V) -> f32, p: V) -> V {
    const E: f32 = 0.004;
    norm([
        sdf([p[0] + E, p[1], p[2]]) - sdf([p[0] - E, p[1], p[2]]),
        sdf([p[0], p[1] + E, p[2]]) - sdf([p[0], p[1] - E, p[2]]),
        sdf([p[0], p[1], p[2] + E]) - sdf([p[0], p[1], p[2] - E]),
    ])
}

/// The picture as lines. `angle` is the animation, `turn` is how far the user
/// has turned it, and `glow` (0 to 1) and `aim` move the light toward the
/// pointer.
pub fn lines(
    solid: &Solid,
    area: Rect,
    glow: f32,
    aim: (f32, f32),
    angle: f32,
    turn: (f32, f32),
) -> Vec<Line<'static>> {
    let (cols, rows) = (usize::from(area.width), usize::from(ART_ROWS));
    let mut canvas = Canvas::new(cols, rows);
    let mut shades: Vec<f32> = vec![0.0; cols * rows];
    let (w, h) = (cols as f32 * 2.0, rows as f32 * 4.0);
    let (cx, cy) = (w / 2.0, h / 2.0);
    let scale = (h * solid.size).min(w * 0.2);

    // Where the light comes from: the upper left, moving to the pointer.
    let light = norm([
        -0.55 + (aim.0 * 0.8 + 0.55) * glow,
        0.45 - (aim.1 * 0.8 + 0.45) * glow,
        -(0.70 - 0.10 * glow),
    ]);
    let half = norm([light[0], light[1], light[2] - 1.0]);

    // From the picture's space to the solid's own: undo the turns.
    let (yaw, pitch) = pose(angle, turn);
    let local = |p: V| rot_z(rot_y(rot_x(p, -pitch), -yaw), -solid.tilt);
    let sdf = |p: V| (solid.sdf)(local(p));

    let reach = (scale * 1.2) as usize;
    let (x0, y0) = (
        (cx as usize).saturating_sub(reach),
        (cy as usize).saturating_sub(reach),
    );
    for py in y0..(cy as usize + reach).min(rows * 4) {
        for px in x0..(cx as usize + reach).min(cols * 2) {
            let x = (px as f32 + 0.5 - cx) / scale;
            let y = -(py as f32 + 0.5 - cy) / scale;

            // March from the front toward the back until the surface is hit.
            let mut z = -1.4;
            let mut hit = None;
            for _ in 0..48 {
                let d = sdf([x, y, z]);
                if d < 0.003 {
                    hit = Some([x, y, z]);
                    break;
                }
                z += d * 0.9;
                if z > 1.4 {
                    break;
                }
            }
            let Some(p) = hit else { continue };

            let n = normal(&sdf, p);
            let diffuse = dot3(n, light).max(0.0);
            let shine = dot3(n, half).max(0.0).powi(24) * 0.6;
            let shade =
                ((0.10 + 0.85 * diffuse + shine) * (solid.pocket)(local(p))).clamp(0.0, 1.0);

            let threshold = (BAYER[py % 4][px % 4] + 0.5) / 16.0;
            if shade * 1.15 + 0.06 > threshold {
                canvas.dot(px as i32, py as i32, Layer::Planet);
                let cell = py / 4 * cols + px / 2;
                shades[cell] = shades[cell].max(shade);
            }
        }
    }

    (0..rows)
        .map(|row| {
            let spans: Vec<Span> = (0..cols)
                .map(|col| {
                    let cell = row * cols + col;
                    if canvas.bits[cell] == 0 {
                        return Span::raw(" ");
                    }
                    let glyph =
                        char::from_u32(0x2800 + u32::from(canvas.bits[cell])).unwrap_or(' ');
                    let color = (solid.color)(shades[cell], glow);
                    Span::styled(glyph.to_string(), Style::new().fg(color))
                })
                .collect();
            Line::from(spans)
        })
        .collect()
}
