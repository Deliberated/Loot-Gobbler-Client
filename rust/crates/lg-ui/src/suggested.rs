//! The Suggested Loadout box (Java: `CharacterDetail.ideal` and its parts): the scope, deleted-enchantment and second
//! chance switches, the enemy DEF, the stat boxes (each a toggle for what you aim for), and per slot the suggested
//! item with where its copy is, its stats, and the enchantments planned for it.

use crate::detail::{Rarity, enchant_badge, uv};
use crate::icons::Icons;
use crate::jtext;
use crate::profile::num;
use crate::theme::{Palette, fixed};
use crate::widgets::{self, small, text};
use egui::{Align2, Color32, Rect, Sense, Stroke, Ui, pos2, vec2};
use lg_app::App;
use lg_app::characters::Char;
use lg_app::loadout::{Enchantments, Pick, STATS, Settings, Summary};

const STAT_SHORT: [&str; 8] = ["HP", "MP", "ATT", "DEF", "SPD", "DEX", "VIT", "WIS"];
const SCOPES: [&str; 3] = ["current", "local", "global"];
const SECOND: [&str; 3] = ["off", "ami", "ami + armor"];
const SLOT_NAMES: [&str; 4] = ["Weapon", "Ability", "Armor", "Ring"];
const UP: Color32 = fixed::GREEN;
const DOWN: Color32 = Color32::from_rgb(0xFF, 0x8A, 0x2A);
const OTHER_STAT: Color32 = Color32::from_rgb(0x3A, 0x9B, 0xFF);

/// A click in the box: what it changes in the settings.
#[derive(Debug, Clone, PartialEq)]
pub enum Change {
    Toggle(String),
    Scope,
    ShowReplaced,
    SecondChance,
    SuggestEnchants,
    EnemyDef(f64),
    /// The item locked in a slot (or unlocked), and an enchantment kept on a slot (or no longer).
    ItemLock(usize, i32),
    EnchantLock(usize, i32),
}

const SCOPE_TIPS: [&str; 3] = [
    "Current: the items the character wears, with the best enchantments for them",
    "Local: the items the character wears or carries (inventory, backpack)",
    "Global: everything on its side (seasonal or not): vaults, gift chest, other characters",
];
const STAT_LABELS: [&str; 8] = ["Life", "Mana", "Attack", "Defense", "Speed", "Dexterity", "Vitality", "Wisdom"];
const HAS_IT: Color32 = fixed::GREEN;

fn plain(lines: &[&str]) -> Vec<widgets::TipLine> {
    lines.iter().map(|l| vec![(l.to_string(), None, false)]).collect()
}

fn click_hint(chosen: bool) -> widgets::TipLine {
    vec![(format!("Click to {}", if chosen { "stop aiming for it" } else { "aim for it" }), Some(fixed::MUTED), false)]
}

/// A small gold padlock (Java `withLock`), its top-left at `at`.
fn padlock(painter: &egui::Painter, at: egui::Pos2) {
    let gold = Color32::from_rgb(0xE5, 0xB2, 0x3A);
    painter.circle_stroke(at + vec2(5.0, 5.0), 3.0, Stroke::new(1.6, gold));
    painter.rect_filled(Rect::from_min_size(at + vec2(1.0, 5.5), vec2(8.0, 6.5)), 1.0, gold);
    painter.rect_filled(Rect::from_min_size(at + vec2(4.5, 7.5), vec2(1.2, 2.5)), 0.0, Color32::from_rgb(0x4A, 0x32, 0x05));
}

/// Java `enchantTip`: name and group, what it does; with stats chosen whether this copy has it, else how rare a
/// reroll gives it.
fn enchant_tip(en: &Enchantments, item: &lg_gamedata::items::Item, e: usize, owned: Option<bool>) -> Vec<widgets::TipLine> {
    let x = en.get(e);
    let mut lines = vec![vec![(x.name.clone(), None, true), (format!(" {}", x.kind_label()), Some(fixed::MUTED), false)]];
    if !x.description.is_empty() {
        lines.push(vec![(x.description.clone(), None, false)]);
    }
    match owned {
        Some(true) => lines.push(vec![("This copy has it: lock it when rerolling".into(), Some(HAS_IT), false)]),
        Some(false) => {
            let total: f64 = en.rollable_on(item).iter().map(|o| en.weight_on(item, *o)).sum();
            let w = en.weight_on(item, e);
            if w > 0.0 && total > 0.0 {
                lines.push(vec![(
                    format!("Reroll for it: about 1 in {} rerolls of a slot give exactly this", num((total / w).round() as i64)),
                    None,
                    false,
                )]);
            }
        }
        None => {}
    }
    lines
}

fn signed(n: i64) -> String {
    format!("{}{n}", if n >= 0 { "+" } else { "" })
}

/// Java `ThreeWaySwitch`'s slide: 160 ms, easing out.
const SLIDE_S: f64 = 0.16;

/// The outline of a pill (a rectangle with half-round ends), clockwise from the top right.
fn pill_outline(r: Rect) -> Vec<egui::Pos2> {
    let rad = r.height() / 2.0;
    let mut pts = Vec::with_capacity(50);
    for (cx, from) in [(r.right() - rad, -90.0f32), (r.left() + rad, 90.0f32)] {
        for k in 0..=24 {
            let a = (from + 180.0 * k as f32 / 24.0).to_radians();
            pts.push(pos2(cx + rad * a.cos(), r.center().y + rad * a.sin()));
        }
    }
    pts
}

/// A convex shape filled from `center` out to its outline in `rings` steps, each point coloured by `color`. Its
/// edge is smoothed as egui smooths its own shapes: a ring 1 px (of the screen) outside fading to nothing, so the
/// gradients do not show stair-stepped edges.
fn fan(painter: &egui::Painter, center: egui::Pos2, outline: &[egui::Pos2], rings: usize, color: impl Fn(egui::Pos2) -> Color32) {
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(center, color(center));
    let n = outline.len() as u32;
    for k in 1..=rings {
        let t = k as f32 / rings as f32;
        for q in outline {
            let v = center + (*q - center) * t;
            mesh.colored_vertex(v, color(v));
        }
    }
    // the feather: each outline point moved outwards (along the outline's normal there) to transparent
    let px = 1.0 / painter.pixels_per_point();
    for i in 0..outline.len() {
        let prev = outline[(i + outline.len() - 1) % outline.len()];
        let next = outline[(i + 1) % outline.len()];
        let tangent = (next - prev).normalized();
        let mut normal = egui::vec2(tangent.y, -tangent.x);
        if normal.dot(outline[i] - center) < 0.0 {
            normal = -normal;
        }
        mesh.colored_vertex(outline[i] + normal * px, Color32::TRANSPARENT);
    }
    for i in 0..n {
        let j = (i + 1) % n;
        mesh.add_triangle(0, 1 + i, 1 + j);
        for k in 1..rings as u32 {
            let (a, b) = (1 + (k - 1) * n, 1 + k * n);
            mesh.add_triangle(a + i, b + i, b + j);
            mesh.add_triangle(a + i, b + j, a + j);
        }
        let (a, b) = (1 + (rings as u32 - 1) * n, 1 + rings as u32 * n);
        mesh.add_triangle(a + i, b + i, b + j);
        mesh.add_triangle(a + i, b + j, a + j);
    }
    painter.add(egui::Shape::mesh(mesh));
}

fn circle_outline(c: egui::Pos2, r: f32) -> Vec<egui::Pos2> {
    (0..64).map(|k| c + egui::Vec2::angled(k as f32 / 64.0 * std::f32::consts::TAU) * r).collect()
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let m = |x: u8, y: u8| (f32::from(x) + (f32::from(y) - f32::from(x)) * t).round() as u8;
    let (a, b) = (a.to_srgba_unmultiplied(), b.to_srgba_unmultiplied());
    Color32::from_rgba_unmultiplied(m(a[0], b[0]), m(a[1], b[1]), m(a[2], b[2]), m(a[3], b[3]))
}

/// Java `ThreeWaySwitch`: a recessed pill (darker at the top), gold as the knob moves on, dots for the stops, a
/// brushed metal knob with a soft shadow. `knob`: where the knob is, in stops (between two while it slides).
fn switch(painter: &egui::Painter, r: Rect, stops: usize, stop: usize, knob: f32, p: &Palette) {
    let (w, h) = (r.width() - 1.0, r.height() - 1.0);
    let o = r.min;
    let at = |x: f32, y: f32| o + vec2(x, y);
    let track = Rect::from_min_size(at(0.5, 0.5), vec2(w, h));
    let outline = pill_outline(track);
    let (top, bottom) = (Color32::from_rgb(0x2A, 0x26, 0x20), Color32::from_rgb(0x5A, 0x54, 0x4A));
    fan(painter, track.center(), &outline, 1, |q| lerp_color(top, bottom, (q.y - o.y) / h));
    if knob > 0.0 {
        let level = (knob / (stops - 1) as f32).min(1.0);
        let g = p.gold;
        painter.rect_filled(track, h / 2.0, Color32::from_rgba_unmultiplied(g.r(), g.g(), g.b(), (150.0 * level) as u8));
    }
    // the inner shadow at the top
    let inner = Rect::from_min_size(at(1.5, 1.5), vec2(w - 2.0, h - 2.0));
    let shade = Color32::from_rgba_unmultiplied(0, 0, 0, 120);
    let clear = Color32::from_rgba_unmultiplied(0, 0, 0, 0);
    fan(painter, inner.center(), &pill_outline(inner), 2, |q| lerp_color(shade, clear, (q.y - o.y) / (h / 2.0)));
    // the rim: dark at the top, light at the bottom
    let (rim_top, rim_bottom) = (Color32::from_rgba_unmultiplied(0, 0, 0, 160), Color32::from_rgba_unmultiplied(255, 255, 255, 90));
    let rim =
        egui::epaint::PathStroke::new_uv(1.2, move |r: Rect, q: egui::Pos2| lerp_color(rim_top, rim_bottom, (q.y - r.top()) / r.height()));
    painter.add(egui::epaint::PathShape::closed_line(outline.clone(), rim));
    let pad = 3.0;
    let d = h - 2.0 * pad;
    let span = w - 2.0 * pad - d;
    let gaps = (stops - 1) as f32;
    for i in 0..stops {
        let c = at(pad + span * i as f32 / gaps + d / 2.0, h / 2.0 + 0.5);
        let on = i == stop && (knob - stop as f32).abs() < 1e-4;
        painter.circle_filled(c, 2.0, if on { p.gold } else { Color32::from_white_alpha(70) });
    }
    // the knob
    let (x, y) = (pad + span * knob / gaps, pad + 0.5);
    let c = at(x + d / 2.0, y + d / 2.0);
    painter.circle_filled(c + vec2(1.0, 2.0), d / 2.0, Color32::from_black_alpha(90));
    let disc = circle_outline(c, d / 2.0);
    let focus = at(x + d * 0.38, y + d * 0.32);
    let (c0, c1, c2) = (Color32::from_rgb(0xFB, 0xFB, 0xFB), Color32::from_rgb(0xCF, 0xCF, 0xCF), Color32::from_rgb(0x8C, 0x8C, 0x8C));
    fan(painter, focus, &disc, 8, |q| {
        let t = (q - focus).length() / (d * 0.75);
        if t < 0.45 { lerp_color(c0, c1, t / 0.45) } else { lerp_color(c1, c2, (t - 0.45) / 0.55) }
    });
    // brushed: fine rings around the centre
    let mut rr = 2.0f32;
    while rr < d / 2.0 {
        let a = (25.0 + 20.0 * rr.sin()) as u8;
        painter.circle_stroke(c, rr, Stroke::new(0.6, Color32::from_rgba_unmultiplied(255, 255, 255, a)));
        rr += 1.6;
    }
    // light across the top left, darker to the bottom right, as on turned metal
    let (lit, dark) = (Color32::from_rgba_unmultiplied(255, 255, 255, 110), Color32::from_rgba_unmultiplied(0, 0, 0, 70));
    let start = at(x, y);
    fan(painter, c, &disc, 1, |q| lerp_color(lit, dark, ((q.x - start.x) + (q.y - start.y)) / (2.0 * d)));
    painter.circle_stroke(c, d / 2.0, Stroke::new(1.2, Color32::from_rgb(0x6E, 0x6E, 0x6E)));
    painter.circle_stroke(c, d / 2.0 - 1.2, Stroke::new(1.2, Color32::from_rgba_unmultiplied(255, 255, 255, 150)));
}

/// A label with a switch at the right end of the column. A click slides the knob on to the next stop (Java
/// `ThreeWaySwitch.advance`); true once it got there, so the change follows the slide as in Java.
fn switch_row(
    ui: &Ui,
    p: &Palette,
    at: egui::Pos2,
    w: f32,
    label: &str,
    stops: usize,
    stop: usize,
    id: &str,
    tip: &[widgets::TipLine],
) -> (f32, bool) {
    let painter = ui.painter();
    let sw = if stops == 2 { 58.0 } else { 92.0 };
    let h = 30.0;
    let r = Rect::from_min_size(at, vec2(w, h));
    jtext::draw(painter, pos2(at.x, r.center().y), Align2::LEFT_CENTER, label, &text(18.0), p.ink);
    let clicked =
        ui.interact(r, ui.scope_id().with(("switch", id)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).clicked();
    let slide_id = egui::Id::unique(("lg-switch-slide", id));
    let now = ui.input(|i| i.time);
    let mut slide = ui.ctx().data(|d| d.get_temp::<(f32, usize, f64)>(slide_id));
    if clicked && slide.is_none() {
        let s = (stop as f32, (stop + 1) % stops, now);
        ui.ctx().data_mut(|d| d.insert_temp(slide_id, s));
        slide = Some(s);
    }
    let mut knob = stop as f32;
    let mut arrived = false;
    if let Some((from, to, start)) = slide {
        let t = ((now - start) / SLIDE_S).clamp(0.0, 1.0) as f32;
        let eased = 1.0 - (1.0 - t).powi(3);
        knob = from + (to as f32 - from) * eased;
        if t >= 1.0 {
            ui.ctx().data_mut(|d| d.remove_temp::<(f32, usize, f64)>(slide_id));
            arrived = true;
        } else {
            ui.ctx().request_repaint();
        }
    }
    switch(painter, Rect::from_min_size(pos2(r.right() - sw, r.top()), vec2(sw, h)), stops, stop, knob, p);
    widgets::rich_tip(ui, r, ("switch-tip", id), tip);
    (h, arrived)
}

/// Java `StatBox`: a dark framed box (gold-tinted when aimed for), the name above "x (+y)" (green up, orange down).
fn stat_box(
    ui: &Ui,
    p: &Palette,
    r: Rect,
    name: &str,
    value: &str,
    ench: &str,
    trend: i32,
    chosen: bool,
    tip: &[widgets::TipLine],
) -> bool {
    let painter = ui.painter();
    let g = p.gold;
    painter.rect_filled(
        r.shrink(3.0),
        0.0,
        if chosen { Color32::from_rgba_unmultiplied(g.r(), g.g(), g.b(), 55) } else { Color32::from_black_alpha(70) },
    );
    let frame = if chosen { g } else { Color32::from_white_alpha(40) };
    let (w, h, c) = (r.width(), r.height(), 8.0);
    for (x, y, ww, hh) in
        [(c, 0.0, w - 2.0 * c, 3.0), (c, h - 3.0, w - 2.0 * c, 3.0), (0.0, c, 3.0, h - 2.0 * c), (w - 3.0, c, 3.0, h - 2.0 * c)]
    {
        painter.rect_filled(Rect::from_min_size(pos2(r.left() + x, r.top() + y), vec2(ww, hh)), 0.0, frame);
    }
    let f = text(18.0);
    let big = text((18.0f32 * 1.15).floor());
    jtext::draw(painter, pos2(r.center().x, r.top() + 6.0), Align2::CENTER_TOP, name, &f, if chosen { g } else { fixed::MUTED });
    let tw = jtext::width(painter, value, &big) + jtext::width(painter, ench, &big);
    let x = r.center().x - (tw / 2.0).floor();
    let y = r.top() + 6.0 + jtext::height(painter, &f);
    let a = jtext::draw(
        painter,
        pos2(x, y),
        Align2::LEFT_TOP,
        value,
        &big,
        if trend > 0 {
            UP
        } else if trend < 0 {
            DOWN
        } else {
            Color32::WHITE
        },
    );
    if !ench.is_empty() {
        jtext::draw(painter, pos2(a.right(), y), Align2::LEFT_TOP, ench, &big, OTHER_STAT);
    }
    let clicked =
        ui.interact(r, ui.scope_id().with(("stat", name)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).clicked();
    widgets::rich_tip(ui, r, ("stat-tip", name), tip);
    clicked
}

/// The box's content; gives its height and what was clicked.
pub fn suggested(
    ui: &mut Ui,
    p: &Palette,
    app: &App,
    icons: &mut Icons,
    rarity: &Rarity,
    en: &Enchantments,
    c: &Char,
    settings: &Settings,
    picks: &[Pick; 4],
    s: &Summary,
    r: Rect,
) -> (f32, Option<Change>) {
    let painter = ui.painter().clone();
    let ctx = ui.ctx().clone();
    let data = app.world.data();
    let chosen = |k: &str| settings.chosen.iter().any(|x| x == k);
    let mut change = None;
    // the stats column: as wide as two boxes for the widest value
    let big = text((18.0f32 * 1.15).floor());
    let box_w = jtext::width(&painter, "9,999 (+999) (+99)", &big) + 30.0;
    let box_h = jtext::height(&painter, &text(18.0)) + jtext::height(&painter, &big) + 14.0;
    let col_w = 2.0 * box_w + 6.0;
    let x = r.left();
    let mut y = r.top();
    let mut scope_tip = plain(&SCOPE_TIPS);
    scope_tip.push(vec![(format!("Click to switch to {}", SCOPES[(settings.scope + 1) % 3]), Some(fixed::MUTED), false)]);
    let (h, clicked) =
        switch_row(ui, p, pos2(x, y), col_w, &format!("Scope: {}", SCOPES[settings.scope]), 3, settings.scope, "scope", &scope_tip);
    if clicked {
        change = Some(Change::Scope);
    }
    y += h + 4.0;
    let replaced_tip = plain(&[
        "In scope current: also list the enchantments the suggestion would replace (struck through),",
        "so you can lock them to keep them. Off: they show while you hover the item",
    ]);
    let (h, clicked) =
        switch_row(ui, p, pos2(x, y), col_w, "Show deleted enchants", 2, usize::from(settings.show_replaced), "replaced", &replaced_tip);
    if clicked {
        change = Some(Change::ShowReplaced);
    }
    y += h + 4.0;
    let (h, clicked) = switch_row(
        ui,
        p,
        pos2(x, y),
        col_w,
        &format!("Second chance: {}", SECOND[settings.second_chance]),
        3,
        settings.second_chance,
        "second",
        &plain(&[
            "Items that save you when your life runs low (local and global scope)",
            "Off: nothing preferred",
            "Ami: an Amulet of Restoration as the ring",
            "Ami + armor: also an armor with a low-life proc, like the Protective Matrix",
            "Only if you have one on the character's side",
        ]),
    );
    if clicked {
        change = Some(Change::SecondChance);
    }
    y += h + 4.0;
    let (h, clicked) = switch_row(
        ui,
        p,
        pos2(x, y),
        col_w,
        "Suggest enchants",
        2,
        usize::from(settings.suggest_enchants),
        "suggest-enchants",
        &plain(&[
            "On: rerolls are planned for the suggested items (gold: reroll for it, green: the copy has it)",
            "Off: the items are chosen and rated with the enchantments each copy already has",
        ]),
    );
    if clicked {
        change = Some(Change::SuggestEnchants);
    }
    y += h;
    if s.has_weapon {
        y += 4.0;
        let row_h = 24.0;
        let n = jtext::draw(&painter, pos2(x, y + row_h / 2.0), Align2::LEFT_CENTER, "Enemy DEF", &small(), fixed::MUTED);
        let vw = jtext::width(&painter, "100", &text(18.0)) + 2.0;
        let (t0, t1) = (n.right() + 6.0 + 8.0, x + col_w - vw - 6.0 - 8.0);
        // dragged (or clicked) along its track, in steps of 5 (Java's slider snaps to its ticks); set on letting go
        let track = Rect::from_min_max(pos2(t0 - 8.0, y), pos2(t1 + 8.0, y + row_h));
        let resp =
            ui.interact(track, ui.scope_id().with("enemy-def"), Sense::click_and_drag()).on_hover_cursor(egui::CursorIcon::PointingHand);
        let drag_id = egui::Id::unique("lg-enemy-def");
        if (resp.dragged() || resp.clicked() || resp.drag_started())
            && let Some(q) = ui.input(|i| i.pointer.interact_pos())
        {
            let v = (((q.x - t0) / (t1 - t0)).clamp(0.0, 1.0) * 100.0 / 5.0).round() * 5.0;
            ui.ctx().data_mut(|d| d.insert_temp(drag_id, v));
        }
        if (resp.drag_stopped() || resp.clicked())
            && let Some(v) = ui.ctx().data_mut(|d| d.remove_temp::<f32>(drag_id))
        {
            change = Some(Change::EnemyDef(f64::from(v)));
        }
        let def = ui.ctx().data(|d| d.get_temp::<f32>(drag_id)).map_or(settings.enemy_def as i32, |v| v as i32);
        jtext::draw(&painter, pos2(x + col_w - vw, y + row_h / 2.0), Align2::LEFT_CENTER, &def.to_string(), &text(18.0), p.ink);
        let def_tip = plain(&[
            "The enemy's defense: it comes off every shot (armor piercing ones ignore it, and at least 10% of a shot always gets through).",
            "Many small shots lose more to it than a few big ones.",
        ]);
        widgets::rich_tip(ui, Rect::from_min_max(pos2(x, y), pos2(x + col_w, y + row_h)), "enemy-def-tip", &def_tip);
        let g = p.gold;
        painter.line_segment(
            [pos2(t0, y + row_h / 2.0), pos2(t1, y + row_h / 2.0)],
            Stroke::new(4.0, Color32::from_rgba_unmultiplied(g.r(), g.g(), g.b(), 128)),
        );
        let kx = t0 + (t1 - t0) * def as f32 / 100.0;
        painter.line_segment([pos2(t0, y + row_h / 2.0), pos2(kx, y + row_h / 2.0)], Stroke::new(4.0, g));
        painter.circle_filled(pos2(kx, y + row_h / 2.0), 8.0, g);
        y += row_h;
    }
    y += 6.0;
    let g = p.gold;
    painter.rect_filled(Rect::from_min_size(pos2(x, y), vec2(col_w, 2.0)), 0.0, Color32::from_rgba_unmultiplied(g.r(), g.g(), g.b(), 170));
    y += 2.0 + 18.0;
    // HP/s | MP/s
    let one = |v: f64| format!("{v:.1}");
    let trend = |d: f64| {
        if d > 1e-9 {
            1
        } else if d < -1e-9 {
            -1
        } else {
            0
        }
    };
    let regen_tips = [
        ("Life regenerated per second", "1 + 0.24 × VIT, plus flat and percentage regeneration enchantments (doubled out of combat)"),
        ("Mana regenerated per second", "0.5 + 0.12 × WIS, plus flat and percentage regeneration enchantments (doubled out of combat)"),
    ];
    for (k, (name, key)) in [("HP/s", lg_app::loadout::HP_REGEN), ("MP/s", lg_app::loadout::MP_REGEN)].iter().enumerate() {
        let d = s.regen[k] - s.regen_now[k];
        let tip = vec![
            vec![(regen_tips[k].0.to_string(), None, true), (" in combat".to_string(), None, false)],
            vec![(regen_tips[k].1.to_string(), None, false)],
            vec![(format!("Now: {}", one(s.regen_now[k])), None, false)],
            click_hint(chosen(key)),
        ];
        let v = format!("{} ({}{})", one(s.regen[k]), if d >= 0.0 { "+" } else { "" }, one(d));
        if stat_box(
            ui,
            p,
            Rect::from_min_size(pos2(x + k as f32 * (box_w + 6.0), y), vec2(box_w, box_h)),
            name,
            &v,
            "",
            trend(d),
            chosen(key),
            &tip,
        ) {
            change = Some(Change::Toggle(key.to_string()));
        }
    }
    y += box_h;
    if s.has_weapon {
        y += 6.0;
        let d = (s.dps.round() - s.dps_now.round()) as i64;
        let v = format!("{} ({})", num(s.dps.round() as i64), signed(d));
        // Java `dpsBox`'s tip: every subattack, and the DPS without defense
        let (att, dex) = (s.att, s.dex);
        let def = settings.enemy_def;
        let weapon = data.items.item(picks[0].id);
        let lines = lg_app::loadout::lines(en, data, weapon, &s.finals[0]);
        let mut tip = vec![
            vec![("Damage per second".to_string(), None, true), (format!(" against {} DEF", def as i64), None, false)],
            vec![(
                format!(
                    "Attack {}: damage ×{:.2}   Dexterity {}: {:.2} attacks/s",
                    att.round() as i64,
                    lg_app::loadout::damage_multiplier(att),
                    dex.round() as i64,
                    lg_app::loadout::attacks_per_second(dex)
                ),
                None,
                false,
            )],
        ];
        let wname = weapon.map_or(String::new(), |w| w.name.clone());
        for l in &lines {
            let from = if l.from == wname { format!("Shot {}", l.index + 1) } else { l.from.clone() };
            let mut t = format!(
                "{from}: {}{}{}, {:.2} shots/s",
                if l.count() > 1 { format!("{} × ", l.count()) } else { String::new() },
                num(l.per_shot(att, def).round() as i64),
                if l.armor_piercing() { " (armor piercing)" } else { "" },
                l.shots_per_second(dex)
            );
            if l.burst() > 0 {
                t += &format!(" in bursts of {}", l.burst());
            } else if (l.rate_of_fire() - 1.0).abs() > 1e-9 {
                t += &format!(" ({}% rate)", (l.rate_of_fire() * 100.0).round() as i64);
            }
            t += &format!(" = {}", num(l.dps(att, dex, def).round() as i64));
            tip.push(vec![(t, None, false)]);
        }
        if def > 0.0 {
            let open: f64 = lines.iter().map(|l| l.dps(att, dex, 0.0)).sum();
            tip.push(vec![(format!("Without defense: {}", num(open.round() as i64)), None, false)]);
        }
        tip.push(vec![(format!("Now: {}", num(s.dps_now.round() as i64)), None, false)]);
        tip.push(click_hint(chosen(lg_app::loadout::DPS)));
        if stat_box(
            ui,
            p,
            Rect::from_min_size(pos2(x, y), vec2(col_w, box_h)),
            "DPS",
            &v,
            "",
            trend(d as f64),
            chosen(lg_app::loadout::DPS),
            &tip,
        ) {
            change = Some(Change::Toggle(lg_app::loadout::DPS.into()));
        }
        y += box_h;
    }
    y += 14.0;
    for i in 0..8 {
        let (col, row) = (i % 2, i / 2);
        let items = s.suggested[i];
        let ench = (s.enchanted[i] + 1e-9).floor() as i64;
        let total = i64::from(s.max[i]) + i64::from(items) + ench;
        let v = format!("{} ({})", num(total), signed(i64::from(items)));
        let e = if ench == 0 { String::new() } else { format!(" ({})", signed(ench)) };
        let br = Rect::from_min_size(pos2(x + col as f32 * (box_w + 6.0), y + row as f32 * (box_h + 6.0)), vec2(box_w, box_h));
        // Java `statTable`'s tip: the class maximum, items, enchantments, compared with what it wears now
        let mut tip = vec![vec![(STAT_LABELS[i].to_string(), None, false)]];
        tip.push(vec![(format!("Class maximum {}", num(i64::from(s.max[i]))), None, false)]);
        tip.push(vec![(format!("Items {}", signed(i64::from(items))), None, false)]);
        let mut et = format!("Enchantments {:+.1}", s.enchanted[i]);
        if !settings.chosen.is_empty() {
            et += &format!(" with the suggested ones ({:+.1} as these copies are now)", s.have[i]);
        }
        tip.push(vec![(et, None, false)]);
        let diff = items - s.now[i];
        if diff != 0 {
            tip.push(vec![(format!("{} from items, compared with what it wears now", signed(i64::from(diff))), None, false)]);
        }
        tip.push(click_hint(chosen(STATS[i])));
        if stat_box(ui, p, br, STAT_SHORT[i], &v, &e, items.signum(), chosen(STATS[i]), &tip) {
            change = Some(Change::Toggle(STATS[i].into()));
        }
    }
    y += 4.0 * box_h + 3.0 * 6.0 + 14.0;
    let d = s.loot - s.loot_now;
    let v = format!("+{}% ({}{})", one(s.loot), if d >= 0.0 { "+" } else { "" }, one(d));
    let others: f64 =
        crate::characters::loot_bonus_parts(c, data, app.world.now()).iter().filter(|p| p.2 && p.0 != "Enchantments").map(|p| p.1).sum();
    let tip = vec![
        vec![("Loot bonus".to_string(), None, true)],
        vec![(format!("Enchantments {}%, crucible / ritual / boosts {}%", one(s.loot - others), one(others)), None, false)],
        vec![(format!("Now: {}%", one(s.loot_now)), None, false)],
        click_hint(chosen(lg_app::loadout::LOOT)),
    ];
    if stat_box(ui, p, Rect::from_min_size(pos2(x, y), vec2(box_w, box_h)), "LOOT", &v, "", trend(d), chosen(lg_app::loadout::LOOT), &tip) {
        change = Some(Change::Toggle(lg_app::loadout::LOOT.into()));
    }
    y += box_h;
    let stats_h = y - r.top();

    // the items column
    let ix = x + col_w + 12.0;
    let iw = r.right() - ix;
    let mut iy = r.top();
    // the item's name the biggest, its lines below smaller
    let small_f = crate::theme::head(14.0);
    let name_f = text(22.0);
    for (slot, pk) in picks.iter().enumerate() {
        if slot > 0 {
            iy += 6.0;
        }
        let it = data.items.item(pk.id);
        // the text first, to centre it on the icon's height
        let mut lines: Vec<Vec<(String, Color32, Option<i32>)>> = Vec::new();
        let name = if pk.id > 0 {
            it.map_or(format!("Item {}", pk.id), |i| i.name.clone())
        } else {
            format!("No {}", SLOT_NAMES[slot].to_lowercase())
        };
        let mut first = vec![(name, p.ink, None)];
        if !pk.r#where.is_empty() && pk.id > 0 {
            first.push((format!(" ({})", pk.r#where), fixed::MUTED, None));
        }
        lines.push(first);
        if let Some(it) = it {
            let (mut wanted, mut other) = (Vec::new(), Vec::new());
            for st in &it.on_equip {
                let Some(si) = STATS.iter().position(|x| *x == st.stat) else { continue };
                let want = chosen(&st.stat) || (chosen(lg_app::loadout::DPS) && (st.stat == "ATT" || st.stat == "DEX"));
                let t = format!("{}{} {}  ", if st.amount > 0 { "+" } else { "" }, st.amount, STAT_SHORT[si]);
                if want { wanted.push(t) } else { other.push(t) }
            }
            if !wanted.is_empty() || !other.is_empty() {
                lines.push(vec![(wanted.concat(), fixed::GREEN, Some(-1)), (other.concat(), OTHER_STAT, Some(-1))]);
            }
            // its share: the suggestion's DPS and life per second, less what they are without this item; each only
            // while you aim for it (its stat box clicked)
            let (dps, hp) = (s.share_dps[slot], s.share_hp_regen[slot]);
            let mut share = Vec::new();
            if chosen(lg_app::loadout::DPS) && s.has_weapon && dps.abs() >= 0.5 {
                share.push(format!("{}{} DPS", if dps > 0.0 { "+" } else { "" }, num(dps.round() as i64)));
            }
            if chosen(lg_app::loadout::HP_REGEN) && hp.abs() >= 0.05 {
                share.push(format!("{}{:.1} HP/s", if hp > 0.0 { "+" } else { "" }, hp));
            }
            if !share.is_empty() {
                lines.push(vec![(share.join("  ·  "), p.gold, Some(-1))]);
            }
        }
        // the enchantments: planned (green: it has it, gold: reroll for it), then what it keeps, then free slots;
        // in scope current those of the worn copy can be locked, and what the suggestion replaces shows struck
        // through (with "show deleted enchants" off only while the item is hovered)
        struct Chip {
            e: Option<usize>,
            text: String,
            color: Color32,
            tip: Vec<widgets::TipLine>,
            struck: bool,
            lockable: bool,
            hover_only: bool,
        }
        // any suggested copy's enchantments can be locked (kept by the suggestion, in every scope)
        let lockable_here = pk.id > 0 && pk.enchants.is_some();
        let locked_now: Vec<i32> = if lockable_here { settings.locked_enchants(c.char_id, slot) } else { Vec::new() };
        let lock_hint = |tip: &mut Vec<widgets::TipLine>, locked: bool| {
            tip.push(if locked {
                vec![("Locked:".to_string(), None, true), (" kept whatever you aim for. Click to unlock".to_string(), None, false)]
            } else {
                vec![(
                    "Click to lock it: the suggestion keeps it and counts it, and suggests nothing that clashes with it".to_string(),
                    Some(fixed::MUTED),
                    false,
                )]
            });
        };
        let mut chips: Vec<Chip> = Vec::new();
        let note =
            |t: String| Chip { e: None, text: t, color: fixed::MUTED, tip: Vec::new(), struck: false, lockable: false, hover_only: false };
        if let Some(item) = it.filter(|i| i.enchant_slots.is_some()) {
            let n = pk.slots(data);
            if pk.enchants.is_none() {
                if !settings.chosen.is_empty() {
                    chips.push(note("Rarity not known yet: open the vault or play the character that has it".into()));
                }
            } else if n == 0 {
                if !settings.chosen.is_empty() {
                    chips.push(note("Common: no enchantment slots".into()));
                }
            } else {
                // rerolls are suggested only with stats chosen and "Suggest enchants" on; else the copy as it is
                let planning = !settings.chosen.is_empty() && settings.suggest_enchants;
                let has: Vec<i32> = pk.enchants.clone().unwrap_or_default();
                let shown = if !planning { en.of(pk.enchants.as_deref()) } else { pk.plan.picks.clone() };
                for e in &shown {
                    let owned = has.contains(&en.get(*e).kind);
                    let color = if !planning {
                        fixed::MUTED
                    } else if owned {
                        HAS_IT
                    } else {
                        p.gold
                    };
                    let mut tip = enchant_tip(en, item, *e, (planning).then_some(owned));
                    let lockable = lockable_here && owned;
                    if lockable {
                        lock_hint(&mut tip, locked_now.contains(&en.get(*e).kind));
                    }
                    chips.push(Chip { e: Some(*e), text: en.get(*e).name.clone(), color, tip, struck: false, lockable, hover_only: false });
                }
                if planning {
                    // what the copy has and keeps in the slots left free
                    for e in &s.finals[slot] {
                        if shown.contains(e) {
                            continue;
                        }
                        let mut tip = enchant_tip(en, item, *e, None);
                        tip.push(vec![(
                            "Already on this copy: it stays in a slot the suggestion leaves free".into(),
                            Some(fixed::MUTED),
                            false,
                        )]);
                        if lockable_here {
                            lock_hint(&mut tip, locked_now.contains(&en.get(*e).kind));
                        }
                        let t = en.get(*e).name.clone();
                        chips.push(Chip {
                            e: Some(*e),
                            text: t,
                            color: fixed::MUTED,
                            tip,
                            struck: false,
                            lockable: lockable_here,
                            hover_only: false,
                        });
                    }
                    if lockable_here {
                        // what the copy has and the suggestion replaces
                        for e in en.of(pk.enchants.as_deref()) {
                            if s.finals[slot].contains(&e) {
                                continue;
                            }
                            let mut tip = enchant_tip(en, item, e, None);
                            tip.push(vec![("On this copy now: the suggestion replaces it".into(), Some(fixed::MUTED), false)]);
                            lock_hint(&mut tip, locked_now.contains(&en.get(e).kind));
                            let t = en.get(e).name.clone();
                            chips.push(Chip {
                                e: Some(e),
                                text: t,
                                color: fixed::MUTED,
                                tip,
                                struck: true,
                                lockable: true,
                                hover_only: !settings.show_replaced,
                            });
                        }
                    }
                }
                let free = n as i64 - if !planning { shown.len() } else { s.finals[slot].len() } as i64;
                if planning && free > 0 {
                    let rarity_name = ["common", "uncommon", "rare", "legendary", "divine"][n.min(4)];
                    let mut c = note(format!("{free} free slot{}", if free == 1 { "" } else { "s" }));
                    c.tip = plain(&[&format!(
                        "Nothing more raises the chosen stats on this {rarity_name} item: whatever these slots hold can stay"
                    )]);
                    chips.push(c);
                }
            }
        }
        let small_h = jtext::height(&painter, &small_f);
        let text_h_of = |chips_shown: usize| {
            let t = jtext::height(&painter, &name_f)
                + (lines.len() as f32 - 1.0) * small_h
                + chips_shown as f32 * small_h
                + if chips_shown == 0 { 0.0 } else { 2.0 };
            (t, t.max(52.0))
        };
        // the replaced ones that show only on hover: shown while the pointer is on the item's row
        // they roll out while the pointer is on the row (as tall as it is drawn, the rolled out lines included)
        // and roll back in after it left
        let always = chips.iter().filter(|c| !c.hover_only).count();
        let (text_h0, h0) = text_h_of(always);
        let reveal_id = ui.scope_id().with(("hover-chips", slot));
        let drawn_h = ctx.data(|d| d.get_temp::<f32>(reveal_id)).unwrap_or(h0).max(h0);
        let hovered = ui.rect_contains_pointer(Rect::from_min_size(pos2(ix, iy), vec2(iw, drawn_h)));
        let reveal = ctx.animate_bool_with_time(reveal_id, hovered, 0.18);
        let reveal = reveal * reveal * (3.0 - 2.0 * reveal); // eased at both ends
        if reveal <= 0.0 {
            chips.retain(|c| !c.hover_only);
        }
        let (text_h1, h1) = text_h_of(chips.len());
        let (text_h, h) = (text_h0 + (text_h1 - text_h0) * reveal, h0 + (h1 - h0) * reveal);
        ctx.data_mut(|d| d.insert_temp(reveal_id, h));
        let icon_r = Rect::from_min_size(pos2(ix, iy + (h - 52.0) / 2.0), vec2(60.0, 52.0));
        if pk.id > 0 {
            let t = icons.object(&ctx, data, pk.id, 48);
            let ir = Rect::from_min_size(pos2(icon_r.right() - 48.0, icon_r.center().y - 24.0), vec2(48.0, 48.0));
            if let Some(t) = t {
                painter.image(t.id(), ir, uv(), Color32::WHITE);
            }
            enchant_badge(&painter, rarity, ir.right_bottom() - vec2(1.0, 1.0), pk.enchants.as_ref().map_or(0, Vec::len), 6.0);
            widgets::item_tip(ui, ir, data, pk.id, pk.enchants.as_deref(), SLOT_NAMES[slot]);
            widgets::item_link(ui, data, ir, pk.id);
        }
        let tx = ix + 60.0 + 10.0;
        let tw = iw - 70.0;
        let mut ty = iy + (h - text_h) / 2.0;
        let item_locked = pk.id > 0 && settings.locked_item(c.char_id, slot) == pk.id;
        for (k, line) in lines.iter().enumerate() {
            let f = if k == 0 { &name_f } else { &small_f };
            let mut lx = tx;
            if k == 0 && item_locked {
                padlock(&painter, pos2(lx, ty + (jtext::height(&painter, f) - 12.0) / 2.0));
                lx += 14.0;
            }
            let start = lx;
            for (j, (t, color, _)) in line.iter().enumerate() {
                let fitted = widgets::fit(&painter, t, f, (tx + tw - lx).max(10.0));
                // a locked item's name in gold
                let color = if k == 0 && j == 0 && item_locked { fixed::GOLD } else { *color };
                let a = jtext::draw(&painter, pos2(lx, ty), Align2::LEFT_TOP, &fitted, f, color);
                lx = a.right();
            }
            if k == 0 && pk.id > 0 {
                // click the name: this item stays in the slot (local and global scope)
                let nr = Rect::from_min_max(pos2(tx, ty), pos2(lx.max(start), ty + jtext::height(&painter, f)));
                let resp = ui
                    .interact(nr, ui.scope_id().with(("item-lock", slot)), Sense::click())
                    .on_hover_cursor(egui::CursorIcon::PointingHand);
                let tip = if item_locked {
                    vec![
                        vec![
                            ("Locked:".to_string(), None, true),
                            (" this item stays in the slot whatever you aim for.".to_string(), None, false),
                        ],
                        vec![("Click to unlock".to_string(), Some(fixed::MUTED), false)],
                    ]
                } else {
                    vec![vec![
                        ("Click to lock this item in the slot".to_string(), Some(fixed::MUTED), false),
                        (" (local and global scope)".to_string(), None, false),
                    ]]
                };
                widgets::rich_tip(ui, nr, ("item-lock-tip", slot), &tip);
                if resp.clicked() {
                    change = Some(Change::ItemLock(slot, pk.id));
                }
            }
            ty += jtext::height(&painter, f);
        }
        if !chips.is_empty() {
            ty += 1.0;
        }
        for (k, chip) in chips.iter().enumerate() {
            let mut lx = tx;
            // a rolling line: only the part rolled out shows, fading in
            let line_h = if chip.hover_only { small_h * reveal } else { small_h };
            let painter = if chip.hover_only {
                painter.with_clip_rect(Rect::from_min_max(pos2(tx - 20.0, ty), pos2(tx + tw + 20.0, ty + line_h)))
            } else {
                painter.clone()
            };
            let kind = chip.e.map(|e| en.get(e).kind);
            let locked = chip.lockable && kind.is_some_and(|k| locked_now.contains(&k));
            if locked {
                padlock(&painter, pos2(lx, ty + (small_h - 12.0) / 2.0));
                lx += 13.0;
            }
            if let Some(kd) = kind
                && let Some((sheet, idx)) = data.enchant_looks.get(&kd).and_then(|l| l.texture.clone())
            {
                if let Some(tex) = icons.sheet(&ctx, &sheet, idx, 16) {
                    painter.image(
                        tex.id(),
                        Rect::from_min_size(pos2(lx, ty + (small_h - 16.0) / 2.0), vec2(16.0, 16.0)),
                        uv(),
                        Color32::WHITE,
                    );
                }
                lx += 19.0;
            }
            let color = if locked { fixed::GOLD } else { chip.color };
            let color = if chip.hover_only { color.gamma_multiply(reveal) } else { color };
            let a = jtext::draw(
                &painter,
                pos2(lx, ty),
                Align2::LEFT_TOP,
                &widgets::fit(&painter, &chip.text, &small_f, tx + tw - lx),
                &small_f,
                color,
            );
            if chip.struck {
                painter.line_segment([pos2(a.left(), a.center().y), pos2(a.right(), a.center().y)], Stroke::new(1.0, color));
            }
            let cr = Rect::from_min_max(pos2(tx, ty), pos2(a.right(), ty + line_h));
            widgets::rich_tip(ui, cr, ("chip", slot, k), &chip.tip);
            if chip.lockable
                && line_h > small_h * 0.9
                && let Some(kd) = kind
                && ui
                    .interact(cr, ui.scope_id().with(("chip-lock", slot, k)), Sense::click())
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .clicked()
            {
                change = Some(Change::EnchantLock(slot, kd));
            }
            ty += line_h;
        }
        iy += h;
    }
    let _ = c;
    (stats_h.max(iy - r.top()), change)
}
