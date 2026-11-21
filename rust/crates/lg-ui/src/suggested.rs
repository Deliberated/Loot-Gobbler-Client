//! The Suggested Loadout box (Java: `CharacterDetail.ideal` and its parts): the scope, deleted-enchantment and second
//! chance switches, the enemy DEF, the stat boxes (each a toggle for what you aim for), and per slot the suggested
//! item with where its copy is, its stats, and the enchantments planned for it.

use crate::detail::{Rarity, enchant_badge, uv};
use crate::icons::Icons;
use crate::jtext;
use crate::profile::num;
use crate::theme::{Palette, fixed};
use crate::widgets::{self, small, text};
use egui::{Align2, Color32, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};
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

/// Java `ThreeWaySwitch`: a recessed pill, gold as the knob moves on, dots for the stops, a metal knob.
fn switch(painter: &egui::Painter, r: Rect, stops: usize, stop: usize, p: &Palette) {
    let h = r.height() - 1.0;
    let track = r.shrink2(vec2(0.5, 0.5));
    painter.rect_filled(track, h / 2.0, Color32::from_rgb(0x42, 0x3D, 0x35));
    if stop > 0 {
        let level = (stop as f32 / (stops - 1) as f32).min(1.0);
        let g = p.gold;
        painter.rect_filled(track, h / 2.0, Color32::from_rgba_unmultiplied(g.r(), g.g(), g.b(), (150.0 * level) as u8));
    }
    painter.rect_stroke(track, h / 2.0, Stroke::new(1.2, Color32::from_black_alpha(120)), StrokeKind::Inside);
    let pad = 3.0;
    let d = h - 2.0 * pad;
    let span = r.width() - 1.0 - 2.0 * pad - d;
    for i in 0..stops {
        let cx = r.left() + pad + span * i as f32 / (stops - 1) as f32 + d / 2.0;
        painter.circle_filled(pos2(cx, r.center().y), 2.0, if i == stop { p.gold } else { Color32::from_white_alpha(70) });
    }
    let x = r.left() + pad + span * stop as f32 / (stops - 1) as f32;
    let c = pos2(x + d / 2.0, r.top() + pad + 0.5 + d / 2.0);
    painter.circle_filled(c + vec2(1.0, 2.0), d / 2.0, Color32::from_black_alpha(90));
    painter.circle_filled(c, d / 2.0, Color32::from_rgb(0xCF, 0xCF, 0xCF));
    painter.circle_filled(c - vec2(d * 0.12, d * 0.18), d * 0.28, Color32::from_rgb(0xF2, 0xF2, 0xF2));
    painter.circle_stroke(c, d / 2.0, Stroke::new(1.2, Color32::from_rgb(0x6E, 0x6E, 0x6E)));
}

/// A label with a switch at the right end of the column; true when clicked.
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
    switch(painter, Rect::from_min_size(pos2(r.right() - sw, r.top()), vec2(sw, h)), stops, stop, p);
    let clicked =
        ui.interact(r, ui.scope_id().with(("switch", id)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).clicked();
    widgets::rich_tip(ui, r, ("switch-tip", id), tip);
    (h, clicked)
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
    let small_f = small();
    let name_f = text(18.0);
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
        let lockable_here = settings.scope == 0 && pk.equipped();
        let locked_now: Vec<i32> = if lockable_here { settings.locked_enchants(c.char_id, slot) } else { Vec::new() };
        let lock_hint = |tip: &mut Vec<widgets::TipLine>, locked: bool| {
            tip.push(if locked {
                vec![("Locked:".to_string(), None, true), (" kept whatever you aim for. Click to unlock".to_string(), None, false)]
            } else {
                vec![("Click to lock it: the suggestion keeps it".to_string(), Some(fixed::MUTED), false)]
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
                let has: Vec<i32> = pk.enchants.clone().unwrap_or_default();
                let shown = if settings.chosen.is_empty() { en.of(pk.enchants.as_deref()) } else { pk.plan.picks.clone() };
                for e in &shown {
                    let owned = has.contains(&en.get(*e).kind);
                    let color = if settings.chosen.is_empty() {
                        fixed::MUTED
                    } else if owned {
                        HAS_IT
                    } else {
                        p.gold
                    };
                    let mut tip = enchant_tip(en, item, *e, (!settings.chosen.is_empty()).then_some(owned));
                    let lockable = lockable_here && owned;
                    if lockable {
                        lock_hint(&mut tip, locked_now.contains(&en.get(*e).kind));
                    }
                    chips.push(Chip { e: Some(*e), text: en.get(*e).name.clone(), color, tip, struck: false, lockable, hover_only: false });
                }
                if !settings.chosen.is_empty() {
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
                let free = n as i64 - if settings.chosen.is_empty() { shown.len() } else { s.finals[slot].len() } as i64;
                if !settings.chosen.is_empty() && free > 0 {
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
        let always = chips.iter().filter(|c| !c.hover_only).count();
        let (_, h0) = text_h_of(always);
        let hovered = ui.rect_contains_pointer(Rect::from_min_size(pos2(ix, iy), vec2(iw, h0)));
        chips.retain(|c| !c.hover_only || hovered);
        let (text_h, h) = text_h_of(chips.len());
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
            for (t, color, _) in line {
                let fitted = widgets::fit(&painter, t, f, (tx + tw - lx).max(10.0));
                let a = jtext::draw(&painter, pos2(lx, ty), Align2::LEFT_TOP, &fitted, f, *color);
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
            let color = if locked { HAS_IT } else { chip.color };
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
            let cr = Rect::from_min_max(pos2(tx, ty), pos2(a.right(), ty + small_h));
            widgets::rich_tip(ui, cr, ("chip", slot, k), &chip.tip);
            if chip.lockable
                && let Some(kd) = kind
                && ui
                    .interact(cr, ui.scope_id().with(("chip-lock", slot, k)), Sense::click())
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .clicked()
            {
                change = Some(Change::EnchantLock(slot, kd));
            }
            ty += small_h;
        }
        iy += h;
    }
    let _ = c;
    (stats_h.max(iy - r.top()), change)
}
