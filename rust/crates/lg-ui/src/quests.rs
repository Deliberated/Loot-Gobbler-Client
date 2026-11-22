//! Profile tab, Quests page (Java `MissionsView`): the season's missions (the battle pass trees) and the Tinkerer's
//! quests as a list: claimable ones first, then the Tinkerer's, then the rest, each row with its kind, where it
//! stands, its objectives with bars and its rewards. Cards on top pick the Tinkerer and each battle pass; "This
//! character" leaves out the missions your character makes no progress on.
//!
//! Not yet: Java's dotted arrows from the missions a locked one needs.

use crate::jtext;
use crate::profile::{Ctx, num};
use crate::theme::{Palette, fixed};
use crate::widgets::{self, small, text};
use egui::{Align2, Color32, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};
use lg_app::missions::{Mission, Status, View};
use std::collections::HashSet;

const IN_PROGRESS: Color32 = Color32::from_rgb(0x4C, 0x9B, 0xE8);
const CLAIMABLE: Color32 = Color32::from_rgb(0xF2, 0xC2, 0x30);
const CLAIMED: Color32 = Color32::from_rgb(0x5F, 0xB0, 0x4A);
const TINKERER_KEY: &str = "tinkerer";
const ROW_ICON: f32 = 36.0;
const W_KIND: f32 = 104.0;
const W_NAME: f32 = 260.0;
const W_GOAL: f32 = 260.0;
const BAR_W: f32 = 210.0;

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

/// "5h 12m", "2d 3h" from seconds.
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

/// A tag: an outline and the text in the colour (Java `tag`).
fn tag(painter: &egui::Painter, at: egui::Pos2, t: &str, color: Color32) -> Rect {
    let f = small();
    let w = jtext::width(painter, t, &f) + 14.0;
    let r = Rect::from_min_size(at, vec2(w, 24.0));
    painter.rect_stroke(r, 4.0, Stroke::new(1.5, color), StrokeKind::Inside);
    jtext::draw(painter, r.center(), Align2::CENTER_CENTER, t, &f, color);
    r
}

/// Java `kind`: who can do it.
fn kind(m: &Mission) -> (&'static str, Color32) {
    match m.participants {
        2 => ("SEASONAL", Color32::from_rgb(0x3F, 0xC4, 0x8A)),
        4 => ("CRUCIBLE", Color32::from_rgb(0xE0, 0x57, 0x4F)),
        _ => ("REGULAR", Color32::from_rgb(0xE5, 0xB2, 0x3A)),
    }
}

/// "Ends in 5h 12m" from a Tinkerer quest's expiration (unix seconds or ms as text), if it says one.
fn ends_in(expiration: &str, now_s: i64) -> Option<String> {
    let mut t: i64 = expiration.trim().parse().ok()?;
    if t > 10_000_000_000 {
        t /= 1000;
    }
    let left = t - now_s;
    (t > 0).then(|| if left > 0 { format!("ends in {}", span(left)) } else { "ended".to_string() })
}

impl QuestsView {
    pub fn show(&mut self, ui: &mut Ui, c: &mut Ctx, body: Rect) {
        let p = *c.p;
        let app = c.app;
        let data = app.world.data();
        let now_s = c.now / 1000;
        let views = app.missions.views(now_s);
        let character = app.characters.playing().or_else(|| app.characters.last_played_char());
        let slot_names: Vec<String> = character
            .and_then(|ch| data.classes.iter().find(|k| k.id == ch.class_type))
            .and_then(|k| k.slot_types)
            .map(|s| s.iter().map(|t| lg_gamedata::items::slot_name(*t).to_string()).collect())
            .unwrap_or_default();
        let for_you = |m: &Mission| character.and_then(|ch| m.for_you(ch.seasonal, ch.crucible, &slot_names));
        let all = std::cell::Cell::new(self.all_missions);
        let hidden = &mut self.hidden;
        let icons = &mut *c.icons;
        widgets::section(
            ui,
            &p,
            body,
            "Season missions",
            fixed::GREEN,
            |ui, head| {
                let names = ["This character", "All missions"];
                let w: f32 = names.iter().map(|n| jtext::width(ui.painter(), &n.to_uppercase(), &small()) + 26.0).sum();
                if let Some(i) = widgets::segmented(
                    ui,
                    &p,
                    pos2(head.right() - 8.0 - w, head.top()),
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
                let mut child = ui.new_child(egui::UiBuilder::new().max_rect(r.shrink2(vec2(12.0, 10.0))));
                egui::ScrollArea::vertical().id_salt("quests").show(&mut child, |ui| {
                    let top = ui.cursor().min;
                    let w = ui.available_width() - 12.0;
                    let painter = ui.painter().clone();
                    let f = text(18.0);
                    let sm = text(15.0);
                    let mut y = top.y;
                    // who you play: the missions shown are those it makes progress on
                    let who = match character {
                        Some(ch) => format!(
                            "{} {}{}",
                            if app.characters.playing().is_some() { "Playing" } else { "Played last:" },
                            ch.class_name,
                            if ch.seasonal {
                                " (seasonal)"
                            } else if ch.crucible {
                                " (crucible)"
                            } else {
                                ""
                            }
                        ),
                        None => "No character known yet".into(),
                    };
                    jtext::draw(&painter, pos2(top.x, y), Align2::LEFT_TOP, &who, &sm, fixed::MUTED);
                    y += jtext::height(&painter, &sm) + 10.0;

                    // the cards: the Tinkerer and each battle pass, clicked to show or hide their quests
                    let tinkerer: Vec<&lg_app::tinkerer::Quest> =
                        app.tinkerer.quests.iter().filter(|q| q.category == 0 && (!q.completed || q.repeatable)).collect();
                    let mut cards: Vec<(String, String, String, String)> = vec![(
                        TINKERER_KEY.into(),
                        "Tinkerer".into(),
                        if app.tinkerer.seen_at == 0 {
                            "Visit the Daily Quest Room".into()
                        } else {
                            format!("{} quest{}", tinkerer.len(), if tinkerer.len() == 1 { "" } else { "s" })
                        },
                        String::new(),
                    )];
                    for (s, list) in &views {
                        let claimed = list.iter().filter(|v| matches!(v.status, Status::Claimed)).count();
                        let left = s.end_date - now_s;
                        let ends = if s.end_date <= 0 {
                            String::new()
                        } else if left > 0 {
                            format!("ends in {}", span(left))
                        } else {
                            "ended".into()
                        };
                        cards.push((
                            format!("season:{}", s.id),
                            s.name.clone().unwrap_or_default(),
                            ends,
                            format!("{claimed}/{} claimed", list.len()),
                        ));
                    }
                    let (cw, ch_h) = (230.0, 64.0);
                    let mut x = top.x;
                    for (key, title, left_t, right_t) in &cards {
                        if x + cw > top.x + w {
                            x = top.x;
                            y += ch_h + 10.0;
                        }
                        let cr = Rect::from_min_size(pos2(x, y), vec2(cw, ch_h));
                        let on = !hidden.contains(key);
                        let resp = ui.interact(cr, ui.scope_id().with(("quest-card", key.as_str())), Sense::click());
                        painter.rect(
                            cr,
                            6.0,
                            if on { p.header } else { p.paper },
                            Stroke::new(if on { 2.0 } else { 1.0 }, if on { p.gold } else { p.line }),
                            StrokeKind::Inside,
                        );
                        let ink = if on { p.ink } else { fixed::MUTED };
                        jtext::draw(&painter.with_clip_rect(cr), pos2(cr.left() + 10.0, cr.top() + 8.0), Align2::LEFT_TOP, title, &f, ink);
                        jtext::draw(&painter, pos2(cr.left() + 10.0, cr.bottom() - 8.0), Align2::LEFT_BOTTOM, left_t, &sm, fixed::MUTED);
                        jtext::draw(&painter, pos2(cr.right() - 10.0, cr.bottom() - 8.0), Align2::RIGHT_BOTTOM, right_t, &sm, fixed::MUTED);
                        if resp.hovered() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        }
                        if resp.clicked() && !hidden.remove(key) {
                            hidden.insert(key.clone());
                        }
                        x += cw + 14.0;
                    }
                    y += ch_h + 12.0;
                    painter.rect_filled(Rect::from_min_size(pos2(top.x, y), vec2(w, 2.0)), 0.0, p.gold.gamma_multiply(0.6));
                    y += 2.0 + 10.0;
                    if views.is_empty() {
                        let t = "The battle pass missions load when you log in to the game (enter the Nexus or your vault).";
                        jtext::draw(&painter, pos2(top.x, y), Align2::LEFT_TOP, t, &sm, fixed::MUTED);
                        y += jtext::height(&painter, &sm) + 8.0;
                    }

                    // the missions of the battle passes picked: claimable first, then the Tinkerer, then the rest
                    let (mut claimable, mut rest): (Vec<&View>, Vec<&View>) = (Vec::new(), Vec::new());
                    for (s, list) in &views {
                        if hidden.contains(&format!("season:{}", s.id)) {
                            continue;
                        }
                        for v in list {
                            if matches!(v.status, Status::Claimed | Status::Cooldown) {
                                continue;
                            }
                            if !all.get() && for_you(v.mission).is_some() {
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
                    let mut rows = 0;
                    for v in claimable {
                        y += mission_row(ui, &p, icons, app, v, for_you(v.mission), pos2(top.x, y), w) + 10.0;
                        rows += 1;
                    }
                    if !hidden.contains(TINKERER_KEY) {
                        for q in &tinkerer {
                            y += tinkerer_row(ui, &p, icons, app, q, now_s, pos2(top.x, y), w) + 10.0;
                            rows += 1;
                        }
                    }
                    for v in rest {
                        y += mission_row(ui, &p, icons, app, v, for_you(v.mission), pos2(top.x, y), w) + 10.0;
                        rows += 1;
                    }
                    if rows == 0 && !views.is_empty() {
                        let t = if hidden.len() > views.len() {
                            "Pick the Tinkerer or a battle pass above to see their quests."
                        } else {
                            "Nothing to do here right now."
                        };
                        jtext::draw(&painter, pos2(top.x, y), Align2::LEFT_TOP, t, &sm, fixed::MUTED);
                        y += jtext::height(&painter, &sm);
                    }
                    ui.allocate_exact_size(vec2(w, y - top.y + 12.0), Sense::hover());
                });
            },
        );
        self.all_missions = all.get();
    }
}

/// An item's icon (32 px) with its tip; `count` above 1 written beside it. Gives the width used.
fn item(ui: &Ui, icons: &mut crate::icons::Icons, app: &lg_app::App, id: i32, count: i64, at: egui::Pos2, faded: bool) -> f32 {
    let data = app.world.data();
    let painter = ui.painter();
    let r = Rect::from_min_size(at, vec2(32.0, 32.0));
    if let Some(t) = icons.object(ui.ctx(), data, id, 32) {
        painter.image(t.id(), r, crate::detail::uv(), if faded { Color32::from_white_alpha(110) } else { Color32::WHITE });
    }
    let slot = data.items.item(id).map_or("", |it| it.slot_name());
    widgets::item_tip(ui, r, data, id, None, slot);
    let mut w = 32.0;
    if count > 1 {
        let t =
            jtext::draw(painter, pos2(r.right() + 2.0, r.center().y), Align2::LEFT_CENTER, &format!("x{count}"), &text(15.0), fixed::MUTED);
        w += t.width() + 4.0;
    }
    w + 6.0
}

/// One mission: icon, kind and state, name and season, objectives with bars, rewards. Gives its height.
fn mission_row(
    ui: &Ui,
    p: &Palette,
    icons: &mut crate::icons::Icons,
    app: &lg_app::App,
    v: &View,
    not_for_you: Option<String>,
    at: egui::Pos2,
    w: f32,
) -> f32 {
    let painter = ui.painter().clone();
    let m = v.mission;
    let f = text(18.0);
    let sm = text(15.0);
    let line = jtext::height(&painter, &sm);
    let goal_h = line + 2.0 + 14.0 + 6.0;
    let worn = m.worn();
    let groups = m.groups();
    let headers = worn.len() + usize::from(groups.len() > 1) + groups.iter().filter(|g| g.len() > 1).count();
    let goals_h = headers as f32 * (line + 2.0) + m.conds.len() as f32 * goal_h;
    let h = (goals_h + 16.0).max(ROW_ICON + 20.0).max(2.0 * jtext::height(&painter, &f) + 16.0);
    let r = Rect::from_min_size(at, vec2(w, h));
    let locked = v.status == Status::Locked;
    painter.rect(
        r,
        6.0,
        p.paper,
        Stroke::new(if v.status == Status::Claimable { 2.0 } else { 1.0 }, if v.status == Status::Claimable { CLAIMABLE } else { p.line }),
        StrokeKind::Inside,
    );
    let cy = r.center().y;
    // the icon: its first reward item
    let first = m.rewards.iter().flatten().find(|x| x.kind == 2 && x.item_id() > 0).map(|x| x.item_id());
    if let Some(id) = first
        && let Some(t) = icons.object(ui.ctx(), app.world.data(), id, ROW_ICON as u32)
    {
        painter.image(
            t.id(),
            Rect::from_min_size(pos2(r.left() + 10.0, cy - ROW_ICON / 2.0), vec2(ROW_ICON, ROW_ICON)),
            crate::detail::uv(),
            if locked { Color32::from_white_alpha(110) } else { Color32::WHITE },
        );
    }
    let kx = r.left() + 10.0 + ROW_ICON + 12.0;
    let (k, kc) = kind(m);
    tag(&painter, pos2(kx, cy - 26.0), k, if locked { fixed::MUTED } else { kc });
    let (state, sc) = match v.status {
        Status::Claimable => ("CLAIM", CLAIMABLE),
        Status::InProgress => ("IN PROGRESS", IN_PROGRESS),
        Status::Locked => ("LOCKED", fixed::MUTED),
        Status::Cooldown => ("COOLDOWN", fixed::MUTED),
        Status::Claimed => ("CLAIMED", CLAIMED),
    };
    jtext::draw(&painter, pos2(kx + 2.0, cy + 6.0), Align2::LEFT_TOP, state, &text(14.0), sc);
    // name, season (or why it is not for you)
    let nx = kx + W_KIND;
    let name_clip = painter.with_clip_rect(Rect::from_min_size(pos2(nx, r.top()), vec2(W_NAME - 10.0, h)));
    jtext::draw(
        &name_clip,
        pos2(nx, cy - 2.0),
        Align2::LEFT_BOTTOM,
        m.name.as_deref().unwrap_or("?"),
        &f,
        if locked { fixed::MUTED } else { p.ink },
    );
    let sub = not_for_you.unwrap_or_else(|| v.season.name.clone().unwrap_or_default());
    jtext::draw(&name_clip, pos2(nx, cy + 2.0), Align2::LEFT_TOP, &sub, &sm, fixed::MUTED);
    // the objectives with bars
    let gx = nx + W_NAME;
    let mut gy = r.top() + 8.0;
    let done = matches!(v.status, Status::Claimed | Status::Cooldown);
    for wr in &worn {
        jtext::draw(&painter, pos2(gx, gy), Align2::LEFT_TOP, &format!("Requires {wr} equipped"), &sm, CLAIMABLE);
        gy += line + 2.0;
    }
    if m.conds.is_empty()
        && let Some(d) = m.desc.as_deref().filter(|d| !d.trim().is_empty())
    {
        jtext::draw(
            &painter.with_clip_rect(Rect::from_min_size(pos2(gx, r.top()), vec2(W_GOAL, h))),
            pos2(gx, gy),
            Align2::LEFT_TOP,
            d,
            &sm,
            fixed::MUTED,
        );
    }
    if groups.len() > 1 {
        let choices = groups.iter().any(|g| g.len() > 1);
        jtext::draw(
            &painter,
            pos2(gx, gy),
            Align2::LEFT_TOP,
            if choices { "All of these groups:" } else { "All of these:" },
            &sm,
            CLAIMABLE,
        );
        gy += line + 2.0;
    }
    for g in &groups {
        if g.len() > 1 {
            jtext::draw(&painter, pos2(gx, gy), Align2::LEFT_TOP, "One of these:", &sm, Color32::from_rgb(0x3F, 0xC4, 0xD6));
            gy += line + 2.0;
        }
        for &i in g {
            let Some(cond) = m.conds.get(i).and_then(Option::as_ref) else { continue };
            let need = cond.amount.max(1);
            let have = if done { need } else { v.progress.get(i).copied().unwrap_or(0).min(need) };
            let clip = painter.with_clip_rect(Rect::from_min_size(pos2(gx, r.top()), vec2(W_GOAL, h)));
            jtext::draw(&clip, pos2(gx, gy), Align2::LEFT_TOP, &cond.text(), &sm, if locked { fixed::MUTED } else { p.ink });
            let bar = Rect::from_min_size(pos2(gx, gy + line + 2.0), vec2(BAR_W, 14.0));
            painter.rect_filled(bar, 7.0, Color32::from_black_alpha(90));
            let fill = Rect::from_min_size(bar.min, vec2(BAR_W * have as f32 / need as f32, 14.0));
            painter.rect_filled(fill, 7.0, if have >= need { CLAIMED } else { IN_PROGRESS });
            jtext::draw(
                &painter,
                bar.center(),
                Align2::CENTER_CENTER,
                &format!("{}/{}", num(i64::from(have)), num(i64::from(need))),
                &text(13.0),
                Color32::WHITE,
            );
            gy += goal_h;
        }
    }
    // the rewards: items (one of each group) and battle pass points
    let mut rx = gx + W_GOAL + 20.0;
    for group in m.reward_groups() {
        let items: Vec<&lg_app::missions::Reward> = group.iter().flat_map(|x| x.as_ref()).collect();
        if items.len() > 1 && items.iter().all(|x| x.kind == 2) {
            jtext::draw(&painter, pos2(rx, cy), Align2::LEFT_CENTER, "one of", &sm, fixed::MUTED);
            rx += jtext::width(&painter, "one of", &sm) + 6.0;
        }
        for x in items {
            if rx > r.right() - 40.0 {
                break;
            }
            if x.kind == 2 && x.item_id() > 0 {
                rx += item(ui, icons, app, x.item_id(), x.amount, pos2(rx, cy - 16.0), locked);
            } else if x.kind == 1 {
                let t = jtext::draw(&painter, pos2(rx, cy), Align2::LEFT_CENTER, &format!("+{} pts", num(x.amount)), &sm, p.gold);
                rx += t.width() + 10.0;
            }
        }
    }
    h
}

/// A Tinkerer quest (Java `tinkererRow`): what to hand in, and what you get. Gives its height.
#[allow(clippy::too_many_arguments)]
fn tinkerer_row(
    ui: &Ui,
    p: &Palette,
    icons: &mut crate::icons::Icons,
    app: &lg_app::App,
    q: &lg_app::tinkerer::Quest,
    now_s: i64,
    at: egui::Pos2,
    w: f32,
) -> f32 {
    let painter = ui.painter().clone();
    let f = text(18.0);
    let sm = text(15.0);
    let h = 60.0;
    let r = Rect::from_min_size(at, vec2(w, h));
    painter.rect(r, 6.0, p.paper, Stroke::new(1.0, p.line), StrokeKind::Inside);
    let cy = r.center().y;
    if let Some(&id) = q.rewards.first()
        && let Some(t) = icons.object(ui.ctx(), app.world.data(), id, ROW_ICON as u32)
    {
        painter.image(
            t.id(),
            Rect::from_min_size(pos2(r.left() + 10.0, cy - ROW_ICON / 2.0), vec2(ROW_ICON, ROW_ICON)),
            crate::detail::uv(),
            Color32::WHITE,
        );
    }
    let kx = r.left() + 10.0 + ROW_ICON + 12.0;
    tag(&painter, pos2(kx, cy - 12.0), "TINKERER", Color32::from_rgb(0xA4, 0x8C, 0xF0));
    let nx = kx + W_KIND;
    let clip = painter.with_clip_rect(Rect::from_min_size(pos2(nx, r.top()), vec2(W_NAME - 10.0, h)));
    jtext::draw(&clip, pos2(nx, cy - 2.0), Align2::LEFT_BOTTOM, &q.name, &f, p.ink);
    let sub = match ends_in(&q.expiration, now_s) {
        Some(e) => format!("The Tinkerer  ·  {e}"),
        None => "The Tinkerer".into(),
    };
    jtext::draw(&clip, pos2(nx, cy + 2.0), Align2::LEFT_TOP, &sub, &sm, fixed::MUTED);
    // what to hand in, the same items once with their count
    let mut counted: Vec<(i32, i64)> = Vec::new();
    for &id in &q.requirements {
        match counted.iter_mut().find(|(i, _)| *i == id) {
            Some(e) => e.1 += 1,
            None => counted.push((id, 1)),
        }
    }
    let mut x = nx + W_NAME;
    for (id, n) in counted {
        x += item(ui, icons, app, id, n, pos2(x, cy - 16.0), false);
    }
    // what you get (one of them, if you pick)
    let mut rx = (nx + W_NAME + W_GOAL + 20.0).max(x + 20.0);
    jtext::draw(&painter, pos2(rx - 14.0, cy), Align2::CENTER_CENTER, ">", &f, fixed::MUTED);
    if q.item_of_choice && q.rewards.len() > 1 {
        jtext::draw(&painter, pos2(rx, cy), Align2::LEFT_CENTER, "one of", &sm, fixed::MUTED);
        rx += jtext::width(&painter, "one of", &sm) + 6.0;
    }
    for &id in &q.rewards {
        if rx > r.right() - 40.0 {
            break;
        }
        rx += item(ui, icons, app, id, 1, pos2(rx, cy - 16.0), false);
    }
    if !q.description.trim().is_empty() {
        widgets::tip(ui, r, ("tinkerer-row", q.id.as_str()), &q.description);
    }
    h
}
