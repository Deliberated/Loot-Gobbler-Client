//! Profile tab, Quests page (Java `MissionsView`): the battle pass missions and the Tinkerer's quests. Cards on top
//! (the Tinkerer, a gold divider, each battle pass with its picture) pick what is listed; the list stands in the
//! middle at its own width: claimable missions first (in a gold frame), the Tinkerer's quests, then the rest. Each
//! row: the mission's picture, who can do it, its name and battle pass, the objectives with bars, a brush arrow (a
//! swoosh gathering several objectives) to the rewards: items, choices side by side with gold dividers in as many
//! lines as the objectives are tall, the battle pass points (BXP) below.
//!
//! Not yet: Java's dotted arrows from the missions a locked one needs, and downloading a new battle pass's picture
//! (they are read from `season-art`, where the Java app keeps them).

use crate::jtext;
use crate::profile::{Ctx, num};
use crate::theme::{Palette, fixed};
use crate::widgets::{self, small, text};
use egui::{Align2, Color32, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};
use lg_app::missions::{Mission, Status, View};
use std::collections::HashSet;
use std::sync::OnceLock;

const CLAIMABLE: Color32 = Color32::from_rgb(0xF2, 0xC2, 0x30);
const CLAIMED: Color32 = Color32::from_rgb(0x5F, 0xB0, 0x4A);
const NOT_FOR_YOU: Color32 = Color32::from_rgb(0xE0, 0x67, 0x9C);
const ONE_OF: Color32 = Color32::from_rgb(0x3F, 0xC4, 0xD6);
const TINKERER_KEY: &str = "tinkerer";
/// The Tinkerer's object type (his sprite).
const TINKERER: i32 = 0x1754;
// Java's columns: icon, kind, name, objectives, the arrow, rewards
const W_ICON: f32 = 44.0;
const W_KIND: f32 = 92.0;
const W_NAME: f32 = 230.0;
const W_GOAL: f32 = 218.0;
const W_LINK: f32 = 58.0;
const W_REWARD: f32 = 330.0;
const ROW_W: f32 = W_ICON + W_KIND + W_NAME + W_GOAL + W_LINK + W_REWARD + 16.0;
const W_BAR: f32 = 210.0;
const REWARD_ICON: f32 = 34.0;
const CARD_W: f32 = 230.0;
const ART_H: f32 = 110.0;
const ROW_GAP: f32 = 10.0;
const LINE_H: f32 = 18.0;
const BAR_BLOCK: f32 = 18.0 + 14.0 + 3.0;

/// The app's folder (for `season-art`), set when the window is made.
static FOLDER: OnceLock<std::path::PathBuf> = OnceLock::new();

pub fn set_folder(dir: &std::path::Path) {
    let _ = FOLDER.set(dir.to_path_buf());
}

pub struct QuestsView {
    /// false: only the missions your character makes progress on.
    all_missions: bool,
    /// The cards not picked (the Tinkerer at first, as in Java).
    hidden: HashSet<String>,
}

impl Default for QuestsView {
    fn default() -> Self {
        QuestsView { all_missions: false, hidden: [TINKERER_KEY.to_string()].into_iter().collect() }
    }
}

fn span(seconds: i64) -> String {
    let (d, h, m) = (seconds / 86_400, seconds % 86_400 / 3600, seconds % 3600 / 60);
    if d > 0 {
        format!("{d}d {h}h")
    } else if h > 0 {
        format!("{h}h {m}m")
    } else {
        format!("{}m", m.max(1))
    }
}

fn ago(ms: i64, now: i64) -> String {
    let s = (now - ms) / 1000;
    if s < 60 { "just now".into() } else { format!("{} ago", span(s)) }
}

/// Java's `String.hashCode`, as `season-art` names its files.
fn java_hash(s: &str) -> i32 {
    s.encode_utf16().fold(0i32, |h, c| h.wrapping_mul(31).wrapping_add(i32::from(c)))
}

/// A battle pass's picture from `season-art` (as the Java app keeps them), loaded once.
fn art(ctx: &egui::Context, s: &lg_app::missions::Season) -> Option<egui::TextureHandle> {
    let url = s.game_data.as_ref()?.get("backgroundSprite")?.as_string().ok().filter(|u| u.starts_with("https://"))?;
    let ext = if url.to_lowercase().ends_with(".png") { "png" } else { "jpg" };
    let name = format!("{}-{:x}.{ext}", s.id, java_hash(&url) as u32);
    let id = egui::Id::unique(("lg-season-art", name.as_str()));
    if let Some(t) = ctx.data(|d| d.get_temp::<Option<egui::TextureHandle>>(id)) {
        return t;
    }
    let tex = FOLDER.get().and_then(|dir| std::fs::read(dir.join("season-art").join(&name)).ok()).and_then(|bytes| {
        let img = image::load_from_memory(&bytes).ok()?.to_rgba8();
        let size = [img.width() as usize, img.height() as usize];
        Some(ctx.load_texture(name.clone(), egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw()), egui::TextureOptions::LINEAR))
    });
    ctx.data_mut(|d| d.insert_temp(id, tex.clone()));
    tex
}

/// Java `kind`: who can do it.
fn kind(m: &Mission) -> (&'static str, Color32) {
    match m.participants {
        2 => ("SEASONAL", Color32::from_rgb(0x3F, 0xC4, 0x8A)),
        4 => ("CRUCIBLE", Color32::from_rgb(0xE0, 0x57, 0x4F)),
        _ => ("REGULAR", Color32::from_rgb(0xE5, 0xB2, 0x3A)),
    }
}

/// An outlined tag, its left middle at `at`.
fn tag(painter: &egui::Painter, at: Pos2, t: &str, color: Color32) {
    let f = text(13.0);
    let w = jtext::width(painter, t, &f) + 12.0;
    let r = Rect::from_min_size(pos2(at.x, at.y - 9.0), vec2(w, 18.0));
    painter.rect_stroke(r, 3.0, Stroke::new(1.2, color), StrokeKind::Inside);
    jtext::draw(painter, r.center(), Align2::CENTER_CENTER, t, &f, color);
}

/// A brush stroke along a cubic curve, its width going from `w0` at the start through `wm` to `w1` at the end
/// (Java `brush`), with round ends.
fn brush(painter: &egui::Painter, c: [Pos2; 4], w0: f32, wm: f32, w1: f32, color: Color32) {
    let n = 40;
    let at = |t: f32| {
        let u = 1.0 - t;
        (c[0].to_vec2() * u * u * u + c[1].to_vec2() * 3.0 * u * u * t + c[2].to_vec2() * 3.0 * u * t * t + c[3].to_vec2() * t * t * t)
            .to_pos2()
    };
    let ease = |t: f32| t * t * (3.0 - 2.0 * t);
    let pts: Vec<Pos2> = (0..=n).map(|i| at(i as f32 / n as f32)).collect();
    let mut mesh = egui::Mesh::default();
    for (i, q) in pts.iter().enumerate() {
        let d = (pts[(i + 1).min(n)] - pts[i.saturating_sub(1)]).normalized();
        let t = i as f32 / n as f32;
        let w = if t < 0.5 { w0 + (wm - w0) * ease(t * 2.0) } else { wm + (w1 - wm) * ease((t - 0.5) * 2.0) };
        let nrm = vec2(-d.y, d.x) * w / 2.0;
        mesh.colored_vertex(*q + nrm, color);
        mesh.colored_vertex(*q - nrm, color);
        if i > 0 {
            let k = (2 * i) as u32;
            mesh.add_triangle(k - 2, k - 1, k);
            mesh.add_triangle(k - 1, k + 1, k);
        }
    }
    painter.add(egui::Shape::mesh(mesh));
    painter.circle_filled(pts[0], w0 / 2.0, color);
    painter.circle_filled(pts[n], w1 / 2.0, color);
}

/// A slim arrowhead with a hollow back, its tip at (x, y), pointing right (Java `arrowHead`).
fn arrow_head(painter: &egui::Painter, x: f32, y: f32, size: f32, color: Color32) {
    let (bx, by, notch) = (x - size * 1.25, size * 0.8, x - size * 0.8);
    painter.add(egui::Shape::convex_polygon(vec![pos2(x, y), pos2(bx, y - by), pos2(notch, y)], color, Stroke::NONE));
    painter.add(egui::Shape::convex_polygon(vec![pos2(x, y), pos2(notch, y), pos2(bx, y + by)], color, Stroke::NONE));
}

/// The gold divider between choices: a slim line with small end caps.
fn divider(painter: &egui::Painter, x: f32, y0: f32, y1: f32, color: Color32) {
    let st = Stroke::new(2.0, color);
    painter.line_segment([pos2(x, y0), pos2(x, y1)], st);
    painter.line_segment([pos2(x - 3.0, y0), pos2(x + 3.0, y0)], st);
    painter.line_segment([pos2(x - 3.0, y1), pos2(x + 3.0, y1)], st);
}

/// The objectives' parts: lines above bars ("One of these:"), and the objectives with bars.
enum Goal {
    Line(String, Color32),
    Bar { text: String, have: i32, need: i32, met: bool },
}

fn goals(v: &View) -> Vec<Goal> {
    let m = v.mission;
    let mut out = Vec::new();
    let done = matches!(v.status, Status::Claimed | Status::Cooldown);
    for w in m.worn() {
        out.push(Goal::Line(format!("Requires {w} equipped"), CLAIMABLE));
    }
    if m.conds.is_empty()
        && let Some(d) = m.desc.as_deref().filter(|d| !d.trim().is_empty())
    {
        out.push(Goal::Line(d.to_string(), fixed::MUTED));
    }
    let groups = m.groups();
    if groups.len() > 1 {
        let choices = groups.iter().any(|g| g.len() > 1);
        out.push(Goal::Line(if choices { "All of these groups:" } else { "All of these:" }.into(), CLAIMABLE));
    }
    for g in &groups {
        if g.len() > 1 {
            out.push(Goal::Line("One of these:".into(), ONE_OF));
        }
        for &i in g {
            let Some(cond) = m.conds.get(i).and_then(Option::as_ref) else { continue };
            let need = cond.amount.max(1);
            let have = if done { need } else { v.progress.get(i).copied().unwrap_or(0).min(need) };
            out.push(Goal::Bar { text: cond.text(), have, need, met: have >= need });
        }
    }
    out
}

fn goals_height(g: &[Goal]) -> f32 {
    g.iter().map(|x| if matches!(x, Goal::Line(..)) { LINE_H } else { BAR_BLOCK }).sum()
}

/// A reward line's part: an item (id, how many), a divider between choices, or a "+" between groups.
#[derive(Clone, Copy)]
enum Part {
    Item(i32, i64),
    Divider,
    Plus,
}

fn part_w(painter: &egui::Painter, p: Part) -> f32 {
    match p {
        Part::Item(_, n) => REWARD_ICON + if n > 1 { 2.0 + jtext::width(painter, &format!("x{n}"), &text(15.0)) } else { 0.0 } + 4.0,
        Part::Divider => 16.0,
        Part::Plus => jtext::width(painter, "+", &text(18.0)) + 8.0,
    }
}

/// Java `rewards`: the item groups in lines on top (choices spread over as many lines as the objectives are tall),
/// the battle pass points below.
struct Rewards {
    lines: Vec<Vec<Part>>,
    points: Vec<i64>,
}

fn rewards(painter: &egui::Painter, m: &Mission) -> Rewards {
    let mut lines: Vec<Vec<Part>> = Vec::new();
    let mut points = Vec::new();
    let width = |l: &[Part]| l.iter().map(|x| part_w(painter, *x)).sum::<f32>();
    let push = |lines: &mut Vec<Vec<Part>>, part: Part| {
        if lines.last().is_none_or(|l| !l.is_empty() && width(l) + part_w(painter, part) > W_REWARD) {
            lines.push(Vec::new());
        }
        lines.last_mut().expect("a line").push(part);
    };
    for group in m.reward_groups() {
        let group: Vec<&lg_app::missions::Reward> = group.iter().flat_map(|x| x.as_ref()).collect();
        if group.len() == 1 && group[0].kind == 1 {
            points.push(group[0].amount);
            continue;
        }
        let items: Vec<(i32, i64)> = group.iter().filter(|r| r.kind == 2 && r.item_id() > 0).map(|r| (r.item_id(), r.amount)).collect();
        if items.is_empty() {
            continue;
        }
        if !lines.is_empty() {
            push(&mut lines, Part::Plus);
        }
        if items.len() == 1 {
            push(&mut lines, Part::Item(items[0].0, items[0].1));
            continue;
        }
        // to pick from: in as many lines as the objectives are tall (one line up to 2 bars, then one more per bar),
        // more if they do not fit, spread evenly (10 in 3 lines: 4, 3, 3)
        let widest = items.iter().map(|(id, n)| part_w(painter, Part::Item(*id, *n))).fold(0.0f32, f32::max);
        let fit = ((W_REWARD + 16.0) / (widest + 16.0)).floor().max(1.0) as usize;
        let n = items.len();
        let want = m.conds.len().saturating_sub(1).max(1);
        let count = n.min(want.max(n.div_ceil(fit)));
        let mut at = 0;
        for line in 0..count {
            let k = n / count + usize::from(line < n % count);
            let mut row = Vec::new();
            for i in 0..k {
                if i > 0 {
                    row.push(Part::Divider);
                }
                row.push(Part::Item(items[at].0, items[at].1));
                at += 1;
            }
            lines.push(row);
        }
    }
    Rewards { lines, points }
}

fn rewards_height(r: &Rewards) -> f32 {
    r.lines.len() as f32 * (REWARD_ICON + 10.0) + if r.points.is_empty() { 0.0 } else { LINE_H + 2.0 }
}

impl QuestsView {
    pub fn show(&mut self, ui: &mut Ui, c: &mut Ctx, body: Rect) {
        let p = *c.p;
        let app = c.app;
        let data = app.world.data();
        let now = c.now;
        let now_s = now / 1000;
        let views = app.missions.views(now_s);
        let character = app.characters.playing().or_else(|| app.characters.last_played_char());
        let slot_names: Vec<String> = character
            .and_then(|ch| data.classes.iter().find(|k| k.id == ch.class_type))
            .and_then(|k| k.slot_types)
            .map(|s| s.iter().map(|t| lg_gamedata::items::slot_name(*t).to_string()).collect())
            .unwrap_or_default();
        let for_you = |m: &Mission| character.and_then(|ch| m.for_you(ch.seasonal, ch.crucible, &slot_names));
        let all = std::cell::Cell::new(self.all_missions);
        let refresh = std::cell::Cell::new(false);
        let hidden = &mut self.hidden;
        let icons = &mut *c.icons;
        let season_keys: Vec<String> = views.iter().map(|(s, _)| format!("season:{}", s.id)).collect();
        widgets::section(
            ui,
            &p,
            body,
            "Season missions",
            fixed::GREEN,
            |ui, head| {
                let painter = ui.painter().clone();
                // "updated 2h ago" after the title
                let title_w = jtext::width(&painter, "SEASON MISSIONS", &crate::theme::head(17.0));
                let note = if app.missions.fetching {
                    "loading...".to_string()
                } else if app.missions.fetched > 0 {
                    format!("updated {}", ago(app.missions.fetched, now))
                } else {
                    String::new()
                };
                jtext::draw(
                    &painter,
                    pos2(head.left() + 26.0 + title_w + 12.0, head.center().y),
                    Align2::LEFT_CENTER,
                    &note,
                    &text(15.0),
                    fixed::MUTED,
                );
                let rw = jtext::width(&painter, "REFRESH", &small()) + 40.0;
                let rr = Rect::from_min_size(pos2(head.right() - 5.0 - rw, head.top()), vec2(rw, head.height()));
                refresh.set(crate::inventory::button(ui, &p, rr, "Refresh", false, "missions-refresh"));
                let names = ["This character", "All missions"];
                let w: f32 = names.iter().map(|n| jtext::width(&painter, &n.to_uppercase(), &small()) + 26.0).sum();
                if let Some(i) = widgets::segmented(
                    ui,
                    &p,
                    pos2(rr.left() - 8.0 - w, head.top()),
                    head.height(),
                    &names,
                    usize::from(all.get()),
                    true,
                    &[],
                ) {
                    all.set(i == 1);
                }
            },
            |ui, r| {
                let mut child = ui.new_child(egui::UiBuilder::new().max_rect(r));
                egui::ScrollArea::vertical().id_salt("quests").show(&mut child, |ui| {
                    let top = ui.cursor().min;
                    let w = ui.available_width() - 12.0;
                    let mid = top.x + w / 2.0;
                    let painter = ui.painter().clone();
                    let sm = text(15.0);
                    let mut y = top.y + 14.0;

                    // the cards: the Tinkerer, a gold divider, each battle pass; their texts below them, centred
                    let tinkerer: Vec<&lg_app::tinkerer::Quest> =
                        app.tinkerer.quests.iter().filter(|q| q.category == 0 && (!q.completed || q.repeatable)).collect();
                    let n_cards = 1 + views.len();
                    let gap_div = if views.is_empty() { 0.0 } else { 24.0 };
                    let cards_w = n_cards as f32 * CARD_W + n_cards.saturating_sub(2) as f32 * 14.0 + gap_div;
                    let mut x = mid - cards_w / 2.0;
                    let mut picked: Option<String> = None;
                    let tink_left = if app.tinkerer.seen_at == 0 {
                        "Visit the Daily Quest Room".to_string()
                    } else {
                        format!("{} quest{}", tinkerer.len(), if tinkerer.len() == 1 { "" } else { "s" })
                    };
                    let tink_right =
                        if app.tinkerer.seen_at == 0 { String::new() } else { format!("seen {}", ago(app.tinkerer.seen_at, now)) };
                    let on = !hidden.contains(TINKERER_KEY);
                    if card(ui, &p, icons, app, pos2(x, y), on, TINKERER_KEY, "Tinkerer", &tink_left, &tink_right, None, Some(TINKERER)) {
                        picked = Some(TINKERER_KEY.into());
                    }
                    x += CARD_W;
                    if !views.is_empty() {
                        divider(&painter, x + 12.0, y + 2.0, y + ART_H + 22.0, p.gold.gamma_multiply(0.67));
                        x += gap_div;
                    }
                    for (s, list) in &views {
                        let claimed = list.iter().filter(|v| v.status == Status::Claimed).count();
                        let left = s.end_date - now_s;
                        let ends = if s.end_date <= 0 {
                            String::new()
                        } else if left > 0 {
                            format!("ends in {}", span(left))
                        } else {
                            "ended".into()
                        };
                        let key = format!("season:{}", s.id);
                        let a = art(ui.ctx(), s);
                        let on = !hidden.contains(&key);
                        let right = format!("{claimed}/{} claimed", list.len());
                        if card(ui, &p, icons, app, pos2(x, y), on, &key, s.name.as_deref().unwrap_or(""), &ends, &right, a, None) {
                            picked = Some(key);
                        }
                        x += CARD_W + 14.0;
                    }
                    if let Some(key) = picked {
                        pick(hidden, &season_keys, &key);
                    }
                    y += ART_H + 4.0 + jtext::height(&painter, &sm) + 12.0;
                    painter.rect_filled(Rect::from_min_size(pos2(top.x, y), vec2(w, 2.0)), 0.0, p.gold.gamma_multiply(0.85));
                    y += 2.0 + 16.0;
                    if views.is_empty() {
                        let t = "The battle pass missions load when you log in to the game (enter the Nexus or your vault).";
                        jtext::draw(&painter, pos2(mid, y), Align2::CENTER_TOP, t, &sm, fixed::MUTED);
                        y += LINE_H + 8.0;
                    }

                    // the list in the middle at its own width: claimable first, the Tinkerer's, then the rest
                    let (mut claimable, mut rest): (Vec<&View>, Vec<&View>) = (Vec::new(), Vec::new());
                    for (s, list) in &views {
                        if hidden.contains(&format!("season:{}", s.id)) {
                            continue;
                        }
                        for v in list {
                            if matches!(v.status, Status::Claimed | Status::Cooldown) || (!all.get() && for_you(v.mission).is_some()) {
                                continue;
                            }
                            if v.status == Status::Claimable { claimable.push(v) } else { rest.push(v) }
                        }
                    }
                    let order = |a: &&View, b: &&View| {
                        if a.season.id == b.season.id { a.mission.id.cmp(&b.mission.id) } else { b.season.current.cmp(&a.season.current) }
                    };
                    claimable.sort_by(order);
                    rest.sort_by(order);
                    let x0 = (mid - ROW_W / 2.0).max(top.x);
                    let mut rows = 0;
                    for v in &claimable {
                        y += mission_row(ui, &p, icons, app, v, for_you(v.mission), pos2(x0, y)) + ROW_GAP;
                        rows += 1;
                    }
                    if !hidden.contains(TINKERER_KEY) {
                        for q in &tinkerer {
                            y += tinkerer_row(ui, &p, icons, app, q, now_s, pos2(x0, y)) + ROW_GAP;
                            rows += 1;
                        }
                    }
                    for v in &rest {
                        y += mission_row(ui, &p, icons, app, v, for_you(v.mission), pos2(x0, y)) + ROW_GAP;
                        rows += 1;
                    }
                    if rows == 0 && !views.is_empty() {
                        jtext::draw(&painter, pos2(mid, y), Align2::CENTER_TOP, "Nothing to do here right now.", &sm, fixed::MUTED);
                        y += LINE_H;
                    }
                    ui.allocate_exact_size(vec2(w, y - top.y + 12.0), Sense::hover());
                });
            },
        );
        self.all_missions = all.get();
        if refresh.get() {
            c.asks.push(crate::window::Ask::RefreshMissions);
        }
    }
}

/// A card to pick (Java `filterCard`): a picture (or a sprite on a dark ground) with the title bottom left, a gold
/// outline when picked; below it one text on the left and one on the right. True when clicked.
#[allow(clippy::too_many_arguments)]
fn card(
    ui: &Ui,
    p: &Palette,
    icons: &mut crate::icons::Icons,
    app: &lg_app::App,
    at: Pos2,
    on: bool,
    key: &str,
    title: &str,
    left_t: &str,
    right_t: &str,
    art: Option<egui::TextureHandle>,
    sprite: Option<i32>,
) -> bool {
    let painter = ui.painter();
    let sm = text(15.0);
    let area = Rect::from_min_size(at, vec2(CARD_W, ART_H));
    let resp = ui.interact(area, ui.scope_id().with(("quest-card", key)), Sense::click());
    let fade = |c: Color32| if on { c } else { c.gamma_multiply(0.4) };
    painter.rect_filled(area, 5.0, fade(Color32::from_rgb(0x1A, 0x14, 0x20)));
    if let Some(t) = &art {
        // covering the area, cut evenly
        let [tw, th] = t.size().map(|v| v as f32);
        let scale = (CARD_W / tw).max(ART_H / th);
        let (sw, sh) = (CARD_W / scale / tw, ART_H / scale / th);
        let uv = Rect::from_min_max(pos2(0.5 - sw / 2.0, 0.5 - sh / 2.0), pos2(0.5 + sw / 2.0, 0.5 + sh / 2.0));
        painter.image(t.id(), area.shrink(1.0), uv, fade(Color32::WHITE));
    } else if let Some(id) = sprite
        && let Some(t) = icons.object(ui.ctx(), app.world.data(), id, 84)
    {
        painter.image(t.id(), Rect::from_center_size(area.center(), vec2(84.0, 84.0)), crate::detail::uv(), fade(Color32::WHITE));
    }
    // the title bottom left, on a dark fade
    let fr = Rect::from_min_max(pos2(area.left() + 1.0, area.bottom() - 44.0), area.max - vec2(1.0, 1.0));
    let mut mesh = egui::Mesh::default();
    let (clear, dark) = (Color32::from_black_alpha(0), Color32::from_black_alpha(190));
    mesh.colored_vertex(fr.left_top(), clear);
    mesh.colored_vertex(fr.right_top(), clear);
    mesh.colored_vertex(fr.right_bottom(), dark);
    mesh.colored_vertex(fr.left_bottom(), dark);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    painter.add(egui::Shape::mesh(mesh));
    let tf = text(17.0);
    let clip = painter.with_clip_rect(area.shrink(4.0));
    jtext::draw(&clip, pos2(area.left() + 11.0, area.bottom() - 8.0), Align2::LEFT_BOTTOM, title, &tf, Color32::from_black_alpha(200));
    jtext::draw(
        &clip,
        pos2(area.left() + 10.0, area.bottom() - 9.0),
        Align2::LEFT_BOTTOM,
        title,
        &tf,
        if on { p.gold } else { Color32::LIGHT_GRAY },
    );
    painter.rect_stroke(area, 5.0, Stroke::new(if on { 2.5 } else { 1.0 }, if on { CLAIMABLE } else { fixed::MUTED }), StrokeKind::Inside);
    jtext::draw(painter, pos2(area.left() + 3.0, area.bottom() + 4.0), Align2::LEFT_TOP, left_t, &sm, fixed::MUTED);
    jtext::draw(painter, pos2(area.right() - 3.0, area.bottom() + 4.0), Align2::RIGHT_TOP, right_t, &sm, fixed::MUTED);
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp.clicked()
}

/// Java `pick`: the Tinkerer shows only his quests; a battle pass while the Tinkerer is shown shows only that pass,
/// otherwise it is added or taken away (one stays picked).
fn pick(hidden: &mut HashSet<String>, seasons: &[String], key: &str) {
    if key == TINKERER_KEY {
        hidden.clear();
        hidden.extend(seasons.iter().cloned());
        return;
    }
    if !hidden.contains(TINKERER_KEY) {
        hidden.clear();
        hidden.insert(TINKERER_KEY.into());
        hidden.extend(seasons.iter().filter(|k| *k != key).cloned());
        return;
    }
    if !hidden.remove(key) && seasons.iter().filter(|k| !hidden.contains(*k)).count() > 1 {
        hidden.insert(key.into());
    }
}

/// An item's picture with its tip and its count beside it.
fn item(ui: &Ui, icons: &mut crate::icons::Icons, app: &lg_app::App, id: i32, count: i64, at: Pos2, faded: bool) {
    let data = app.world.data();
    let painter = ui.painter();
    let r = Rect::from_min_size(at, vec2(REWARD_ICON, REWARD_ICON));
    if let Some(t) = icons.object(ui.ctx(), data, id, REWARD_ICON as u32) {
        painter.image(t.id(), r, crate::detail::uv(), if faded { Color32::from_white_alpha(110) } else { Color32::WHITE });
    }
    let slot = data.items.item(id).map_or("", |it| it.slot_name());
    widgets::item_tip(ui, r, data, id, None, slot);
    if count > 1 {
        jtext::draw(painter, pos2(r.right() + 2.0, r.center().y), Align2::LEFT_CENTER, &format!("x{count}"), &text(15.0), fixed::MUTED);
    }
}

/// The objectives from `gy`; gives the first and the last bar's middle.
fn draw_goals(painter: &egui::Painter, p: &Palette, list: &[Goal], gx: f32, gy: f32, faded: bool) -> (f32, f32) {
    let sm = text(15.0);
    let mut y = gy;
    let (mut first, mut last) = (None, gy);
    for g in list {
        match g {
            Goal::Line(t, c) => {
                jtext::draw(painter, pos2(gx, y), Align2::LEFT_TOP, t, &sm, if faded { fixed::MUTED } else { *c });
                y += LINE_H;
            }
            Goal::Bar { text: t, have, need, met } => {
                let shown = widgets::fit(painter, t, &sm, W_GOAL);
                jtext::draw(painter, pos2(gx, y), Align2::LEFT_TOP, &shown, &sm, if faded { fixed::MUTED } else { p.ink });
                let bar = Rect::from_min_size(pos2(gx, y + LINE_H), vec2(W_BAR, 12.0));
                painter.rect_filled(bar, 6.0, Color32::from_rgb(0x5C, 0x4B, 0x36));
                if *have > 0 {
                    let fill = Rect::from_min_size(bar.min, vec2((W_BAR * *have as f32 / *need as f32).max(12.0), 12.0));
                    painter.rect_filled(fill, 6.0, if *met { CLAIMED } else { p.gold });
                }
                let label = format!("{}/{}", num(i64::from(*have)), num(i64::from(*need)));
                jtext::draw(painter, bar.center(), Align2::CENTER_CENTER, &label, &text(13.0), Color32::WHITE);
                first.get_or_insert(bar.center().y);
                last = bar.center().y;
                y += BAR_BLOCK;
            }
        }
    }
    (first.unwrap_or(gy), last)
}

/// One mission (Java `row`); gives its height.
fn mission_row(
    ui: &Ui,
    p: &Palette,
    icons: &mut crate::icons::Icons,
    app: &lg_app::App,
    v: &View,
    not_for_you: Option<String>,
    at: Pos2,
) -> f32 {
    let painter = ui.painter().clone();
    let m = v.mission;
    let faded = matches!(v.status, Status::Claimed | Status::Cooldown | Status::Locked) || not_for_you.is_some();
    let list = goals(v);
    let rw = rewards(&painter, m);
    let name_lines = 2 + usize::from(not_for_you.is_some());
    let h = goals_height(&list).max(rewards_height(&rw)).max(name_lines as f32 * LINE_H + 4.0).max(36.0) + 12.0;
    let r = Rect::from_min_size(at, vec2(ROW_W, h));
    if v.status == Status::Claimable {
        painter.rect_stroke(r, 8.0, Stroke::new(2.0, CLAIMABLE), StrokeKind::Inside);
    }
    let cy = r.center().y;
    let mut x = r.left() + 8.0;
    // the mission's picture as the game shows it ("sheet/index"), else its first reward item
    let pic = m
        .icon
        .as_deref()
        .and_then(|i| i.rsplit_once('/'))
        .and_then(|(sheet, idx)| icons.sheet(ui.ctx(), sheet, idx.trim().parse().ok()?, 36))
        .or_else(|| {
            let id = m.rewards.iter().flatten().find(|x| x.kind == 2 && x.item_id() > 0).map(|x| x.item_id())?;
            icons.object(ui.ctx(), app.world.data(), id, 36)
        });
    if let Some(t) = pic {
        let tint = if faded { Color32::from_white_alpha(120) } else { Color32::WHITE };
        painter.image(t.id(), Rect::from_min_size(pos2(x, cy - 18.0), vec2(36.0, 36.0)), crate::detail::uv(), tint);
    }
    x += W_ICON;
    let (k, kc) = kind(m);
    tag(&painter, pos2(x, cy), k, if faded { fixed::MUTED } else { kc });
    x += W_KIND;
    // the name (without "(Repeatable)"), its battle pass, why it is not for you
    let name = m.name.as_deref().unwrap_or("?").replace("(Repeatable)", "");
    let sm = text(15.0);
    let mut ny = cy - name_lines as f32 * LINE_H / 2.0;
    let clip = painter.with_clip_rect(Rect::from_min_size(pos2(x, r.top()), vec2(W_NAME - 8.0, h)));
    jtext::draw(&clip, pos2(x, ny), Align2::LEFT_TOP, name.trim(), &text(17.0), if faded { fixed::MUTED } else { p.ink });
    ny += LINE_H;
    jtext::draw(&clip, pos2(x, ny), Align2::LEFT_TOP, v.season.name.as_deref().unwrap_or(""), &sm, fixed::MUTED);
    ny += LINE_H;
    if let Some(n) = &not_for_you {
        jtext::draw(&clip, pos2(x, ny), Align2::LEFT_TOP, n, &sm, NOT_FOR_YOU);
    }
    x += W_NAME;
    // the objectives in the middle of the row
    let gh = goals_height(&list);
    let (first, last) = draw_goals(&painter, p, &list, x, cy - gh / 2.0, faded);
    // the arrow: one gentle wave, or a swoosh gathering several objectives
    let lx = x + W_GOAL;
    let gold = if faded { fixed::MUTED } else { p.gold.gamma_multiply(0.85) };
    let join = lx + W_LINK - 13.0;
    let bars = list.iter().filter(|g| matches!(g, Goal::Bar { .. })).count();
    let amid = if bars > 1 { (first + last) / 2.0 } else { cy };
    if bars > 1 {
        let (top, bottom) = (first - 22.0, last + 8.0);
        let span = amid - top;
        let reach = (span.max(0.0) / 40.0).min(1.0);
        let wl = W_LINK;
        brush(
            &painter,
            [
                pos2(lx + 1.0, top),
                pos2(lx + wl * (0.45 + 0.4 * reach), top - 5.0),
                pos2(lx + wl * 0.12, amid - span * 0.15),
                pos2(join, amid),
            ],
            3.5,
            6.5,
            4.5,
            gold,
        );
        brush(
            &painter,
            [
                pos2(lx, bottom),
                pos2(lx + wl * 0.5, bottom + 2.0),
                pos2(lx + wl * 0.3, amid + (bottom - amid) * 0.2),
                pos2(join, amid + 0.5),
            ],
            1.0,
            2.8,
            4.5,
            gold,
        );
    } else {
        brush(
            &painter,
            [pos2(lx + 2.0, amid + 5.0), pos2(lx + W_LINK * 0.3, amid + 9.0), pos2(lx + W_LINK * 0.45, amid - 7.0), pos2(join, amid)],
            1.0,
            4.5,
            4.5,
            gold,
        );
    }
    arrow_head(&painter, lx + W_LINK - 3.0, amid, 9.0, gold);
    // the rewards: their lines in the middle of the row, the points below
    let rx = lx + W_LINK;
    let mut ry = cy - rewards_height(&rw) / 2.0;
    for line in &rw.lines {
        let mut px = rx;
        for part in line {
            match *part {
                Part::Item(id, n) => item(ui, icons, app, id, n, pos2(px, ry + 4.0), faded),
                Part::Divider => {
                    let dx = px + 6.0;
                    divider(&painter, dx, ry + 2.0, ry + REWARD_ICON + 6.0, if faded { fixed::MUTED } else { p.gold.gamma_multiply(0.67) });
                    let dr = Rect::from_min_max(pos2(dx - 5.0, ry), pos2(dx + 5.0, ry + REWARD_ICON + 8.0));
                    widgets::tip(ui, dr, ("pick-one", m.id, px as i32, ry as i32), "Pick one");
                }
                Part::Plus => {
                    jtext::draw(
                        &painter,
                        pos2(px + 2.0, ry + 4.0 + REWARD_ICON / 2.0),
                        Align2::LEFT_CENTER,
                        "+",
                        &text(18.0),
                        fixed::MUTED,
                    );
                }
            }
            px += part_w(&painter, *part);
        }
        ry += REWARD_ICON + 10.0;
    }
    if !rw.points.is_empty() {
        let t = rw.points.iter().map(|n| format!("{} BXP", num(*n))).collect::<Vec<_>>().join(" + ");
        let tr = jtext::draw(&painter, pos2(rx, ry + 2.0), Align2::LEFT_TOP, &t, &sm, if faded { fixed::MUTED } else { p.gold });
        widgets::tip(ui, tr, ("bxp", m.id, v.season.id), "Battle pass points");
    }
    h
}

/// A Tinkerer quest (Java `tinkererRow`): what to hand in, and what you get; gives its height.
fn tinkerer_row(
    ui: &Ui,
    p: &Palette,
    icons: &mut crate::icons::Icons,
    app: &lg_app::App,
    q: &lg_app::tinkerer::Quest,
    now_s: i64,
    at: Pos2,
) -> f32 {
    let painter = ui.painter().clone();
    let data = app.world.data();
    let h = 36.0 + 20.0;
    let r = Rect::from_min_size(at, vec2(ROW_W, h));
    let cy = r.center().y;
    let mut x = r.left() + 8.0;
    let pic = q.rewards.first().and_then(|id| icons.object(ui.ctx(), data, *id, 36)).or_else(|| icons.object(ui.ctx(), data, TINKERER, 36));
    if let Some(t) = pic {
        painter.image(t.id(), Rect::from_min_size(pos2(x, cy - 18.0), vec2(36.0, 36.0)), crate::detail::uv(), Color32::WHITE);
    }
    x += W_ICON;
    tag(&painter, pos2(x, cy), "TINKERER", Color32::from_rgb(0xA4, 0x8C, 0xF0));
    x += W_KIND;
    let clip = painter.with_clip_rect(Rect::from_min_size(pos2(x, r.top()), vec2(W_NAME - 8.0, h)));
    jtext::draw(&clip, pos2(x, cy - LINE_H), Align2::LEFT_TOP, &q.name, &text(17.0), p.ink);
    let mut expiration: i64 = q.expiration.trim().parse().unwrap_or(0);
    if expiration > 10_000_000_000 {
        expiration /= 1000;
    }
    let sub = if expiration > 0 {
        let left = expiration - now_s;
        format!("The Tinkerer  ·  {}", if left > 0 { format!("ends in {}", span(left)) } else { "ended".into() })
    } else {
        "The Tinkerer".into()
    };
    jtext::draw(&clip, pos2(x, cy), Align2::LEFT_TOP, &sub, &text(15.0), fixed::MUTED);
    x += W_NAME;
    // what to hand in (the same items once, with their count), right-aligned before the arrow
    let mut counted: Vec<(i32, i64)> = Vec::new();
    for &id in &q.requirements {
        match counted.iter_mut().find(|(i, _)| *i == id) {
            Some(e) => e.1 += 1,
            None => counted.push((id, 1)),
        }
    }
    let total: f32 = counted.iter().map(|(id, n)| part_w(&painter, Part::Item(*id, *n))).sum();
    let mut ix = (x + W_GOAL - 8.0 - total).max(x);
    for (id, n) in counted {
        item(ui, icons, app, id, n, pos2(ix, cy - REWARD_ICON / 2.0), false);
        ix += part_w(&painter, Part::Item(id, n));
    }
    let lx = x + W_GOAL;
    let gold = p.gold.gamma_multiply(0.85);
    let join = lx + W_LINK - 13.0;
    brush(
        &painter,
        [pos2(lx + 2.0, cy + 5.0), pos2(lx + W_LINK * 0.3, cy + 9.0), pos2(lx + W_LINK * 0.45, cy - 7.0), pos2(join, cy)],
        1.0,
        4.5,
        4.5,
        gold,
    );
    arrow_head(&painter, lx + W_LINK - 3.0, cy, 9.0, gold);
    // what you get: one of them (dividers between), or all ("+" between)
    let mut rx = lx + W_LINK;
    for (i, &id) in q.rewards.iter().enumerate() {
        if i > 0 {
            if q.item_of_choice {
                divider(&painter, rx + 6.0, cy - 20.0, cy + 20.0, p.gold.gamma_multiply(0.67));
                rx += part_w(&painter, Part::Divider);
            } else {
                jtext::draw(&painter, pos2(rx + 2.0, cy), Align2::LEFT_CENTER, "+", &text(18.0), fixed::MUTED);
                rx += part_w(&painter, Part::Plus);
            }
        }
        item(ui, icons, app, id, 1, pos2(rx, cy - REWARD_ICON / 2.0), false);
        rx += part_w(&painter, Part::Item(id, 1));
    }
    if !q.description.trim().is_empty() {
        widgets::tip(ui, r, ("tinkerer-row", q.id.as_str()), &q.description);
    }
    h
}
