//! The little plate of a character's game mode (Java `ModeBadge`): the mode's name on a fade of its own colours. PPE
//! orange into brown, UPE purple into gold, pure UPE purple, STPE gold, shiny UPE holographic with sparkles, divine PE
//! gold with stars and light beams. A guild version shows its plain mode in the plate and a small gold shield right
//! of it; a run without a pet a struck-out paw right of that. Drawn at a scale `k`.

use crate::jtext;
use egui::{Align2, Color32, Mesh, Painter, Pos2, Shape, Stroke, pos2};
use lg_app::upe;

pub const H: f32 = 26.0;
const PAD: f32 = 10.0;
const SHIELD: f32 = 16.0;
const SHIELD_GAP: f32 = 5.0;
const PAW: f32 = 22.0;

fn rgb(c: u32) -> Color32 {
    Color32::from_rgb((c >> 16) as u8, (c >> 8) as u8, c as u8)
}

fn font(k: f32) -> egui::FontId {
    crate::theme::head(17.0 * k)
}

/// What the plate says: the mode without its guild part (the shield shows that).
fn label(mode: &str) -> String {
    upe::label(upe::base(mode))
}

fn plate_width(painter: &Painter, mode: &str) -> f32 {
    PAD + jtext::width(painter, &label(mode), &font(1.0)) + PAD
}

/// How wide the plate (with the guild's shield and the no-pet paw) is drawn at scale 1.
pub fn width(painter: &Painter, mode: &str, no_pet: bool) -> f32 {
    plate_width(painter, mode) + if upe::guild(mode) { SHIELD_GAP + SHIELD } else { 0.0 } + if no_pet { SHIELD_GAP + PAW } else { 0.0 }
}

struct Style {
    colors: Vec<Color32>,
    edge: Color32,
    text: Color32,
    shadow: Option<Color32>,
    glow: Option<Color32>,
}

fn style(base: &str) -> Style {
    let s = |a: u32, b: u32, edge: u32, text: Color32, shadow: Option<Color32>, glow: Option<Color32>| Style {
        colors: vec![rgb(a), rgb(b)],
        edge: rgb(edge),
        text,
        shadow,
        glow,
    };
    let w = Color32::WHITE;
    match base {
        upe::PPE => s(0xF2A64A, 0x9A5420, 0xFFCB8A, w, Some(rgb(0x4A2208)), None),
        upe::PUPE => s(0xB57BE8, 0x4E2384, 0xDDB8F7, w, Some(rgb(0x240B40)), None),
        upe::STPE => {
            s(0xFFEA8A, 0xE0A815, 0xFFF6C8, rgb(0x4A3000), Some(Color32::from_rgba_unmultiplied(255, 255, 255, 110)), Some(rgb(0xFFD24A)))
        }
        upe::SHINY_UPE => Style {
            colors: [0xFF7EC8, 0xFFE27A, 0x7DF3E1, 0x8FA8FF, 0xD58BFF].iter().map(|c| rgb(*c)).collect(),
            edge: w,
            text: w,
            shadow: Some(rgb(0x4A2A6A)),
            glow: Some(rgb(0xBFF6FF)),
        },
        upe::DIVINE_PE => {
            s(0xFFE68C, 0xC98A12, 0xFFF6CF, rgb(0x4A2E00), Some(Color32::from_rgba_unmultiplied(255, 255, 255, 90)), Some(rgb(0xF2C232)))
        }
        _ => s(0x8E4FD0, 0xE0A92A, 0xE6C8FF, w, Some(rgb(0x2A0F45)), None),
    }
}

fn lerp(a: Color32, b: Color32, t: f32) -> Color32 {
    let m = |x: u8, y: u8| (f32::from(x) + (f32::from(y) - f32::from(x)) * t).round() as u8;
    Color32::from_rgba_unmultiplied(m(a.r(), b.r()), m(a.g(), b.g()), m(a.b(), b.b()), m(a.a(), b.a()))
}

/// The colour at t (0..1) along evenly spaced stops.
fn stop(colors: &[Color32], t: f32) -> Color32 {
    if colors.len() == 1 {
        return colors[0];
    }
    let t = t.clamp(0.0, 1.0) * (colors.len() - 1) as f32;
    let i = (t.floor() as usize).min(colors.len() - 2);
    lerp(colors[i], colors[i + 1], t - i as f32)
}

/// A rounded rectangle's outline, its straight sides cut into short pieces so a fade follows its stops.
fn rounded(x: f32, y: f32, w: f32, h: f32, r: f32) -> Vec<Pos2> {
    let mut pts = Vec::new();
    let arc = |pts: &mut Vec<Pos2>, cx: f32, cy: f32, from: f32| {
        for i in 0..=6 {
            let a = (from + 90.0 * i as f32 / 6.0).to_radians();
            pts.push(pos2(cx + r * a.cos(), cy + r * a.sin()));
        }
    };
    let side = |pts: &mut Vec<Pos2>, a: Pos2, b: Pos2| {
        let n = ((b - a).length() / 2.0).ceil().max(1.0) as usize;
        for i in 1..n {
            pts.push(a + (b - a) * (i as f32 / n as f32));
        }
    };
    arc(&mut pts, x + w - r, y + r, -90.0);
    let a = *pts.last().expect("arc");
    side(&mut pts, a, pos2(x + w, y + h - r));
    arc(&mut pts, x + w - r, y + h - r, 0.0);
    let a = *pts.last().expect("arc");
    side(&mut pts, a, pos2(x + r, y + h));
    arc(&mut pts, x + r, y + h - r, 90.0);
    let a = *pts.last().expect("arc");
    side(&mut pts, a, pos2(x, y + r));
    arc(&mut pts, x + r, y + r, 180.0);
    pts
}

/// A shape that is star-shaped around `c` (convex ones too), each point coloured by `color`.
fn fan(painter: &Painter, c: Pos2, pts: &[Pos2], color: impl Fn(Pos2) -> Color32) {
    let mut mesh = Mesh::default();
    mesh.colored_vertex(c, color(c));
    for p in pts {
        mesh.colored_vertex(*p, color(*p));
    }
    let n = pts.len() as u32;
    for i in 0..n {
        mesh.add_triangle(0, 1 + i, 1 + (i + 1) % n);
    }
    painter.add(Shape::mesh(mesh));
}

fn star_points(cx: f32, cy: f32, r: f32, waist: f32) -> Vec<Pos2> {
    vec![
        pos2(cx, cy - r),
        pos2(cx + waist, cy - waist),
        pos2(cx + r, cy),
        pos2(cx + waist, cy + waist),
        pos2(cx, cy + r),
        pos2(cx - waist, cy + waist),
        pos2(cx - r, cy),
        pos2(cx - waist, cy - waist),
    ]
}

fn star(painter: &Painter, cx: f32, cy: f32, r: f32, waist: f32, fill: Color32) {
    fan(painter, pos2(cx, cy), &star_points(cx, cy, r, waist), |_| fill);
}

/// A star with a soft halo, to stand out on a bright plate.
fn glint(painter: &Painter, cx: f32, cy: f32, r: f32) {
    let rr = r * 1.2;
    let ring: Vec<Pos2> =
        (0..20).map(|i| i as f32 / 20.0 * std::f32::consts::TAU).map(|a| pos2(cx + rr * a.cos(), cy + rr * a.sin())).collect();
    fan(painter, pos2(cx, cy), &ring, |p| if p == pos2(cx, cy) { Color32::from_white_alpha(170) } else { Color32::TRANSPARENT });
    star(painter, cx, cy, r, r * 0.22, Color32::WHITE);
}

/// Draws the plate with its top-left corner at `at`, `k` times its size.
pub fn draw(painter: &Painter, at: Pos2, mode: &str, no_pet: bool, k: f32) {
    let base = upe::base(mode);
    let (x, y) = (at.x, at.y);
    let w = plate_width(painter, mode) * k;
    let h = H * k;
    let s = style(base);
    let divine = base == upe::DIVINE_PE;
    let shiny = base == upe::SHINY_UPE;
    if divine {
        // golden light beams fanning out behind the plate
        let (cx, cy) = (x + w / 2.0, y + h / 2.0);
        let reach = w * 0.85;
        let gold = |d: f32| Color32::from_rgba_unmultiplied(255, 220, 100, (185.0 * (1.0 - d / reach).max(0.0)) as u8);
        for i in 0..18 {
            let a = std::f32::consts::TAU * i as f32 / 18.0;
            let b = a + 0.11;
            let p1 = pos2(cx + a.cos() * w * 0.85, cy + a.sin() * h * 1.15);
            let p2 = pos2(cx + b.cos() * w * 0.85, cy + b.sin() * h * 1.15);
            let mut mesh = Mesh::default();
            mesh.colored_vertex(pos2(cx, cy), gold(0.0));
            mesh.colored_vertex(p1, gold((p1 - pos2(cx, cy)).length()));
            mesh.colored_vertex(p2, gold((p2 - pos2(cx, cy)).length()));
            mesh.add_triangle(0, 1, 2);
            painter.add(Shape::mesh(mesh));
        }
    }
    if let Some(g) = s.glow {
        for i in (1..=5).rev() {
            let i = i as f32 * k;
            let a = if divine { 40 } else { 30 };
            painter.rect_filled(
                egui::Rect::from_min_size(pos2(x - i, y - i), egui::vec2(w + 2.0 * i, h + 2.0 * i)),
                5.0 * k + i,
                Color32::from_rgba_unmultiplied(g.r(), g.g(), g.b(), a),
            );
        }
    }
    let r = 5.0 * k;
    let outline = rounded(x, y, w, h, r);
    fan(painter, pos2(x + w / 2.0, y + h / 2.0), &outline, |p| stop(&s.colors, (p.y - y) / h));
    // a sheen on top
    let top = rounded(x + k, y + k, w - 2.0 * k, h / 2.0, 4.5 * k);
    fan(painter, pos2(x + w / 2.0, y + h / 4.0), &top, |p| {
        Color32::from_white_alpha((60.0 * (1.0 - ((p.y - y) / (h / 2.0)).clamp(0.0, 1.0))) as u8)
    });
    if divine {
        for (sx, sy, sr) in
            [(6.0, 6.0, 3.2), (w / k - 7.0, 7.0, 3.6), (w / k * 0.28, H - 6.0, 2.4), (w / k * 0.7, H - 5.0, 2.6), (w / k * 0.5, 4.0, 2.0)]
        {
            star(painter, x + sx * k, y + sy * k, sr * k, sr * 0.25 * k, Color32::from_rgba_unmultiplied(255, 255, 240, 220));
        }
    }
    painter.add(Shape::closed_line(outline, Stroke::new(1.4 * k, s.edge)));
    let t = label(mode);
    let f = font(k);
    let c = pos2(x + w / 2.0, y + h / 2.0);
    if let Some(sh) = s.shadow {
        jtext::draw(painter, c + egui::vec2(k, k), Align2::CENTER_CENTER, &t, &f, sh);
    }
    jtext::draw(painter, c, Align2::CENTER_CENTER, &t, &f, s.text);
    if shiny {
        // over the text's edges too: they should be seen
        let wk = w / k;
        for (gx, gy, gr) in [
            (-3.0, 3.0, 6.0),
            (wk - 4.0, 4.0, 6.5),
            (wk * 0.3, H - 2.0, 4.0),
            (wk * 0.62, 1.0, 4.5),
            (wk + 2.0, H - 4.0, 4.5),
            (6.0, H - 3.0, 3.2),
            (wk * 0.82, H - 1.0, 3.5),
        ] {
            glint(painter, x + gx * k, y + gy * k, gr * k);
        }
    }
    let mut after = x + w;
    if upe::guild(mode) {
        shield(painter, pos2(after + SHIELD_GAP * k, y + (H - SHIELD - 4.0) / 2.0 * k), k);
        after += (SHIELD_GAP + SHIELD) * k;
    }
    if no_pet {
        no_pet_icon(painter, pos2(after + SHIELD_GAP * k, y + (H - PAW) / 2.0 * k), k);
    }
}

/// A small plain gold heater shield: the guild (16 x 20 at scale 1).
pub fn shield(painter: &Painter, at: Pos2, k: f32) {
    let (x, y, w, h) = (at.x, at.y, SHIELD * k, (SHIELD + 4.0) * k);
    let mut pts = vec![pos2(x, y), pos2(x + w, y), pos2(x + w, y + h * 0.45)];
    let quad = |p0: Pos2, c: Pos2, p1: Pos2, pts: &mut Vec<Pos2>| {
        for i in 1..=8 {
            let t = i as f32 / 8.0;
            let u = 1.0 - t;
            pts.push(pos2(u * u * p0.x + 2.0 * u * t * c.x + t * t * p1.x, u * u * p0.y + 2.0 * u * t * c.y + t * t * p1.y));
        }
    };
    quad(pos2(x + w, y + h * 0.45), pos2(x + w, y + h * 0.85), pos2(x + w / 2.0, y + h), &mut pts);
    quad(pos2(x + w / 2.0, y + h), pos2(x, y + h * 0.85), pos2(x, y + h * 0.45), &mut pts);
    let (top, bottom) = (rgb(0xF6D36B), rgb(0xA8740E));
    fan(painter, pos2(x + w / 2.0, y + h / 2.0), &pts, |p| lerp(top, bottom, ((p.y - y) / h).clamp(0.0, 1.0)));
    painter.add(Shape::closed_line(pts, Stroke::new(1.4 * k, rgb(0x4A3205))));
    painter
        .line_segment([pos2(x + w / 2.0, y + 3.0 * k), pos2(x + w / 2.0, y + h - 4.0 * k)], Stroke::new(k, Color32::from_white_alpha(90)));
}

/// A paw print struck out in red: played without a pet (22 x 22 at scale 1).
pub fn no_pet_icon(painter: &Painter, at: Pos2, k: f32) {
    let g = PAW / 16.0 * k; // the paw is drawn on a 16 px grid
    let parts = [(3.5, 7.5, 9.0, 7.5), (0.5, 4.5, 3.6, 4.4), (4.2, 1.0, 3.6, 4.6), (8.4, 1.0, 3.6, 4.6), (12.0, 4.5, 3.6, 4.4)];
    let ellipse = |off: egui::Vec2, (ex, ey, ew, eh): (f32, f32, f32, f32), fill: Color32| {
        let (cx, cy) = (at.x + (ex + ew / 2.0) * g + off.x, at.y + (ey + eh / 2.0) * g + off.y);
        let pts: Vec<Pos2> = (0..24)
            .map(|i| i as f32 / 24.0 * std::f32::consts::TAU)
            .map(|a| pos2(cx + ew / 2.0 * g * a.cos(), cy + eh / 2.0 * g * a.sin()))
            .collect();
        painter.add(Shape::convex_polygon(pts, fill, Stroke::NONE));
    };
    // a soft dark shadow, so it shows on any card
    for e in parts {
        ellipse(egui::vec2(k, 1.2 * k), e, Color32::from_black_alpha(120));
    }
    for e in parts {
        ellipse(egui::Vec2::ZERO, e, rgb(0xE6EAEE));
    }
    let (a, b) = (pos2(at.x + k, at.y + (PAW - 2.0) * k), pos2(at.x + (PAW - 1.0) * k, at.y + k));
    painter.line_segment([a, b], Stroke::new(3.4 * k, rgb(0x5A1410)));
    painter.line_segment([a, b], Stroke::new(2.2 * k, rgb(0xFF5A4E)));
}
