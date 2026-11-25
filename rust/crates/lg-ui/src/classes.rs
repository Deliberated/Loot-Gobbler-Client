//! Characters > Classes (Java: `ClassesView`): a card per class with its exaltations and your characters of it, most
//! exalted first; beside them the account's exaltation progress, in total and per stat.

use crate::icons::Icons;
use crate::jtext;
use crate::profile::{Ctx, num};
use crate::theme::{Palette, fixed, mix};
use crate::widgets::{self, small, text};
use egui::{Align2, Color32, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};
use lg_app::exalt_store::{THRESHOLDS, level, total};

const CARD_MIN_WIDTH: f32 = 400.0;
const GAP: f32 = 8.0;
const PORTRAIT: f32 = 72.0;
const ICON: f32 = 24.0;
const STAT_NAMES: [&str; 8] = ["Life", "Mana", "Attack", "Defense", "Speed", "Dexterity", "Vitality", "Wisdom"];
/// Each shown stat's index in the exaltation data (life, mana, attack, defense, speed, dexterity, vitality, wisdom).
const STAT_INDEX: [usize; 8] = [7, 6, 5, 4, 1, 0, 2, 3];
const STAT_POTION: [i32; 8] = [0xae9, 0xaea, 0xa1f, 0xa20, 0xa21, 0xa4c, 0xa34, 0xa35];
const PIP_ON: Color32 = Color32::from_rgb(0xFF, 0xD2, 0x1F);
const PIP_W: f32 = 8.0;
const PIP_H: f32 = 11.0;
const PIP_STEP: f32 = 10.0;

/// Java `Palette.CLASS_COLORS`.
pub fn class_color(name: &str) -> Color32 {
    let c = match name {
        "Rogue" => 0x6E7B8B,
        "Archer" => 0x5FA343,
        "Wizard" => 0xB36BE0,
        "Priest" => 0xE0C36B,
        "Warrior" => 0xC0503C,
        "Knight" => 0xE88A3A,
        "Paladin" => 0xF2C230,
        "Assassin" => 0x49A07A,
        "Necromancer" => 0x8C5AD6,
        "Huntress" => 0xE0679C,
        "Mystic" => 0x3FC4D6,
        "Trickster" => 0xE85AB4,
        "Sorcerer" => 0x4C8FE0,
        "Ninja" => 0x7A8899,
        "Samurai" => 0xD64545,
        "Bard" => 0xD9B44A,
        "Summoner" => 0x57B894,
        "Kensei" => 0xB5543A,
        "Druid" => 0x6B8E3A,
        _ => return fixed::STONE,
    };
    Color32::from_rgb((c >> 16) as u8, (c >> 8) as u8, c as u8)
}

struct Info {
    id: i32,
    name: String,
    progress: Option<Vec<i32>>,
    total: i32,
    sum: i32,
    chars: usize,
    best: i64,
}

/// A progress bar as FlatLaf draws it: the line colour, gold for the part done.
fn bar(painter: &egui::Painter, p: &Palette, r: Rect, share: f32) {
    painter.rect_filled(r, 0.0, p.line);
    painter.rect_filled(Rect::from_min_size(r.min, vec2((r.width() * share.clamp(0.0, 1.0)).round(), r.height())), 0.0, p.gold);
}

pub fn show(ui: &mut Ui, c: &mut Ctx, body: Rect) {
    let p = *c.p;
    let app = c.app;
    let data = app.world.data();
    let max_class = 8 * THRESHOLDS.len() as i32;
    let mut infos: Vec<Info> = data
        .classes
        .iter()
        .map(|k| {
            let pr = app.exalts.progress(k.id).cloned();
            let mine: Vec<_> = app.characters.all().filter(|x| x.alive && x.class_type == k.id).collect();
            Info {
                id: k.id,
                name: k.name.clone(),
                total: total(pr.as_deref()),
                sum: pr.as_ref().map_or(0, |v| v.iter().take(8).sum()),
                progress: pr,
                chars: mine.len(),
                best: mine.iter().map(|x| x.fame).max().unwrap_or(0),
            }
        })
        .collect();
    infos.sort_by(|a, b| b.total.cmp(&a.total).then(b.sum.cmp(&a.sum)).then(a.name.cmp(&b.name)));

    // the split: the side 280 px, the main area snapped down to whole columns of cards (Java `SideSplit.setSnap`)
    let overhead = 2.0 + 12.0;
    let base = overhead + 20.0 - GAP;
    let (main, side) = widgets::side_split(ui, c.p, c.props, "classes", body, 280.0, 220.0, 200.0, Some((base, CARD_MIN_WIDTH + GAP)));
    let cols = (((main.width() - base + 0.001) / (CARD_MIN_WIDTH + GAP)).floor()).max(1.0);

    let icons = &mut *c.icons;
    widgets::section(
        ui,
        &p,
        main,
        "Classes",
        fixed::GOLD,
        |_, _| {},
        |ui, r| {
            egui::ScrollArea::vertical().id_salt("classes").show(ui, |ui| {
                let top = ui.cursor().min;
                let inner_w = r.width() - 12.0 - 20.0;
                let cw = ((inner_w - (cols - 1.0) * GAP) / cols).floor();
                let painter = ui.painter().clone();
                let ch = card_height(&painter);
                let picked = ui.ctx().data(|d| d.get_temp::<i32>(selected_id()));
                for (i, info) in infos.iter().enumerate() {
                    let (col, row) = ((i as f32 % cols), (i as f32 / cols).floor());
                    let cr = Rect::from_min_size(pos2(top.x + 10.0 + col * (cw + GAP), top.y + 10.0 + row * (ch + GAP)), vec2(cw, ch));
                    card(ui, &p, icons, data, info, cr);
                    // the class clicked: a gold frame with a soft glow around it
                    if picked == Some(info.id) {
                        for i in 1..=4 {
                            let a = 0.35 * (1.0 - (i - 1) as f32 / 4.0);
                            painter.rect_stroke(cr.expand(i as f32), 0.0, Stroke::new(1.0, p.gold.gamma_multiply(a)), StrokeKind::Outside);
                        }
                        painter.rect_stroke(cr, 0.0, Stroke::new(3.0, p.gold), StrokeKind::Inside);
                    }
                    // click: the class on the right
                    let resp = ui
                        .interact(cr, ui.scope_id().with(("class-card", info.id)), Sense::click())
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .on_hover_text(format!("Deaths and best dungeons of the {}", info.name));
                    if resp.clicked() {
                        ui.ctx().data_mut(|d| d.insert_temp(selected_id(), info.id));
                    }
                }
                let rows = (infos.len() as f32 / cols).ceil();
                ui.allocate_exact_size(vec2(r.width() - 12.0, 20.0 + rows * (ch + GAP) - GAP), Sense::hover());
            });
        },
    );

    // the side: exaltation progress, then per stat
    let painter = ui.painter().clone();
    let all: i32 = infos.iter().map(|i| i.total).sum();
    let max = infos.len() as i32 * max_class;
    let pct = if max == 0 { 0.0 } else { 100.0 * f64::from(all) / f64::from(max) };
    let full = infos.iter().filter(|i| i.total == max_class).count();
    let lh = jtext::height(&painter, &text(18.0));
    let prog_h = jtext::height(&painter, &small()) + 10.0 + lh + 6.0 + lh;
    let head = widgets::section_head_height(&painter);
    // the picked class on top, with Close
    let picked = ui.ctx().data(|d| d.get_temp::<i32>(selected_id())).filter(|id| data.classes.iter().any(|k| k.id == *id));
    let mut side_top = side.top();
    if let Some(class) = picked {
        let name = data.classes.iter().find(|k| k.id == class).map_or(String::new(), |k| k.name.clone());
        let body_h = class_body(ui, &p, c.icons, app, class, c.now, Rect::from_min_size(side.min, vec2(side.width(), 0.0)), false);
        let r = Rect::from_min_size(side.min, vec2(side.width(), head + 2.0 + body_h));
        let mut close = false;
        let icons = &mut *c.icons;
        let now = c.now;
        widgets::section(
            ui,
            &p,
            r,
            &name,
            fixed::BLUE,
            |ui, h| {
                let bw = jtext::width(ui.painter(), "CLOSE", &small()) + 32.0;
                close = crate::inventory::button(
                    ui,
                    &p,
                    Rect::from_min_size(pos2(h.right() - bw, h.top()), vec2(bw, h.height())),
                    "Close",
                    false,
                    "class-close",
                );
            },
            |ui, b| {
                class_body(ui, &p, icons, app, class, now, b, true);
            },
        );
        if close {
            ui.ctx().data_mut(|d| d.remove::<i32>(selected_id()));
        }
        side_top = r.bottom() + 8.0;
    }
    let prog = Rect::from_min_size(pos2(side.left(), side_top), vec2(side.width(), head + 2.0 + 20.0 + prog_h));
    widgets::section(
        ui,
        &p,
        prog,
        "Exaltation Progress",
        fixed::GREEN,
        |_, _| {},
        |ui, r| {
            let painter = ui.painter();
            let x = r.left() + 10.0;
            let w = r.width() - 20.0;
            let mut y = r.top() + 10.0;
            let t = jtext::draw(painter, pos2(x, y), Align2::LEFT_TOP, "EXALTATIONS", &small(), p.ink_muted);
            y = t.bottom();
            bar(painter, &p, Rect::from_min_size(pos2(x, y), vec2(w, 10.0)), (pct / 100.0) as f32);
            y += 10.0;
            let t = jtext::draw(painter, pos2(x, y), Align2::LEFT_TOP, &format!("{all} / {max}   {pct:.1}%"), &text(18.0), p.ink);
            y = t.bottom() + 6.0;
            let note = if app.exalts.progress.is_empty() {
                "Exaltations load when you log in with the sniffer running.".to_string()
            } else {
                format!("{full} of {} classes fully exalted", infos.len())
            };
            jtext::draw(painter, pos2(x, y), Align2::LEFT_TOP, &note, &text(18.0), fixed::MUTED);
        },
    );
    let stats = Rect::from_min_max(pos2(side.left(), prog.bottom() + 8.0), side.max);
    let icons = &mut *c.icons;
    widgets::section(
        ui,
        &p,
        stats,
        "Per Stat",
        fixed::PURPLE,
        |_, _| {},
        |ui, r| {
            let painter = ui.painter().clone();
            let x = r.left() + 10.0;
            let w = r.width() - 20.0;
            let mut y = r.top() + 8.0;
            let stat_max = infos.len() as i32 * THRESHOLDS.len() as i32;
            for s in 0..8 {
                let have: i32 =
                    infos.iter().filter_map(|i| i.progress.as_ref()).map(|pr| level(pr.get(STAT_INDEX[s]).copied().unwrap_or(0))).sum();
                let rh = lh.max(18.0);
                if let Some(t) = icons.object(ui.ctx(), data, STAT_POTION[s], 18) {
                    painter.image(
                        t.id(),
                        Rect::from_min_size(pos2(x, y + (rh - 18.0) / 2.0), vec2(18.0, 18.0)),
                        crate::detail::uv(),
                        Color32::WHITE,
                    );
                }
                jtext::draw(&painter, pos2(x + 26.0, y + rh / 2.0), Align2::LEFT_CENTER, STAT_NAMES[s], &text(18.0), p.ink);
                jtext::draw(&painter, pos2(x + w, y + rh / 2.0), Align2::RIGHT_CENTER, &format!("{have} / {stat_max}"), &text(18.0), p.ink);
                y += rh + 2.0;
                bar(
                    &painter,
                    &p,
                    Rect::from_min_size(pos2(x, y), vec2(w, 5.0)),
                    if stat_max == 0 { 0.0 } else { have as f32 / stat_max as f32 },
                );
                y += 5.0 + 5.0;
            }
        },
    );
}

fn card_height(painter: &egui::Painter) -> f32 {
    let head = jtext::height(painter, &text(22.0)).max(jtext::height(painter, &text(20.0)));
    let body = head + 4.0 + 2.0 * ICON + 4.0 + 4.0 + jtext::height(painter, &text(18.0));
    body.max(PORTRAIT) + 12.0
}

/// The "Exalted <Class>" skin (Java `exaltedSkin`), or -1.
fn exalted_skin(data: &lg_gamedata::GameData, class: &str) -> i32 {
    let name = format!("Exalted {class}");
    let mut ids: Vec<i32> = data.objects.values().filter(|o| o.id_name == name && o.class == "Skin").map(|o| o.id).collect();
    ids.sort_unstable();
    ids.last().copied().unwrap_or(-1)
}

/// One class's card (Java `Card`): its sprite (the Exalted skin once fully exalted), name and exaltations, the eight
/// stats with a pip per exaltation, your characters of it.
fn card(ui: &Ui, p: &Palette, icons: &mut Icons, data: &lg_gamedata::GameData, info: &Info, r: Rect) {
    let painter = ui.painter();
    let max_class = 8 * THRESHOLDS.len() as i32;
    let done = info.total == max_class;
    painter.rect_filled(r, 0.0, if done { mix(p.gold, p.paper, 0.45) } else { p.header });
    if done {
        painter.rect_stroke(r, 0.0, Stroke::new(2.0, p.gold), StrokeKind::Inside);
    } else {
        painter.rect_stroke(r, 0.0, Stroke::new(1.0, Color32::BLACK), StrokeKind::Inside);
    }
    let skin = if done { exalted_skin(data, &info.name) } else { -1 };
    let sprite = if skin > 0 { skin } else { info.id };
    if let Some(t) = icons.object(ui.ctx(), data, sprite, (PORTRAIT - 8.0) as u32) {
        let pr = Rect::from_min_size(pos2(r.left() + 6.0, r.center().y - PORTRAIT / 2.0), vec2(PORTRAIT, PORTRAIT));
        painter.image(
            t.id(),
            Rect::from_center_size(pr.center(), vec2(PORTRAIT - 8.0, PORTRAIT - 8.0)),
            crate::detail::uv(),
            Color32::WHITE,
        );
    }
    let bx = r.left() + 6.0 + PORTRAIT + 10.0;
    let bw = r.right() - 10.0 - bx;
    let mut y = r.top() + 6.0;
    let hh = jtext::height(painter, &text(22.0)).max(jtext::height(painter, &text(20.0)));
    painter.rect_filled(Rect::from_min_size(pos2(bx, y + hh / 2.0 - 4.0), vec2(8.0, 8.0)), 0.0, class_color(&info.name));
    jtext::draw(painter, pos2(bx + 16.0, y + hh / 2.0), Align2::LEFT_CENTER, &info.name, &text(22.0), p.ink);
    let t = if info.progress.is_none() { "-".to_string() } else { format!("{} / {max_class}", info.total) };
    let tr = jtext::draw(painter, pos2(bx + bw, y + hh / 2.0), Align2::RIGHT_CENTER, &t, &text(20.0), if done { p.gold } else { p.ink });
    widgets::tip(ui, tr, ("class-total", info.id), "Exaltations of this class");
    y += hh + 4.0;
    // two rows of four: potion, then five pips
    let cell_w = ICON + 6.0 + 5.0 * PIP_STEP;
    let cw = ((bw - 3.0 * 8.0) / 4.0).min(cell_w);
    for s in 0..8 {
        let (col, row) = ((s % 4) as f32, (s / 4) as f32);
        let cx = bx + col * (cw + 8.0);
        let cy = y + row * (ICON + 4.0);
        if let Some(t) = icons.object(ui.ctx(), data, STAT_POTION[s], ICON as u32) {
            painter.image(t.id(), Rect::from_min_size(pos2(cx, cy), vec2(ICON, ICON)), crate::detail::uv(), Color32::WHITE);
        }
        let progress = info.progress.as_ref().and_then(|v| v.get(STAT_INDEX[s]).copied()).unwrap_or(0);
        let lv = level(progress);
        // Java `StatCell`'s tip
        let tip = if info.progress.is_none() {
            STAT_NAMES[s].to_string()
        } else if (lv as usize) < THRESHOLDS.len() {
            format!("{}: {lv} / {}, {progress} / {} to the next", STAT_NAMES[s], THRESHOLDS.len(), THRESHOLDS[lv as usize])
        } else {
            format!("{}: {lv} / {}, maxed", STAT_NAMES[s], THRESHOLDS.len())
        };
        widgets::tip(ui, Rect::from_min_size(pos2(cx, cy), vec2(cw, ICON)), ("stat-cell", info.id, s), &tip);
        let px0 = cx + ICON + 6.0;
        let step = ((cw - ICON - 6.0) / 5.0).floor().clamp(3.0, PIP_STEP);
        let w = (step - (PIP_STEP - PIP_W)).max(2.0);
        let py = cy + ((ICON - PIP_H) / 2.0).floor();
        for i in 0..5 {
            let px = px0 + i as f32 * step;
            if i < lv {
                painter.rect_filled(Rect::from_min_size(pos2(px, py), vec2(w, PIP_H)), 0.0, Color32::BLACK);
                painter.rect_filled(Rect::from_min_size(pos2(px + 1.0, py + 1.0), vec2(w - 2.0, PIP_H - 2.0)), 0.0, PIP_ON);
            } else {
                painter.rect_filled(
                    Rect::from_min_size(pos2(px + 1.0, py + 1.0), vec2(w - 2.0, PIP_H - 2.0)),
                    0.0,
                    Color32::from_black_alpha(50),
                );
            }
        }
    }
    y += 2.0 * ICON + 4.0 + 4.0;
    let who = if info.chars == 0 {
        "No characters".to_string()
    } else {
        format!("{} character{}  ·  best {} fame", info.chars, if info.chars == 1 { "" } else { "s" }, num(info.best))
    };
    jtext::draw(painter, pos2(bx, y), Align2::LEFT_TOP, &who, &text(18.0), fixed::MUTED);
}

/// Shows this class on the right as if its card was clicked (Java `open`).
pub fn open(ctx: &egui::Context, class: i32) {
    ctx.data_mut(|d| d.insert_temp(selected_id(), class));
}

/// The class picked on its card (Java `selected`), -1: none.
fn selected_id() -> egui::Id {
    egui::Id::unique("lg-class-selected")
}

const STAGE_TRACK: Color32 = Color32::from_rgb(0x3A, 0x2E, 0x20);
const STAGE_EDGE: Color32 = Color32::from_rgb(0x1A, 0x14, 0x0D);
const STAGE_RECENT: Color32 = Color32::from_rgb(0x4C, 0x8F, 0xE0);
const STAGE_INSET: f32 = 6.0;

/// Java `stageTip`.
fn stage_tip(before: i32, progress: i32) -> String {
    let lv = level(progress) as usize;
    let max = THRESHOLDS.len();
    let day = if progress > before { format!("\n+{} in the last {RECENT_DAYS} days", progress - before) } else { String::new() };
    if lv >= max {
        return format!("All {max} exaltations{day}");
    }
    let next = THRESHOLDS[lv];
    format!("{lv} of {max} exaltations  ·  {} to the next (at {next}){day}", next - progress)
}

/// What the class page counts as recent progress (blue on the bars).
const RECENT_DAYS: i64 = 3;

/// Java `StageBar`: a stat's progress from 0 to 75 to scale, a diamond at each exaltation, lit once reached; what
/// came in the last RECENT_DAYS days blue.
fn stage_bar(painter: &egui::Painter, r: Rect, before: i32, progress: i32) {
    let before = before.min(progress);
    let end = *THRESHOLDS.last().expect("thresholds") as f32;
    let w = r.width() - 2.0 * STAGE_INSET;
    let (h, cy) = (6.0, r.center().y);
    let y = cy - h / 2.0;
    let x0 = r.left() + STAGE_INSET;
    painter.rect_filled(Rect::from_min_size(pos2(x0, y), vec2(w, h)), 0.0, STAGE_TRACK);
    let old = (w * before.min(end as i32) as f32 / end).round();
    painter.rect_filled(Rect::from_min_size(pos2(x0, y), vec2(old, h)), 0.0, PIP_ON);
    let now = (w * progress.min(end as i32) as f32 / end).round();
    painter.rect_filled(Rect::from_min_size(pos2(x0 + old, y), vec2((now - old).max(0.0), h)), 0.0, STAGE_RECENT);
    for at in THRESHOLDS {
        let x = x0 + (w * at as f32 / end).round();
        let pts = vec![pos2(x, cy - 7.0), pos2(x + 6.0, cy), pos2(x, cy + 7.0), pos2(x - 6.0, cy)];
        let fill = if before >= at {
            PIP_ON
        } else if progress >= at {
            STAGE_RECENT
        } else {
            STAGE_TRACK
        };
        painter.add(egui::Shape::convex_polygon(pts, fill, Stroke::new(1.0, STAGE_EDGE)));
    }
}

/// The picked class on the right (Java `showClass`): its exaltations per stat to scale, how many of your
/// characters of it died, the dungeons with your best DPS with it. Paints only with `paint`; gives its height.
fn class_body(ui: &Ui, p: &Palette, icons: &mut Icons, app: &lg_app::App, class: i32, now: i64, r: Rect, paint: bool) -> f32 {
    let data = app.world.data();
    let painter = ui.painter().clone();
    let f = text(18.0);
    let lh = jtext::height(&painter, &f);
    let sh = jtext::height(&painter, &small());
    let (x, w) = (r.left() + 10.0, r.width() - 20.0);
    let mut y = r.top() + 10.0;
    let name = data.classes.iter().find(|k| k.id == class).map_or(String::new(), |k| k.name.clone());
    if paint {
        if let Some(t) = icons.object(ui.ctx(), data, class, 64) {
            painter.image(t.id(), Rect::from_min_size(pos2(x, y), vec2(64.0, 64.0)), crate::detail::uv(), Color32::WHITE);
        }
        jtext::draw(&painter, pos2(x + 64.0 + 12.0, y + 32.0), Align2::LEFT_CENTER, &name, &text(26.0), p.ink);
    }
    y += 64.0 + 8.0;
    let caption = |y: &mut f32, t: &str| {
        if paint {
            jtext::draw(&painter, pos2(x, *y), Align2::LEFT_TOP, t, &small(), p.ink_muted);
        }
        *y += sh;
    };
    let line = |y: &mut f32, t: &str, font: &egui::FontId, color: Color32| {
        if paint {
            jtext::draw(&painter, pos2(x, *y), Align2::LEFT_TOP, t, font, color);
        }
        *y += jtext::height(&painter, font);
    };
    // each stat's progress on the way to 75, its five exaltations marked where they are reached
    caption(&mut y, "EXALTATIONS");
    match app.exalts.progress(class) {
        None => line(&mut y, "Not known yet: loads when you log in with the sniffer running.", &f, fixed::MUTED),
        Some(progress) => {
            let end = *THRESHOLDS.last().expect("thresholds");
            let since = now - RECENT_DAYS * 24 * 3_600_000;
            let before_of = |s: usize| app.exalts.progress_at(&app.dungeon_visits, class, STAT_INDEX[s] as i32, since);
            // all the stats' progress in those days together, in the bars' blue
            let gained: i32 =
                (0..8).map(|s| (progress.get(STAT_INDEX[s]).copied().unwrap_or(0).min(end) - before_of(s).min(end)).max(0)).sum();
            if gained > 0 {
                line(&mut y, &format!("+{gained} in the last {RECENT_DAYS} days"), &f, STAGE_RECENT);
            }
            for s in 0..8 {
                let pr = progress.get(STAT_INDEX[s]).copied().unwrap_or(0);
                let before = before_of(s);
                if paint {
                    let rh = lh.max(18.0);
                    if let Some(t) = icons.object(ui.ctx(), data, STAT_POTION[s], 18) {
                        painter.image(
                            t.id(),
                            Rect::from_min_size(pos2(x, y + (rh - 18.0) / 2.0), vec2(18.0, 18.0)),
                            crate::detail::uv(),
                            Color32::WHITE,
                        );
                    }
                    jtext::draw(&painter, pos2(x + 26.0, y + rh / 2.0), Align2::LEFT_CENTER, STAT_NAMES[s], &f, p.ink);
                    jtext::draw(&painter, pos2(x + w, y + rh / 2.0), Align2::RIGHT_CENTER, &format!("{} / {end}", pr.min(end)), &f, p.ink);
                    let br = Rect::from_min_size(pos2(x, y + rh + 2.0), vec2(w, 16.0));
                    stage_bar(&painter, br, before, pr);
                    widgets::tip(ui, br, ("stage", class, s), &stage_tip(before, pr));
                }
                y += lh.max(18.0) + 2.0 + 16.0 + 5.0;
            }
            // the scale: 0 and where each exaltation is reached
            if paint {
                let bw = w - 2.0 * STAGE_INSET;
                for at in std::iter::once(0).chain(THRESHOLDS) {
                    let t = at.to_string();
                    let tw = jtext::width(&painter, &t, &small());
                    let tx = (x + STAGE_INSET + (bw * at as f32 / end as f32).round() - tw / 2.0).clamp(x, x + w - tw);
                    jtext::draw(&painter, pos2(tx, y), Align2::LEFT_TOP, &t, &small(), fixed::MUTED);
                }
            }
            y += 16.0;
        }
    }
    y += 12.0;
    // deaths
    let all: Vec<_> = app.characters.all().filter(|c| c.class_type == class).collect();
    let dead: Vec<_> = all.iter().filter(|c| !c.alive).collect();
    caption(&mut y, "DEATHS");
    line(
        &mut y,
        &format!(
            "{} {}  ·  {} alive",
            dead.len(),
            if dead.len() == 1 { "character died" } else { "characters died" },
            all.len() - dead.len()
        ),
        &f,
        p.ink,
    );
    if let Some(best) = dead.iter().map(|c| c.fame).max() {
        line(&mut y, &format!("best fame of a fallen one: {}", num(best)), &f, fixed::MUTED);
    }
    y += 12.0;
    // the best peak DPS per dungeon, over the runs where you played this class
    let peak = |r: &lg_app::dps_history::Run| -> f64 {
        r.bosses
            .iter()
            .filter_map(|b| b.players.iter().find(|x| x.me).map(|me| f64::from(me.damage) / (b.fight_ms as f64 / 1000.0).max(1.0)))
            .fold(0.0, f64::max)
    };
    let mut best: Vec<&lg_app::dps_history::Run> = Vec::new();
    for r in &app.dps_history.runs {
        if r.guild_hall || !r.bosses.iter().any(|b| b.players.iter().any(|x| x.me && x.class_type == class)) {
            continue;
        }
        match best.iter_mut().find(|b| b.dungeon == r.dungeon) {
            Some(b) if peak(r) > peak(b) => *b = r,
            Some(_) => {}
            None => best.push(r),
        }
    }
    best.sort_by(|a, b| peak(b).total_cmp(&peak(a)));
    caption(&mut y, "TOP DPS DUNGEONS");
    if best.is_empty() {
        line(&mut y, "No dungeon runs with this class yet.", &f, fixed::MUTED);
    }
    for (i, r) in best.iter().take(3).enumerate() {
        let nf = text(if i == 0 { 20.0 } else { 18.0 });
        let rh = (jtext::height(&painter, &nf) + lh).max(32.0) + 8.0;
        if paint {
            if let Some(id) = data.dungeons.portal_by_name.get(&r.dungeon)
                && let Some(t) = icons.fitted(ui.ctx(), data, *id, 32, 32)
            {
                painter.image(
                    t.id(),
                    Rect::from_min_size(pos2(x, y + (rh - 32.0) / 2.0), vec2(32.0, 32.0)),
                    crate::detail::uv(),
                    Color32::WHITE,
                );
            }
            let t = jtext::draw(&painter, pos2(x + 42.0, y + 4.0), Align2::LEFT_TOP, &r.dungeon, &nf, p.ink);
            jtext::draw(
                &painter,
                pos2(x + 42.0, t.bottom()),
                Align2::LEFT_TOP,
                &format!("{} DPS", num(peak(r).round() as i64)),
                &f,
                fixed::MUTED,
            );
        }
        y += rh;
    }
    y + 10.0 - r.top()
}
