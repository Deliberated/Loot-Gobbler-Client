//! The Game Modes tab (Java: `GameModesView`, `RuleChips`): the categories (PPE, UPE) on the left; on the right the
//! chosen one: for a UPE its variant, the guild version and no-pet switches, the rules at a glance and in full, and
//! your characters playing it, able to start a run, or that lost theirs.

use crate::jtext;
use crate::profile::{Ctx, num};
use crate::theme::{Palette, fixed, mix};
use crate::widgets::{self, brighter, small, text, wrapped};
use egui::{Align2, Color32, Rect, Sense, Shape, Stroke, StrokeKind, Ui, pos2, vec2};
use lg_app::upe::{self, CATEGORY_INFO, GUILD_RULES, NO_PET_RULES, OWN_RULES, PREREQUISITE, UPE_VARIANTS};

const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

pub struct GameModesView {
    pub category: String,
    pub variant: String,
    pub guild: bool,
    pub no_pet: bool,
    /// The wizard for a new run, while open.
    pub wizard: Option<crate::game_mode_wizard::Wizard>,
}

impl Default for GameModesView {
    fn default() -> Self {
        GameModesView { category: "UPE".into(), variant: "UPE".into(), guild: false, no_pet: false, wizard: None }
    }
}

/// A rule tag (Java `RuleChips.Chip`): words, fill, edge, ink, allowed, and the shiny star.
pub(crate) struct Chip {
    text: &'static str,
    fill: Color32,
    edge: Color32,
    ink: Color32,
    allowed: bool,
    sparkle: bool,
}

fn rgb(c: u32) -> Color32 {
    Color32::from_rgb((c >> 16) as u8, (c >> 8) as u8, c as u8)
}

fn chip(text: &'static str, fill: u32, ink: Color32, allowed: bool) -> Chip {
    let f = rgb(fill);
    Chip { text, fill: f, edge: brighter(f), ink, allowed, sparkle: false }
}

/// What the loadout of a run in this mode may hold (Java `RuleChips.loadout`).
pub(crate) fn loadout_chips(mode: &str) -> Vec<Chip> {
    let (ut, st, tiered, divine) = (0x8E4FD0, 0xD9A21B, 0x5F6B7A, 0xF2C232);
    let w = Color32::WHITE;
    let st_ink = rgb(0x3A2604);
    let mut l = match upe::base(mode) {
        "PPE" => vec![chip("Tiered", tiered, w, true), chip("UT", ut, w, true), chip("ST", st, st_ink, true)],
        "PUPE" => vec![chip("UT", ut, w, true), chip("ST", st, st_ink, false), chip("Tiered", tiered, w, false)],
        "STPE" => vec![chip("ST", st, st_ink, true), chip("UT", ut, w, false), chip("Tiered", tiered, w, false)],
        "ShinyUPE" => vec![
            Chip { text: "UT", fill: rgb(0xFFC8E0), edge: w, ink: rgb(0x3A2350), allowed: true, sparkle: true },
            chip("ST", st, st_ink, false),
            chip("Tiered", tiered, w, false),
        ],
        "DivinePE" => vec![chip("Divine", divine, rgb(0x4A2E00), true), chip("Common to Legendary", tiered, w, false)],
        _ => vec![chip("UT", ut, w, true), chip("ST", st, st_ink, true), chip("Tiered", tiered, w, false)],
    };
    let darker =
        |c: Color32| Color32::from_rgb((f32::from(c.r()) * 0.7) as u8, (f32::from(c.g()) * 0.7) as u8, (f32::from(c.b()) * 0.7) as u8);
    let t0 = darker(rgb(tiered));
    l.push(Chip { text: "T0 start gear", fill: t0, edge: brighter(t0), ink: w, allowed: true, sparkle: false });
    l
}

/// Where what it equips may come from (Java `RuleChips.sources`).
pub(crate) fn source_chips(guild: bool) -> Vec<Chip> {
    let w = Color32::WHITE;
    vec![
        chip("Own drops", 0x3C9347, w, true),
        chip("Vault", 0x4D5A3A, w, false),
        if guild { chip("Guild trades", 0xB07D0A, w, true) } else { chip("Trades", 0x4D5A3A, w, false) },
        chip("Shop & rewards", 0x4D5A3A, w, false),
    ]
}

/// A row of tags after a caption (Java `RuleChips.row`); gives its height.
pub(crate) fn chip_row(painter: &egui::Painter, at: egui::Pos2, caption: &str, chips: &[Chip]) -> f32 {
    const H: f32 = 28.0;
    const PAD: f32 = 9.0;
    const MARK: f32 = 16.0;
    let f = text(16.0);
    let mut x = at.x + 6.0;
    let y = at.y + 2.0;
    jtext::draw(painter, pos2(x, y + (H + 2.0) / 2.0), Align2::LEFT_CENTER, caption, &f, fixed::MUTED);
    x += 90.0 + 6.0;
    for c in chips {
        let star = if c.sparkle { 14.0 + 4.0 } else { 0.0 };
        let mark = if c.allowed { 6.0 + MARK - 2.0 } else { 0.0 };
        let w = PAD + star + jtext::width(painter, c.text, &f) + mark + PAD;
        let plate = Rect::from_min_size(pos2(x + 0.5, y + 1.0), vec2(w - 1.0, H - 1.0));
        let fade = |col: Color32| if c.allowed { col } else { col.gamma_multiply(0.8) };
        painter.rect_filled(plate, 4.5, fade(c.fill));
        painter.rect_stroke(plate, 4.5, Stroke::new(1.0, fade(c.edge)), StrokeKind::Inside);
        let mut tx = x + PAD;
        if c.sparkle {
            let (cx, cy, r) = (x + PAD + 7.0, plate.center().y, 7.0);
            let k = r * 0.24;
            let pts = vec![
                pos2(cx, cy - r),
                pos2(cx + k, cy - k),
                pos2(cx + r, cy),
                pos2(cx + k, cy + k),
                pos2(cx, cy + r),
                pos2(cx - k, cy + k),
                pos2(cx - r, cy),
                pos2(cx - k, cy - k),
            ];
            painter.add(Shape::closed_line(pts.clone(), Stroke::new(1.4, rgb(0x4B2F8F))));
            painter.add(Shape::convex_polygon(vec![pts[0], pts[2], pts[4], pts[6]], Color32::WHITE, Stroke::NONE));
            tx += 14.0 + 4.0;
        }
        jtext::draw(painter, pos2(tx, plate.center().y), Align2::LEFT_CENTER, c.text, &f, fade(c.ink));
        if c.allowed {
            let (mx, my) = (x + w - 1.0 - PAD + 2.0 - MARK, y + 1.0 + (H - 1.0 - MARK) / 2.0);
            painter.circle_filled(pos2(mx + MARK / 2.0, my + MARK / 2.0), MARK / 2.0, rgb(0x2E8B3E));
            let s = Stroke::new(2.0, Color32::WHITE);
            painter.line_segment([pos2(mx + 4.0, my + 8.5), pos2(mx + 7.0, my + 11.5)], s);
            painter.line_segment([pos2(mx + 7.0, my + 11.5), pos2(mx + 12.0, my + 5.0)], s);
        } else {
            let (a, b) = (pos2(x + 3.0, y + H - 2.0), pos2(x + w - 4.0, y + 3.0));
            painter.line_segment([a, b], Stroke::new(4.6, rgb(0x5A1410)));
            painter.line_segment([a, b], Stroke::new(3.0, rgb(0xFF4A3D)));
        }
        x += w + 6.0;
    }
    H + 2.0 + 4.0
}

fn date(zone: &lg_app::localtime::Zone, t: i64) -> String {
    if t <= 0 {
        return "?".into();
    }
    let l = zone.local_date_time(t);
    format!("{} {} {}", l[8..10].trim_start_matches('0'), MONTHS[l[5..7].parse::<usize>().unwrap_or(1) - 1], &l[11..16])
}

impl GameModesView {
    fn mode(&self) -> String {
        upe::with_guild(if self.category == "UPE" { &self.variant } else { &self.category }, self.guild)
    }

    pub fn show(&mut self, ui: &mut Ui, c: &mut Ctx) {
        let p = *c.p;
        let app = c.app;
        let all = ui.max_rect().shrink(8.0);
        let mut runs: Vec<&upe::Run> = app.upe.runs.values().collect();
        runs.sort_by_key(|r| r.char_id);
        let alive = |id: i32| app.characters.get(id).is_some_and(|ch| ch.alive);
        // the categories on the left
        let list = Rect::from_min_size(all.min, vec2(290.0, all.height()));
        let mut picked = None;
        let mut open_wizard = false;
        widgets::section(
            ui,
            &p,
            list,
            "Game Modes",
            fixed::GOLD,
            |ui, head| {
                // the plus: a new game mode run, in the wizard (Java `GameModeWizard.plusButton`)
                let r = Rect::from_center_size(pos2(head.right() - 13.0, head.center().y), vec2(26.0, 26.0));
                let resp = ui
                    .interact(r, ui.scope_id().with("new-run"), Sense::click())
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .on_hover_text("Start a new game mode run: pick a mode, then create its character");
                if resp.clicked() {
                    open_wizard = true;
                }
                let painter = ui.painter();
                let green = if resp.hovered() { crate::theme::mix(fixed::GREEN, Color32::WHITE, 0.25) } else { fixed::GREEN };
                painter.rect_filled(r.shrink(1.0), 3.0, green);
                let s = Stroke::new(2.5, Color32::WHITE);
                painter.line_segment([pos2(r.center().x - 5.0, r.center().y), pos2(r.center().x + 5.0, r.center().y)], s);
                painter.line_segment([pos2(r.center().x, r.center().y - 5.0), pos2(r.center().x, r.center().y + 5.0)], s);
            },
            |ui, r| {
                let painter = ui.painter().clone();
                let mut y = r.top() + 8.0;
                for m in &CATEGORY_INFO {
                    let playing = runs
                        .iter()
                        .filter(|x| upe::category(&x.mode) == m.code && x.marked && x.broken.is_none() && alive(x.char_id))
                        .count();
                    let ch = jtext::height(&painter, &text(24.0)).max(jtext::height(&painter, &text(14.0)));
                    let h = 8.0 + 2.0 * ch + 8.0;
                    let er = Rect::from_min_size(pos2(r.left() + 8.0, y), vec2(r.width() - 16.0, h));
                    if m.code == self.category {
                        painter.rect_filled(er, 0.0, mix(p.gold, p.paper, 0.25));
                        painter.rect_stroke(er, 0.0, Stroke::new(2.0, p.gold), StrokeKind::Inside);
                    }
                    jtext::draw(
                        &painter,
                        pos2(er.left() + 10.0, er.top() + 8.0 + ch / 2.0),
                        Align2::LEFT_CENTER,
                        m.code,
                        &text(24.0),
                        p.gold,
                    );
                    jtext::draw(
                        &painter,
                        pos2(er.left() + 10.0, er.top() + 8.0 + ch * 1.5),
                        Align2::LEFT_CENTER,
                        m.name,
                        &text(14.0),
                        p.ink,
                    );
                    if playing > 0 {
                        jtext::draw(
                            &painter,
                            pos2(er.right() - 10.0, er.center().y),
                            Align2::RIGHT_CENTER,
                            &playing.to_string(),
                            &small(),
                            fixed::GREEN,
                        );
                    }
                    if playing > 0 {
                        let cr = Rect::from_min_size(pos2(er.right() - 40.0, er.top()), vec2(40.0, er.height()));
                        widgets::tip(
                            ui,
                            cr,
                            ("playing", m.code),
                            &format!("{playing} character{} playing it", if playing == 1 { "" } else { "s" }),
                        );
                    }
                    widgets::tip(
                        ui,
                        Rect::from_min_max(pos2(er.left(), er.center().y), pos2(er.right() - 40.0, er.bottom())),
                        ("mode-name", m.code),
                        m.summary,
                    );
                    if ui.interact(er, ui.scope_id().with(("mode", m.code)), Sense::click()).clicked() {
                        picked = Some(m.code.to_string());
                    }
                    y += h + 6.0;
                }
            },
        );
        if let Some(m) = picked {
            self.category = m;
        }
        if open_wizard && self.wizard.is_none() {
            self.wizard = Some(crate::game_mode_wizard::Wizard::new());
        }
        // the chosen mode on the right
        let mode = self.mode();
        let info = upe::info(&mode);
        let title = format!(
            "{} · {}{}{}",
            upe::label(&mode),
            if self.guild { "Guild " } else { "" },
            info.name,
            if self.no_pet { " · no pet" } else { "" }
        );
        let detail = Rect::from_min_max(pos2(list.right() + 8.0, all.top()), all.max);
        let cat_summary = CATEGORY_INFO.iter().find(|m| m.code == self.category).map_or(CATEGORY_INFO[1].summary, |m| m.summary);
        let mut variant = None;
        let mut toggles = (self.guild, self.no_pet);
        let icons = &mut *c.icons;
        let category = self.category.clone();
        let cur_variant = self.variant.clone();
        widgets::section(
            ui,
            &p,
            detail,
            &title,
            fixed::PURPLE,
            |_, _| {},
            |ui, r| {
                egui::ScrollArea::vertical().id_salt("game-modes").show(ui, |ui| {
                let painter = ui.painter().clone();
                let top = ui.cursor().min;
                let x = top.x + 16.0;
                let mut y = top.y + 12.0;
                let w = 640.0f32.min(r.width() - 32.0 - 12.0);
                y += wrapped(&painter, pos2(x, y), cat_summary, &text(18.0), w, p.ink);
                let heading = |painter: &egui::Painter, y: &mut f32, t: &str| {
                    let r = jtext::draw(painter, pos2(x, *y), Align2::LEFT_TOP, &t.to_uppercase(), &text(18.0), p.gold);
                    *y = r.bottom();
                };
                if category == "UPE" {
                    y += 12.0;
                    heading(&painter, &mut y, "Variant");
                    y += 4.0;
                    let names: Vec<String> = UPE_VARIANTS.iter().map(|v| upe::label(v)).collect();
                    let refs: Vec<&str> = names.iter().map(String::as_str).collect();
                    let sel = UPE_VARIANTS.iter().position(|v| *v == cur_variant).unwrap_or(0);
                    // each variant's name and summary on hover
                    let mut vx = x + 6.0;
                    for (k, v) in UPE_VARIANTS.iter().enumerate() {
                        let vw = widgets::segmented_width(&painter, &refs[k..=k], false);
                        let info = upe::info(v);
                        widgets::tip(ui, Rect::from_min_size(pos2(vx, y), vec2(vw, 36.0)), ("variant", k), &format!("{}: {}", info.name, info.summary));
                        vx += vw;
                    }
                    if let Some(i) = widgets::segmented(ui, &p, pos2(x + 6.0, y), 36.0, &refs, sel, false, &[]) {
                        variant = Some(UPE_VARIANTS[i].to_string());
                    }
                    y += 36.0;
                }
                y += 10.0;
                for (k, label) in [format!("Guild version ({}): trade with Loot Gobbler guild mates on a guild run", upe::label(&upe::with_guild(&mode, true))), "No pet (NPE): the whole run without a pet".to_string()].iter().enumerate() {
                    let on = if k == 0 { toggles.0 } else { toggles.1 };
                    let lh = jtext::height(&painter, &text(15.0)).max(20.0);
                    let bx = Rect::from_min_size(pos2(x + 2.0, y + (lh - 15.0) / 2.0), vec2(15.0, 15.0));
                    painter.rect(bx, 0.0, if on { p.check } else { p.paper }, Stroke::new(1.0, p.line), StrokeKind::Inside);
                    if on {
                        painter.line_segment([pos2(bx.left() + 3.0, bx.center().y), pos2(bx.center().x - 1.0, bx.bottom() - 4.0)], Stroke::new(2.0, p.ink));
                        painter.line_segment([pos2(bx.center().x - 1.0, bx.bottom() - 4.0), pos2(bx.right() - 3.0, bx.top() + 3.0)], Stroke::new(2.0, p.ink));
                    }
                    let tr = jtext::draw(&painter, pos2(x + 24.0, y + lh / 2.0), Align2::LEFT_CENTER, label, &text(15.0), p.ink);
                    if ui.interact(Rect::from_min_max(bx.min, tr.max), ui.scope_id().with(("toggle", k)), Sense::click()).clicked() {
                        if k == 0 {
                            toggles.0 = !toggles.0;
                        } else {
                            toggles.1 = !toggles.1;
                        }
                    }
                    y += lh;
                }
                y += 8.0;
                y += chip_row(&painter, pos2(x, y), "Loadout", &loadout_chips(&mode));
                y += chip_row(&painter, pos2(x, y), "From", &source_chips(toggles.0));
                // the rules in full
                y += 8.0;
                y += wrapped(&painter, pos2(x, y), info.summary, &text(15.0), w, fixed::MUTED);
                y += 12.0;
                heading(&painter, &mut y, "Before a run starts");
                y += wrapped(&painter, pos2(x, y), &format!("•  {PREREQUISITE}"), &text(15.0), w, p.ink);
                if self.no_pet {
                    y += wrapped(&painter, pos2(x, y), "•  It has no pet: the character list shows none following it.", &text(15.0), w, p.ink);
                }
                y += 12.0;
                heading(&painter, &mut y, "Rules");
                for rule in std::iter::once(info.first_rule).chain(OWN_RULES) {
                    y += wrapped(&painter, pos2(x, y), &format!("•  {rule}"), &text(15.0), w, p.ink);
                }
                if upe::guild(&mode) {
                    y += 8.0;
                    heading(&painter, &mut y, "Guild version");
                    for rule in GUILD_RULES {
                        y += wrapped(&painter, pos2(x, y), &format!("•  {rule}"), &text(15.0), w, p.ink);
                    }
                }
                if self.no_pet {
                    y += 8.0;
                    heading(&painter, &mut y, "No pet");
                    for rule in NO_PET_RULES {
                        y += wrapped(&painter, pos2(x, y), &format!("•  {rule}"), &text(15.0), w, p.ink);
                    }
                }
                // the characters: playing, could start, lost
                let mine: Vec<&&upe::Run> = runs.iter().filter(|x| upe::category(&x.mode) == category && app.characters.get(x.char_id).is_some()).collect();
                let playing: Vec<&upe::Run> = mine.iter().filter(|x| x.marked && x.broken.is_none() && alive(x.char_id)).map(|x| **x).collect();
                let could: Vec<&upe::Run> = runs.iter().filter(|x| app.characters.get(x.char_id).is_some() && x.potential() && alive(x.char_id)).copied().collect();
                let lost: Vec<&upe::Run> = mine.iter().filter(|x| x.marked && (x.broken.is_some() || !alive(x.char_id))).map(|x| **x).collect();
                let card_w = r.width() - 32.0 - 12.0;
                for (title, group, empty) in [
                    ("Playing", &playing, Some(format!("No character is playing a {category} run."))),
                    ("Could start", &could, Some("Create a character with the sniffer running and upgrades disabled: it can start a run until it breaks a rule.".to_string())),
                    ("Lost", &lost, None),
                ] {
                    if title == "Lost" && group.is_empty() {
                        continue;
                    }
                    y += 18.0;
                    heading(&painter, &mut y, &format!("{title}{}", if group.is_empty() { String::new() } else { format!("  ({})", group.len()) }));
                    y += 6.0;
                    if group.is_empty()
                        && let Some(e) = &empty
                    {
                        y += wrapped(&painter, pos2(x, y), e, &text(15.0), w, fixed::MUTED);
                    }
                    for run in group.iter() {
                        let ch = app.characters.get(run.char_id).expect("checked");
                        y += card(ui, &p, app, icons, run, ch, &mode, toggles.1, Rect::from_min_size(pos2(x, y), vec2(card_w, 0.0))) + 6.0;
                    }
                }
                ui.allocate_exact_size(vec2(r.width() - 12.0, y + 16.0 - top.y), Sense::hover());
            });
            },
        );
        if let Some(v) = variant {
            self.variant = v;
        }
        self.guild = toggles.0;
        self.no_pet = toggles.1;
    }
}

/// One character in a mode (Java `card`): its look, where it stands, what can be done with it; gives its height.
fn card(
    ui: &Ui,
    p: &Palette,
    app: &lg_app::App,
    icons: &mut crate::icons::Icons,
    r: &upe::Run,
    c: &lg_app::characters::Char,
    mode: &str,
    no_pet: bool,
    at: Rect,
) -> f32 {
    let painter = ui.painter().clone();
    let data = app.world.data();
    let zone = &app.zone;
    let lvl = if c.level < 20 { format!("Level {}", c.level.max(1)) } else { format!("{} Fame", num(c.fame)) };
    let name = format!(
        "{}   {lvl}{}",
        c.class_name,
        if r.marked { format!("   ·   {}{}", upe::label(&r.mode), if r.no_pet { " · no pet" } else { "" }) } else { String::new() }
    );
    let status = if !c.alive {
        format!("Died{}", r.broken.as_ref().map_or(String::new(), |b| format!(" (the run was lost before: {b})")))
    } else if let Some(b) = &r.broken {
        format!("Lost {}: {b}", date(zone, r.broken_at))
    } else if r.marked {
        format!("Playing since {}", date(zone, r.marked_at))
    } else {
        "Seen from its creation, nothing against the rules yet".to_string()
    };
    let n = r.gathered.len();
    let more = format!("{n} item{} gathered", if n == 1 { "" } else { "s" });

    // the buttons, right to left at the card's right edge
    use crate::window::Ask;
    let id = r.char_id;
    // each button: its label, primary, enabled, what it does (an ask, or a page to open)
    let mut buttons: Vec<(String, bool, bool, Result<Ask, String>)> = Vec::new();
    // the tooltips of the greyed out ones, and of Allow a pet (by label)
    let mut tips: Vec<(String, String)> = Vec::new();
    if c.alive && r.potential() {
        let ask = Ask::UpeMark { char_id: id, on: true, mode: Some(mode.to_string()), no_pet: Some(no_pet) };
        let label = format!("Start {}{}", upe::label(mode), if no_pet { " without pet" } else { "" });
        if !r.can_start(mode, no_pet) {
            tips.push((label.clone(), "It has a pet: a run without a pet cannot start while one follows it".into()));
        }
        buttons.push((label, true, r.can_start(mode, no_pet), Ok(ask)));
    }
    if c.alive && r.marked && r.broken.is_none() {
        let on = upe::guild(&r.mode);
        let other = upe::with_guild(&r.mode, !on);
        let label = format!("{}{}", if on { "Guild off: " } else { "Guild on: " }, upe::label(&other));
        if on && r.guild_locked {
            tips.push((label.clone(), format!("It equipped or used an item traded from a guild mate: it stays {}", upe::label(&r.mode))));
        }
        buttons.push((label, false, !(on && r.guild_locked), Ok(Ask::UpeMode(id, other))));
    }
    if c.alive && r.marked && r.broken.is_none() && r.no_pet {
        tips.push(("Allow a pet".into(), "Ends the no-pet part of the run: it cannot be turned on again".into()));
        buttons.push(("Allow a pet".into(), false, true, Ok(Ask::UpeNoPet(id, false))));
    }
    if c.alive && r.marked && r.broken.is_none() {
        buttons.push(("End run".into(), false, true, Ok(Ask::UpeMark { char_id: id, on: false, mode: None, no_pet: None })));
    }
    if r.marked {
        buttons.push(("Finds".into(), false, true, Err(format!("Inventory:upe:{id}"))));
    }
    buttons.push(("Open".into(), false, true, Err(format!("Characters:{id}"))));
    // what it gathered that this class can equip (each item once, first found first)
    let class_slots = data.classes.iter().find(|k| k.id == c.class_type).and_then(|k| k.slot_types);
    let mut usable: Vec<i32> = Vec::new();
    for g in &r.gathered {
        if usable.contains(&g.item) {
            continue;
        }
        if let (Some(it), Some(slots)) = (data.items.item(g.item), class_slots)
            && it.slot_type != 0
            && slots.contains(&it.slot_type)
        {
            usable.push(g.item);
        }
    }
    const USABLE_ICON: f32 = 32.0;
    let usable_h = if usable.is_empty() { 0.0 } else { jtext::height(&painter, &text(13.0)) + 4.0 + USABLE_ICON + 4.0 };
    // while the run stands: the missions it is solving alone, those still at 0% it could, and what it may take from
    // the gift chest (rewards of missions and quests it solved alone that fit its mode)
    let standing = r.marked && r.broken.is_none() && c.alive;
    let mut mission_lines: Vec<(String, Vec<crate::widgets::TipLine>)> = Vec::new();
    // its bank: the forge resources its character gained while playing the run (counted when the amounts rose)
    if let Some(bank) = app.resources.banks.get(&r.char_id).filter(|b| !b.is_empty()) {
        let mut parts: Vec<String> = Vec::new();
        for (k, v) in bank.dust.iter().filter(|(_, v)| **v > 0) {
            parts.push(format!("+{} {}", num(*v), crate::inventory::dust_name(*k)));
        }
        for (k, v) in bank.materials.iter().filter(|(_, v)| **v > 0) {
            parts.push(format!("+{} {}", num(*v), crate::inventory::material_name(*k)));
        }
        if bank.forgefire > 0 {
            parts.push(format!("+{} Forgefire", num(bank.forgefire)));
        }
        let tip = vec![vec![(
            "What this run's character gained while playing it: its own, whatever the account's amounts are now".into(),
            None,
            false,
        )]];
        mission_lines.push((format!("Its own forge resources: {}", parts.join(", ")), tip));
    }
    if standing {
        let name_of = |k: &str| app.missions.by_key(k).and_then(|m| m.name.clone()).unwrap_or_else(|| format!("mission {k}"));
        let working: Vec<String> =
            app.upe.missions.iter().filter(|(_, m)| m.char_id == r.char_id && m.clean && m.from_zero).map(|(k, _)| name_of(k)).collect();
        if !working.is_empty() {
            let tip = working.iter().map(|n| vec![(n.clone(), None, false)]).collect();
            mission_lines.push((format!("Solving alone: {}", working.join(", ")), tip));
        }
        let now_s = app.world.now() / 1000;
        let untouched = app.missions.untouched(now_s);
        if app.missions.defs.is_some() {
            let mut tip: Vec<crate::widgets::TipLine> =
                vec![vec![("Still at 0%: done from the start by this character alone, their rewards are its own".into(), None, true)]];
            for (_, m) in untouched.iter().take(20) {
                let n = m.name.clone().unwrap_or_default();
                let d = m.desc.clone().unwrap_or_default();
                tip.push(vec![(n, None, false), (if d.is_empty() { String::new() } else { format!(": {d}") }, Some(fixed::MUTED), false)]);
            }
            if untouched.len() > 20 {
                tip.push(vec![(format!("and {} more", untouched.len() - 20), Some(fixed::MUTED), false)]);
            }
            mission_lines.push((format!("Missions it can still do alone: {} (hover for the list)", untouched.len()), tip));
        } else {
            mission_lines.push(("Missions: not loaded yet (they come with the game's login)".into(), Vec::new()));
        }
    }
    let gift: Vec<i32> = r.allowed.iter().filter_map(|g| g.first().copied()).collect();
    let gift_h = if gift.is_empty() { 0.0 } else { jtext::height(&painter, &text(13.0)) + 4.0 + USABLE_ICON + 4.0 };
    let mission_h = mission_lines.len() as f32 * (jtext::height(&painter, &text(13.0)) + 4.0);
    let lines = 3 + r.notes.len();
    let text_h = jtext::height(&painter, &text(22.0))
        + jtext::height(&painter, &text(15.0))
        + 4.0
        + (lines - 2) as f32 * (jtext::height(&painter, &text(13.0)) + 4.0)
        + usable_h
        + mission_h
        + gift_h;
    let h = 8.0 + text_h.max(56.0) + 8.0;
    let rect = Rect::from_min_size(at.min, vec2(at.width(), h));
    painter.rect_filled(rect, 0.0, p.paper);
    painter.rect_stroke(rect, 0.0, Stroke::new(1.0, p.line), StrokeKind::Inside);
    let skin = if c.skin > 0 { c.skin } else { c.class_type };
    if let Some(t) = icons.skin(ui.ctx(), data, skin, c.tex1, c.tex2, 56) {
        painter.image(
            t.id(),
            Rect::from_min_size(pos2(rect.left() + 10.0, rect.center().y - 28.0), vec2(56.0, 56.0)),
            crate::detail::uv(),
            if c.alive { Color32::WHITE } else { Color32::from_white_alpha(90) },
        );
    }
    let tx = rect.left() + 10.0 + 56.0 + 14.0;
    let mut ty = rect.top() + 8.0 + (h - 16.0 - text_h) / 2.0;
    let t = jtext::draw(&painter, pos2(tx, ty), Align2::LEFT_TOP, &name, &text(22.0), p.ink);
    ty = t.bottom();
    let t = jtext::draw(
        &painter,
        pos2(tx, ty + 2.0),
        Align2::LEFT_TOP,
        &status,
        &text(15.0),
        if r.broken.is_some() || !c.alive { fixed::MUTED } else { p.ink },
    );
    ty = t.bottom() + 2.0;
    for line in std::iter::once(more).chain(r.notes.iter().map(|n| format!("Worth a look: {n}"))) {
        let t = jtext::draw(&painter, pos2(tx, ty + 2.0), Align2::LEFT_TOP, &line, &text(13.0), fixed::MUTED);
        ty = t.bottom() + 2.0;
    }
    if !usable.is_empty() {
        let t = jtext::draw(
            &painter,
            pos2(tx, ty + 2.0),
            Align2::LEFT_TOP,
            &format!("Usable by this {} ({})", c.class_name, usable.len()),
            &text(13.0),
            p.ink,
        );
        ty = t.bottom() + 4.0;
        // as many as fit before the buttons; the rest counted
        let room = (rect.right() - 10.0 - 6.0 * 108.0 - tx).max(USABLE_ICON * 4.0);
        let fit = ((room / (USABLE_ICON + 4.0)) as usize).max(1);
        let shown = if usable.len() > fit { fit - 1 } else { usable.len() };
        for (k, id) in usable.iter().take(shown).enumerate() {
            let ir = Rect::from_min_size(pos2(tx + k as f32 * (USABLE_ICON + 4.0), ty), vec2(USABLE_ICON, USABLE_ICON));
            if let Some(t) = icons.object(ui.ctx(), data, *id, USABLE_ICON as u32) {
                painter.image(t.id(), ir, crate::detail::uv(), Color32::WHITE);
            }
            let slot = data.items.item(*id).map_or("", |it| it.slot_name());
            crate::widgets::item_tip(ui, ir, data, *id, None, slot);
        }
        if shown < usable.len() {
            jtext::draw(
                &painter,
                pos2(tx + shown as f32 * (USABLE_ICON + 4.0) + 4.0, ty + USABLE_ICON / 2.0),
                Align2::LEFT_CENTER,
                &format!("+{}", usable.len() - shown),
                &text(15.0),
                fixed::MUTED,
            );
        }
        ty += USABLE_ICON + 4.0;
    }
    for (k, (line, tip)) in mission_lines.iter().enumerate() {
        let t = jtext::draw(&painter, pos2(tx, ty + 2.0), Align2::LEFT_TOP, line, &text(13.0), p.ink);
        if !tip.is_empty() {
            crate::widgets::rich_tip(ui, t, ("mission-tip", r.char_id, k), tip);
        }
        ty = t.bottom() + 2.0;
    }
    if !gift.is_empty() {
        let t = jtext::draw(
            &painter,
            pos2(tx, ty + 2.0),
            Align2::LEFT_TOP,
            &format!("May take from the gift chest ({}): rewards it earned alone", gift.len()),
            &text(13.0),
            p.ink,
        );
        ty = t.bottom() + 4.0;
        for (k, id) in gift.iter().enumerate() {
            let ir = Rect::from_min_size(pos2(tx + k as f32 * (USABLE_ICON + 4.0), ty), vec2(USABLE_ICON, USABLE_ICON));
            if let Some(t) = icons.object(ui.ctx(), data, *id, USABLE_ICON as u32) {
                painter.image(t.id(), ir, crate::detail::uv(), Color32::WHITE);
            }
            let slot = data.items.item(*id).map_or("", |it| it.slot_name());
            crate::widgets::item_tip(ui, ir, data, *id, None, slot);
        }
    }
    let mut bx = rect.right() - 10.0;
    for (i, (label, primary, enabled, action)) in buttons.iter().enumerate().rev() {
        // FlatLaf buttons are at least 72 px wide, scaled
        let bw = (jtext::width(&painter, &label.to_uppercase(), &small()) + 32.0).max(108.0);
        bx -= bw;
        let br = Rect::from_min_size(pos2(bx, rect.center().y - 18.0), vec2(bw, 36.0));
        let clicked = if !*enabled {
            // greyed out, as a disabled button
            painter.rect(br, 0.0, p.header, Stroke::new(1.5, p.line), StrokeKind::Inside);
            jtext::draw(&painter, br.center(), Align2::CENTER_CENTER, &label.to_uppercase(), &small(), p.ink_muted);
            false
        } else if *primary {
            painter.rect(br, 0.0, p.gold, Stroke::new(1.5, Color32::BLACK), StrokeKind::Inside);
            jtext::draw(&painter, br.center(), Align2::CENTER_CENTER, &label.to_uppercase(), &small(), p.on_accent);
            ui.interact(br, ui.scope_id().with(("mode-start", r.char_id, i)), egui::Sense::click()).clicked()
        } else {
            crate::inventory::button(ui, p, br, label, false, &format!("mode-{}-{i}", r.char_id))
        };
        if let Some((_, t)) = tips.iter().find(|(l, _)| l == label) {
            crate::widgets::tip(ui, br, ("mode-tip", r.char_id, i), t);
        }
        if clicked {
            match action {
                Ok(ask) => crate::window::ask_later(ui.ctx(), ask.clone()),
                Err(spec) => crate::window::open_later(ui.ctx(), spec.clone()),
            }
        }
        bx -= 6.0;
    }
    h
}
