//! The Profile tab (Java: `ProfileView`): pages Overview, Quests, Dungeon Stats; the overview's cards about you, the
//! last 24 hours, recent drops and runs.

use crate::icons::Icons;
use crate::theme::{Palette, fixed};
use crate::widgets::{self, caption, heading, label, muted, row, soft, text};
use egui::{Color32, Rect, Sense, Ui, pos2, vec2};
use lg_app::App;
use lg_app::characters::{Char, Characters, DAY_MS};

const RECENT: usize = 5;
const AMULET: i32 = 23624; // Amulet of Restoration
const AMULET_DAYS: i64 = 3;

const GREETINGS: [&str; 18] = [
    "Hello",
    "Hi",
    "Hey",
    "Howdy",
    "Aye",
    "Ahoy",
    "Yo",
    "Greetings",
    "Welcome back",
    "Good to see you",
    "Hail",
    "Salutations",
    "Well met",
    "Heyo",
    "Oi",
    "Ello",
    "Look who's back",
    "Ready to gobble",
];

/// What the view needs besides its own state.
pub struct Ctx<'a> {
    pub app: &'a App,
    pub p: &'a Palette,
    pub icons: &'a mut Icons,
    pub now: i64,
    /// The game's enchantments, for the suggested loadout.
    pub en: &'a lg_app::loadout::Enchantments,
    /// The suggested loadout's settings (what you aim for, the scope...).
    pub loadout: &'a mut lg_app::loadout::Settings,
    /// The app's settings file (`realmShark.properties`).
    pub props: &'a lg_app::properties::Properties,
    /// What the views ask for this frame: settings to save, things to do to the records.
    pub asks: &'a mut Vec<crate::window::Ask>,
}

impl Ctx<'_> {
    /// A setting to save (Java `PropertiesManager.setProperties`).
    pub fn set(&mut self, key: &str, value: &str) {
        self.asks.push(crate::window::Ask::Set(key.to_string(), value.to_string()));
    }
}

pub struct Profile {
    pub page: usize,
    quests: crate::quests::QuestsView,
    greeting: &'static str,
    /// The frame it was last drawn in, the account then and when the guild card was last asked for (Java loads it
    /// when shown, after logging in or out, every minute and after your uploads).
    last_pass: u64,
    last_account: String,
    guild_asked: i64,
}

impl Default for Profile {
    fn default() -> Self {
        let pick = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.subsec_nanos()) as usize;
        Profile {
            page: 0,
            quests: Default::default(),
            greeting: GREETINGS[pick % GREETINGS.len()],
            last_pass: 0,
            last_account: String::new(),
            guild_asked: 0,
        }
    }
}

/// A drop of a collectible item (Java `DropRow`).
struct Drop {
    item: i32,
    bag: String,
    r#where: String,
    time: i64,
}

/// Java `CharactersView.ago`.
pub fn ago(now: i64, t: i64) -> String {
    let m = ((now - t) / 60_000).max(0);
    if m < 60 {
        format!("{m} min ago")
    } else if m < 60 * 24 {
        format!("{} h ago", m / 60)
    } else {
        format!("{} d ago", m / 60 / 24)
    }
}

/// Java's `NumberFormat.getIntegerInstance()` (English grouping).
pub fn num(n: i64) -> String {
    let s = n.abs().to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    if n < 0 { format!("-{out}") } else { out }
}

fn duration(ms: i64) -> String {
    let m = ms / 60_000;
    if m < 60 { format!("{m}m") } else { format!("{}h {}m", m / 60, m % 60) }
}

fn clock(ms: i64) -> String {
    let s = ms / 1000;
    format!("{}:{:02}", s / 60, s % 60)
}

fn short_dps(v: f64) -> String {
    if v <= 0.0 {
        "-".into()
    } else if v >= 1000.0 {
        format!("{:.1}k", v / 1000.0)
    } else {
        num(v.round() as i64)
    }
}

fn base_bag(bag: &str) -> &str {
    bag.strip_prefix("Boosted ").unwrap_or(bag)
}

/// The bag's object id from its stored name ("White", "Boosted Orange").
pub(crate) fn bag_id(bag: &str) -> Option<i32> {
    if bag.is_empty() {
        return None;
    }
    let name = match bag.strip_prefix("Boosted ") {
        Some(b) => format!("B.{b}"),
        None => bag.to_string(),
    };
    lg_gamedata::LOOT_BAGS.iter().find(|(_, n)| *n == name).map(|(id, _)| *id)
}

/// Every recorded drop, newest first (Java `allDrops`).
fn all_drops(app: &App) -> Vec<Drop> {
    let mut out = Vec::new();
    for (item, e) in app.collection.all() {
        for f in e.history.iter().flatten() {
            if f.source == lg_app::collection::DROP
                && let Some(t) = app.zone.parse_local(&f.time)
            {
                out.push(Drop { item, bag: f.bag.clone(), r#where: f.r#where.clone(), time: t });
            }
        }
    }
    out.sort_by_key(|d| std::cmp::Reverse(d.time));
    out
}

/// The page buttons above the tab's pages (Java `Segmented` in `SubTabs`); the last one greyed out.
pub fn pages(ui: &mut Ui, p: &Palette, names: &[&str], selected: &mut usize, disabled: &[usize]) {
    let font = widgets::small();
    let painter = ui.painter().clone();
    let top = ui.cursor().min;
    let mut x = top.x + 7.0;
    for (i, n) in names.iter().enumerate() {
        let t = n.to_uppercase();
        let w = crate::jtext::width(&painter, &t, &font) + 30.0;
        let r = egui::Rect::from_min_size(pos2(x, top.y), vec2(w, 36.0));
        let off = disabled.contains(&i);
        let resp = ui.interact(r, ui.scope_id().with(("page", i)), if off { Sense::hover() } else { Sense::click() });
        let on = *selected == i;
        let (bg, fg) = if on {
            (p.gold, p.on_accent)
        } else if off {
            (p.paper, p.ink_muted)
        } else if resp.hovered() {
            (p.header, p.ink_soft)
        } else {
            (p.paper, p.ink_soft)
        };
        painter.rect(r, 0.0, bg, egui::Stroke::new(1.5, Color32::BLACK), egui::StrokeKind::Inside);
        crate::jtext::draw(&painter, r.center(), egui::Align2::CENTER_CENTER, &t, &font, fg);
        if resp.clicked() {
            *selected = i;
        }
        x += w + 8.0;
    }
    ui.allocate_exact_size(vec2(ui.available_width(), 36.0), Sense::hover());
}

impl Profile {
    pub fn show(&mut self, ui: &mut Ui, c: &mut Ctx, rarity: &crate::detail::Rarity) {
        let p = *c.p;
        let pass = ui.ctx().cumulative_pass_nr();
        let account = c.app.server.account();
        let shown = pass > self.last_pass + 1;
        if shown && c.app.server.profile_guild.stale
            || account != self.last_account
            || c.now - self.guild_asked >= 60_000
            || (self.guild_asked == 0)
        {
            self.guild_asked = c.now;
            c.asks.push(crate::window::Ask::Server(crate::window::ServerAsk::ProfileGuildLoad));
        }
        self.last_pass = pass;
        self.last_account = account;
        let all = ui.max_rect().shrink(8.0);
        let mut ui = ui.new_child(egui::UiBuilder::new().max_rect(all).layout(egui::Layout::top_down(egui::Align::Min)));
        ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
        pages(&mut ui, &p, &["Overview", "Quests", "Dungeon Stats"], &mut self.page, &[2]);
        ui.add_space(8.0);
        match self.page {
            0 => {
                egui::ScrollArea::vertical().id_salt("profile-overview").show(&mut ui, |ui| {
                    ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                    let inner = ui.available_rect_before_wrap();
                    // Java's scroll bar takes 12 px at the right
                    let body =
                        egui::Rect::from_min_max(pos2(inner.left() + 20.0, inner.top() + 16.0), pos2(inner.right() - 32.0, inner.bottom()));
                    let mut b = ui.new_child(egui::UiBuilder::new().max_rect(body).layout(egui::Layout::top_down(egui::Align::Min)));
                    b.spacing_mut().item_spacing = vec2(0.0, 0.0);
                    self.overview(&mut b, c, rarity);
                    b.add_space(20.0);
                    ui.allocate_exact_size(vec2(inner.width(), b.min_rect().height() + 16.0), Sense::hover());
                });
            }
            1 => {
                let body = ui.available_rect_before_wrap();
                self.quests.show(&mut ui, c, body);
            }
            _ => {
                muted(&mut ui, "Coming soon.");
            }
        }
    }

    fn overview(&self, ui: &mut Ui, c: &mut Ctx, rarity: &crate::detail::Rarity) {
        let p = *c.p;
        let app = c.app;
        let now = c.now;
        let props = c.props;
        let since = now - DAY_MS;
        let drops = all_drops(app);
        let runs: Vec<&lg_app::dps_history::Run> = app.dps_history.runs.iter().filter(|r| !r.hidden).collect();
        let greeting = self.greeting;
        row(
            ui,
            &p,
            16.0,
            &mut *c.icons,
            vec![
                Box::new(|ui: &mut Ui, icons: &mut Icons| player_card(ui, &p, app, icons, now, greeting)),
                Box::new(|ui: &mut Ui, icons: &mut Icons| crate::profile_guild::guild_card(ui, &p, app, icons, props, rarity, now)),
            ],
        );
        ui.add_space(20.0);
        // suggestions of what to do next (each a box; their content comes later)
        heading(ui, &p, "What you can do");
        ui.add_space(8.0);
        let ideas: Vec<widgets::Card<'_, ()>> =
            (0..SUGGESTIONS).map(|_| -> widgets::Card<'_, ()> { Box::new(|ui: &mut Ui, _: &mut ()| suggestion(ui, &p)) }).collect();
        row(ui, &p, 12.0, &mut (), ideas);
        ui.add_space(20.0);
        heading(ui, &p, "Last 24 hours");
        ui.add_space(8.0);
        today_tiles(ui, &p, app, &runs, &drops, now, since);
        ui.add_space(20.0);
        row(
            ui,
            &p,
            16.0,
            &mut *c.icons,
            vec![
                Box::new(|ui: &mut Ui, icons: &mut Icons| recent_drops(ui, &p, app, icons, &drops, now)),
                Box::new(|ui: &mut Ui, icons: &mut Icons| recent_runs(ui, &p, app, icons, &runs, &drops)),
            ],
        );
        ui.add_space(20.0);
        row(
            ui,
            &p,
            16.0,
            &mut (),
            vec![
                Box::new(|ui: &mut Ui, _: &mut ()| top_dungeons(ui, &p, app)),
                Box::new(|ui: &mut Ui, _: &mut ()| exaltation(ui, &p, app)),
                Box::new(|ui: &mut Ui, _: &mut ()| {
                    heading(ui, &p, "Daily quests");
                    ui.add_space(6.0);
                    muted(ui, "Coming soon from the Loot Gobbler server.");
                }),
            ],
        );
    }
}

fn player_card(ui: &mut Ui, p: &Palette, app: &App, icons: &mut Icons, now: i64, greeting: &str) {
    let chars = &app.characters;
    let name = chars.player_name.clone();
    // the greeting (soft) before the name (gold), one line in the head font at 56, smaller (to 28) if it does not fit
    let first = format!("{greeting},");
    let second = format!(" {}", name.as_deref().unwrap_or("stranger"));
    let room = ui.available_width();
    let painter = ui.painter().clone();
    let natural = crate::jtext::width(&painter, &format!("{first}{second}"), &widgets::head_px(56.0));
    let size = if natural <= room { 56.0 } else { (56.0 * room / natural).floor().max(28.0) };
    let f = widgets::head_px(size);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
        label(ui, &first, &f, p.ink_soft);
        label(ui, &second, &f, p.gold);
    });
    if name.is_none() {
        muted(ui, "Start the sniffer and enter the game once to show your name.");
    }
    ui.add_space(14.0);

    let most = chars.most_played(now);
    let amulet = lg_app::amulet_saves::AmuletSaves::worn_recently(chars, app.world.data(), now) || amulet_saves(app, now) > 0;
    let top = ui.cursor().min;
    let full = ui.available_width();
    // the most played character at the left
    let mut left = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(egui::Rect::from_min_size(top, vec2(full, f32::INFINITY)))
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    left.spacing_mut().item_spacing = vec2(0.0, 0.0);
    caption(&mut left, p, "MOST PLAYED CHARACTER");
    left.add_space(6.0);
    match most {
        None => {
            muted(&mut left, "Log in once so your character list is loaded.");
        }
        Some(m) => character_block(&mut left, p, app, icons, m, now),
    }
    let mut height = left.min_rect().height();
    if let Some(m) = most {
        let x = left.min_rect().right() + 32.0;
        let r = egui::Rect::from_min_size(pos2(x, top.y), vec2((top.x + full - x).max(0.0), f32::INFINITY));
        let mut col = ui.new_child(egui::UiBuilder::new().max_rect(r).layout(egui::Layout::top_down(egui::Align::Min)));
        col.spacing_mut().item_spacing = vec2(0.0, 0.0);
        if recent_exalts(&mut col, p, app, icons, m, now) {
            height = height.max(col.min_rect().height());
        }
    }
    // the maybe-showables at the right: the Amulet of Restoration
    if amulet {
        let n = amulet_saves(app, now);
        let w = 160.0;
        let r = egui::Rect::from_min_size(pos2(top.x + full - w, top.y), vec2(w, f32::INFINITY));
        let mut col = ui.new_child(egui::UiBuilder::new().max_rect(r).layout(egui::Layout::top_down(egui::Align::Center)));
        col.spacing_mut().item_spacing = vec2(0.0, 0.0);
        let tex = icons.object(ui.ctx(), app.world.data(), AMULET, 56);
        widgets::icon(&mut col, tex.as_ref(), 56.0);
        col.add_space(4.0);
        let line1 = format!("saved you {n} time{}", if n == 1 { "" } else { "s" });
        label(&mut col, &line1, &text(18.0), p.ink_soft);
        label(&mut col, &format!("in the last {AMULET_DAYS} days"), &text(18.0), p.ink_soft);
        widgets::tip(
            ui,
            col.min_rect(),
            "amulet-saves",
            &format!("Your Amulet of Restoration kept you alive {n} time{} in the last {AMULET_DAYS} days", if n == 1 { "" } else { "s" }),
        );
        height = height.max(col.min_rect().height());
    }
    ui.allocate_exact_size(vec2(full, height), Sense::hover());

    if let Some(m) = most {
        ui.add_space(16.0);
        fame_gain(ui, p, app, icons, chars, m, now);
    }
    // the Newcomer's Guide frames all of the card
    let m = ui.max_rect();
    crate::guide::mark(
        ui,
        "profile.overview",
        egui::Rect::from_min_max(pos2(m.left() - 18.0, m.top() - 14.0), pos2(m.right() + 18.0, ui.min_rect().bottom() + 14.0)),
    );
}

const EXALT_DAYS: i64 = 7;
const EXALT_CLASSES: usize = 8;
/// Potion sprites by stat (Java `ClassesView.STAT_POTION`), and the stats in the order shown (life first).
const STAT_POTION: [i32; 8] = [0xae9, 0xaea, 0xa1f, 0xa20, 0xa21, 0xa4c, 0xa34, 0xa35];
const EXALT_POTION: [usize; 8] = [5, 4, 6, 7, 3, 2, 1, 0];
const ORDER: [usize; 8] = [7, 6, 5, 4, 1, 0, 2, 3]; // life, mana, att, def, spd, dex, vit, wis

/// The exaltation progress classes got in the last days (Java `recentExalts`): your most played class first, then
/// the most progress; a class sprite and per stat "Nx" with its potion. False when no class got any.
fn recent_exalts(ui: &mut Ui, p: &Palette, app: &App, icons: &mut Icons, most: &Char, now: i64) -> bool {
    let since = now - EXALT_DAYS * DAY_MS;
    let cap = *lg_app::exalt_store::THRESHOLDS.last().expect("thresholds");
    let data = app.world.data();
    let mut gained: Vec<(i32, [i32; 8])> = Vec::new();
    for cls in &data.classes {
        let Some(progress) = app.exalts.progress(cls.id) else { continue };
        let mut got = [0; 8];
        for (stat, g) in got.iter_mut().enumerate().take(progress.len().min(8)) {
            let then = app.exalts.progress_at(&app.dungeon_visits, cls.id, stat as i32, since);
            *g = (progress[stat].min(cap) - then.min(cap)).max(0);
        }
        if got.iter().sum::<i32>() > 0 {
            gained.push((cls.id, got));
        }
    }
    if gained.is_empty() {
        return false;
    }
    gained.sort_by_key(|(id, got)| (*id != most.class_type, -got.iter().sum::<i32>()));
    let start = ui.cursor().min;
    let head = caption(ui, p, &format!("EXALTATIONS ({EXALT_DAYS} DAYS)"));
    if ui
        .interact(head.rect, ui.scope_id().with("open-classes"), Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text("Open the Classes page")
        .clicked()
    {
        crate::window::open_later(ui.ctx(), "Characters:classes");
    }
    ui.add_space(4.0);
    // Java `DpsLoggerView.EXALT_STATS`
    const STAT_NAMES: [&str; 8] = ["DEX", "SPD", "VIT", "WIS", "DEF", "ATT", "MP", "HP"];
    for (id, got) in gained.iter().take(EXALT_CLASSES) {
        let row = ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            let tex = icons.object(ui.ctx(), data, *id, 32);
            widgets::icon(ui, tex.as_ref(), 32.0);
            ui.add_space(10.0);
            let mut shown = 0;
            for stat in ORDER {
                if got[stat] <= 0 {
                    continue;
                }
                if shown > 0 {
                    ui.add_space(12.0);
                }
                label(ui, &format!("{}x", got[stat]), &text(18.0), p.ink);
                ui.add_space(3.0);
                let tex = icons.object(ui.ctx(), data, STAT_POTION[EXALT_POTION[stat]], 20);
                widgets::icon(ui, tex.as_ref(), 20.0);
                shown += 1;
            }
        });
        // click: the class in Classes, its progress on hover
        let name = data.classes.iter().find(|k| k.id == *id).map_or(String::new(), |k| k.name.clone());
        let mut tip = format!("{name}  ·  exaltation progress in the last {EXALT_DAYS} days:");
        for stat in ORDER {
            if got[stat] > 0 {
                tip += &format!("\n{} +{}", STAT_NAMES[stat], got[stat]);
            }
        }
        tip += "\nclick to open it in Classes";
        if ui
            .interact(row.response.rect, ui.scope_id().with(("open-class", *id)), Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .on_hover_text(tip)
            .clicked()
        {
            crate::window::open_later(ui.ctx(), format!("Characters:class:{id}"));
        }
    }
    let _ = start;
    true
}

fn amulet_saves(app: &App, now: i64) -> usize {
    app.amulet_saves.count_since(now - AMULET_DAYS * DAY_MS)
}

/// The character's skin, class, stats maxed and fame (Java `characterBlock`).
fn character_block(ui: &mut Ui, p: &Palette, app: &App, icons: &mut Icons, c: &Char, now: i64) {
    let block = ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
        let id = if c.skin > 0 { c.skin } else { c.class_type };
        let tex = icons.skin(ui.ctx(), app.world.data(), id, c.tex1, c.tex2, 96);
        widgets::icon(ui, tex.as_ref(), 96.0);
        ui.add_space(18.0);
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            label(ui, &format!("{}   {}/8", c.class_name, c.stats_maxed), &text(30.0), p.ink);
            let progress = if c.level < 20 { format!("Level {}", c.level) } else { format!("{} Fame", num(c.fame)) };
            label(ui, &progress, &text(22.0), p.gold);
            if Characters::gained_today(c, now) == 0 {
                let t = if c.last_played == 0 { "Not played yet".to_string() } else { format!("Last played {}", ago(now, c.last_played)) };
                muted(ui, &t);
            }
        });
    });
    crate::guide::mark(ui, "profile.character", block.response.rect);
    // click: the character in the Characters tab
    if ui
        .interact(block.response.rect, ui.scope_id().with(("open-char", c.char_id)), Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text("Open in Characters")
        .clicked()
    {
        crate::window::open_later(ui.ctx(), format!("Characters:{}", c.char_id));
    }
}

/// The fame all characters gained in the last 24 hours, at the right, its chart with the dungeons played in it, and
/// which characters it is made of (Java `fameGain`).
fn fame_gain(ui: &mut Ui, p: &Palette, app: &App, icons: &mut Icons, chars: &Characters, most: &Char, now: i64) {
    let mut gainers: Vec<&Char> = chars.all().filter(|o| Characters::gained_today(o, now) > 0).collect();
    gainers.sort_by_key(|o| -Characters::gained_today(o, now));
    let gain: i64 = gainers.iter().map(|o| Characters::gained_today(o, now)).sum();
    let color = if gain > 0 { fixed::GREEN } else { fixed::MUTED };
    widgets::label_right(ui, &format!("+{} fame", num(gain)), &text(20.0), color, 24.0)
        .on_hover_text("Fame gained by all your characters in the last 24 hours");
    let hour: i64 = gainers.iter().flat_map(|o| Characters::sessions(o, now - 3_600_000)).map(|s| s.gained).sum();
    if hour > 0 {
        widgets::label_right(ui, &format!("+{} last hour", num(hour)), &text(18.0), p.gold, 24.0)
            .on_hover_text("Fame gained in the last hour");
    }
    let since = now - DAY_MS;
    let points = crate::characters::combined_gain(if gainers.is_empty() { std::slice::from_ref(&most) } else { &gainers }, since);
    if points.len() > 1 {
        ui.add_space(6.0);
        fame_graph(ui, p, app, icons, &points, now);
    }
    if !gainers.is_empty() {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, 2.0);
            for (i, o) in gainers.iter().enumerate() {
                let t = format!("{}{} +{}", if i == 0 { "" } else { "     " }, o.class_name, num(Characters::gained_today(o, now)));
                let l = label(ui, &t, &widgets::small(), if i == 0 { p.gold } else { fixed::MUTED });
                if ui
                    .interact(l.rect, ui.scope_id().with(("gainer", o.char_id)), Sense::click())
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .on_hover_text(format!("Open this {} in Characters", o.class_name))
                    .clicked()
                {
                    crate::window::open_later(ui.ctx(), format!("Characters:{}", o.char_id));
                }
            }
        });
    }
}

/// Fame over the last 24 hours as a thin step line on a base line (Java `ProfileView.FameGraph`): stretches of an
/// hour or more without a change cut out to a narrow break, and every dungeon above the line where you left it, the
/// same dungeon several times in a row as one portal with "3x" before it (click: that dungeon under Battles).
fn fame_graph(ui: &mut Ui, p: &Palette, app: &App, icons: &mut Icons, points: &[(i64, i64)], now: i64) {
    use crate::charts::{Timeline, changes, fame_at, fame_in, gap};
    // GAP: the room between a portal and the line below it; AXIS: room for the break times
    const BREAK: f32 = 26.0;
    const ICON: f32 = 24.0;
    const GAP: f32 = 7.0;
    const TOP: f32 = ICON + 16.0 + GAP + 4.0;
    const AXIS: f32 = 15.0;
    let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 64.0 + TOP + AXIS), Sense::click());
    let painter = ui.painter().clone();
    let Some(&(from, _)) = points.first() else { return };
    let tl = Timeline::new(&changes(points), from, now, BREAK);
    let (left, right) = (r.left() + 2.0, r.right() - 2.0);
    let h = r.height() - 4.0 - AXIS;
    let hi = points.iter().map(|q| q.1).max().unwrap_or(1).max(1);
    let y_of = |v: i64| r.top() + h - v.max(0) as f32 * (h - 4.0 - TOP) / hi as f32 - 2.0;
    let breaks = tl.breaks(left, right);
    // the lines leave the cut-out stretches empty: drawn piece by piece between them
    let mut pieces = Vec::new();
    let mut x0 = r.left();
    for &(_, _, cx) in &breaks {
        pieces.push((x0, cx.round() - BREAK / 2.0 + 3.0));
        x0 = cx.round() + BREAK / 2.0 - 3.0;
    }
    pieces.push((x0, r.right()));
    let gold = p.gold;
    let mut line = Vec::new();
    for (i, &(t, v)) in points.iter().enumerate() {
        let x = tl.x(t, left, right).round();
        if let Some(&last) = line.last() {
            let last: egui::Pos2 = last;
            line.push(pos2(x, last.y));
        }
        line.push(pos2(x, y_of(v)));
        if i + 1 == points.len() {
            // fame stays until now
            line.push(pos2(tl.x(now, left, right).round(), y_of(v)));
        }
    }
    for &(a, b) in &pieces {
        let clipped = painter.with_clip_rect(Rect::from_min_max(pos2(a, r.top()), pos2(b, r.bottom())));
        clipped.rect_filled(Rect::from_min_max(pos2(r.left(), r.top() + h + 2.0), pos2(r.right(), r.top() + h + 4.0)), 0.0, p.line);
        clipped.add(egui::Shape::line(line.clone(), egui::Stroke::new(2.5, gold)));
    }
    // the line's highest point (smallest y) in every pixel column, so the dungeons sit evenly above it
    let w = r.width().max(1.0) as usize;
    let mut line_top = vec![f32::INFINITY; w];
    for s in line.windows(2) {
        let (a, b) = (s[0], s[1]);
        let (c0, c1) = (((a.x.min(b.x) - r.left()).max(0.0)) as usize, ((a.x.max(b.x) - r.left()).max(0.0)) as usize);
        for c in line_top.iter_mut().take(c1.min(w - 1) + 1).skip(c0) {
            *c = c.min(a.y.min(b.y));
        }
    }
    // the cut-out stretches: two slashes on the base line and how long it was below
    let small = text(12.0);
    let mut tip: Option<String> = None;
    let hover = resp.hover_pos();
    for &(a, b, cx) in &breaks {
        let cx = cx.round();
        let base = r.top() + h;
        for dx in [-3.0, 2.0] {
            painter.line_segment([pos2(cx + dx - 2.0, base + 5.0), pos2(cx + dx + 2.0, base - 3.0)], egui::Stroke::new(1.3, fixed::MUTED));
        }
        let t = gap(b - a);
        let tw = crate::jtext::width(&painter, &t, &small);
        let tx = (cx - tw / 2.0).min(r.right() - tw).max(r.left());
        crate::jtext::draw(&painter, pos2(tx, base + 7.0), egui::Align2::LEFT_TOP, &t, &small, fixed::MUTED);
        if hover.is_some_and(|q| (q.x - cx).abs() <= BREAK / 2.0) {
            tip = Some(format!("No fame for {} (left out)", gap(b - a)));
        }
    }
    // the dungeons, the same one several times in a row as one group
    let mut visits: Vec<&lg_app::dungeon_visits::Visit> =
        app.dungeon_visits.visits.iter().chain(app.dungeon_visits.current.iter()).collect();
    visits.sort_by_key(|v| v.start);
    let end_of = |v: &lg_app::dungeon_visits::Visit| if v.duration_ms > 0 { v.start + v.duration_ms } else { now };
    let mut groups: Vec<Vec<&lg_app::dungeon_visits::Visit>> = Vec::new();
    for v in visits {
        if v.hidden || end_of(v) < tl.from || v.start > now {
            continue;
        }
        match groups.last_mut() {
            Some(g) if g[0].dungeon == v.dungeon => g.push(v),
            _ => groups.push(vec![v]),
        }
    }
    let count_f = text(16.0);
    let data = app.world.data();
    let mut last_right = -10_000.0f32;
    let mut clicked: Option<i64> = None;
    for group in &groups {
        let (first, last) = (group[0], group[group.len() - 1]);
        let Some(icon) = data.dungeons.portal_by_name.get(&first.dungeon).and_then(|id| icons.object(ui.ctx(), data, *id, ICON as u32))
        else {
            continue;
        };
        let end = end_of(last).min(tl.to);
        let gained: i64 = group.iter().filter_map(|v| fame_in(points, v.start, end_of(v))).sum();
        let count = if group.len() > 1 { format!("{}x", group.len()) } else { String::new() };
        let count_w = if count.is_empty() { 0.0 } else { crate::jtext::width(&painter, &count, &count_f) + 2.0 };
        // the portal's centre where you left, kept clear of the group before it and inside the graph
        let x = tl.x(end, left, right).round().max(last_right + 6.0 + count_w + ICON / 2.0).min(right - ICON / 2.0);
        let (c0, c1) =
            ((x - ICON / 2.0 - count_w - r.left()).max(0.0) as usize, ((x + ICON / 2.0 - r.left()).max(0.0) as usize).min(w - 1));
        let mut under = line_top[c0.min(w - 1)..=c1].iter().copied().fold(f32::INFINITY, f32::min);
        if !under.is_finite() {
            under = y_of(fame_at(points, end).unwrap_or(0));
        }
        let y = (under - ICON - GAP).max(r.top() + 18.0);
        let mut box_r = Rect::from_min_size(pos2(x - ICON / 2.0, y), vec2(ICON, ICON));
        painter.image(icon.id(), box_r, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), egui::Color32::WHITE);
        if !count.is_empty() {
            crate::jtext::draw(&painter, pos2(box_r.left() - count_w, box_r.center().y), egui::Align2::LEFT_CENTER, &count, &count_f, gold);
            box_r = box_r.union(Rect::from_min_size(pos2(box_r.left() - count_w, y), vec2(count_w, ICON)));
        }
        // the guildmates' characters (of all the group's runs), in their look then, side by side right above it
        let mut mates: Vec<(String, i32, i32, i32, i32)> = Vec::new();
        for v in group {
            let current = app.dungeon_visits.current.as_ref().is_some_and(|c| c.start == v.start);
            let run = if current { app.dps_history.live.as_ref() } else { lg_app::dungeon_visits::run_of(v, &app.dps_history) };
            let fought = app.server.mates.fought(run, &v.dungeon);
            if fought.is_empty() {
                continue;
            }
            let key = format!("{}/{}", v.dungeon, v.start);
            if !current && !app.server.mates.looks.contains_key(&key) {
                crate::window::ask_later(
                    ui.ctx(),
                    crate::window::Ask::Server(crate::window::ServerAsk::RunLooks(v.dungeon.clone(), v.start, v.duration_ms)),
                );
            }
            for m in app.server.mates.with(&key, &fought, &data.items.transform_skins) {
                if !mates.iter().any(|x| x.0.eq_ignore_ascii_case(&m.0)) {
                    mates.push(m);
                }
            }
        }
        const MATE: f32 = 16.0;
        let mut with = Vec::new();
        let mut mx = x - mates.len() as f32 * MATE / 2.0;
        for (name, class, skin, t1, t2) in &mates {
            if let Some(t) = icons.skin(ui.ctx(), data, if *skin > 0 { *skin } else { *class }, *t1, *t2, MATE as u32) {
                painter.image(
                    t.id(),
                    Rect::from_min_size(pos2(mx, y - MATE - 1.0), vec2(MATE, MATE)),
                    Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }
            mx += MATE;
            let cls = data.classes.iter().find(|k| k.id == *class).map_or(String::new(), |k| format!(" ({})", k.name));
            with.push(format!("{name}{cls}"));
        }
        if !mates.is_empty() {
            box_r = box_r.union(Rect::from_min_size(
                pos2(x - mates.len() as f32 * MATE / 2.0, y - MATE - 1.0),
                vec2(mates.len() as f32 * MATE, MATE),
            ));
        }
        if hover.is_some_and(|q| box_r.contains(q)) {
            let n = group.len();
            tip = Some(format!(
                "{}{}:  +{} fame{}  ·  {}",
                if n > 1 { format!("{n}x ") } else { String::new() },
                first.dungeon,
                num(gained),
                if with.is_empty() { String::new() } else { format!("  ·  with {}", with.join(", ")) },
                if n > 1 { "click to open the last one in Battles" } else { "click to open it in Battles" }
            ));
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            if resp.clicked() {
                clicked = Some(last.start);
            }
        }
        last_right = x + ICON / 2.0;
    }
    let resp = resp.on_hover_text(tip.unwrap_or_else(|| "Fame gained over the last 24 hours".into()));
    let _ = resp;
    if let Some(start) = clicked {
        crate::window::open_later(ui.ctx(), format!("Battles:visit:{start}"));
    }
}

/// One number with its caption and a line below (Java `tile`).
/// How many suggestion boxes the overview shows.
const SUGGESTIONS: usize = 4;

/// A suggestion box: a quill and "Coming soon" for now.
fn suggestion(ui: &mut Ui, p: &Palette) {
    let (r, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 64.0), egui::Sense::hover());
    quill(ui.painter(), egui::pos2(r.left() + 30.0, r.center().y), p.ink_soft, p.gold);
    crate::jtext::draw(
        ui.painter(),
        egui::pos2(r.left() + 66.0, r.center().y),
        egui::Align2::LEFT_CENTER,
        "Coming soon",
        &widgets::head_px(26.0),
        p.ink_soft,
    );
}

/// A quill around `c`, slanted: the feather's vane, its shaft and the nib with a drop of ink.
fn quill(painter: &egui::Painter, c: egui::Pos2, feather: egui::Color32, nib: egui::Color32) {
    let (s, co) = (-std::f32::consts::FRAC_PI_4).sin_cos();
    // points along the quill (x down the shaft) turned 45 degrees, tip at the bottom left
    let at = |x: f32, y: f32| egui::pos2(c.x + x * co - y * s, c.y + x * s + y * co);
    let vane: Vec<egui::Pos2> =
        vec![at(-24.0, 0.0), at(-14.0, -7.0), at(0.0, -8.0), at(10.0, -4.0), at(12.0, 0.0), at(10.0, 3.0), at(0.0, 5.0), at(-14.0, 4.0)];
    painter.add(egui::Shape::convex_polygon(vane, feather.gamma_multiply(0.85), egui::Stroke::new(1.0, feather)));
    painter.line_segment([at(-26.0, 0.0), at(20.0, 0.0)], egui::Stroke::new(1.6, feather));
    painter.add(egui::Shape::convex_polygon(vec![at(14.0, -2.5), at(24.0, 0.0), at(14.0, 2.5)], nib, egui::Stroke::NONE));
    painter.circle_filled(at(27.0, 0.0), 1.8, nib);
}

fn tile(ui: &mut Ui, p: &Palette, caption_text: &str, value: &str, sub: &str) {
    caption(ui, p, caption_text);
    label(ui, value, &widgets::head_px(42.0), p.ink);
    soft(ui, p, sub);
}

fn today_tiles(ui: &mut Ui, p: &Palette, app: &App, runs: &[&lg_app::dps_history::Run], drops: &[Drop], now: i64, since: i64) {
    let mut play_ms = 0;
    let mut sessions = 0;
    for c in app.characters.all() {
        for s in Characters::sessions(c, since) {
            play_ms += s.duration_ms();
            sessions += 1;
        }
    }
    let today: Vec<_> = runs.iter().filter(|r| r.start >= since && !r.guild_hall).collect();
    let kinds = today.iter().map(|r| r.dungeon.as_str()).collect::<std::collections::BTreeSet<_>>().len();
    let amulet = app.amulet_saves.relevant(&app.characters, app.world.data(), now);
    let bag = |color: &str| -> (String, String) {
        let mut bags: Vec<(String, String)> = Vec::new(); // key, where (in first-seen order)
        for d in drops {
            if d.time >= since && base_bag(&d.bag) == color {
                let key = format!("{}|{}", d.time / 1000, d.r#where);
                if !bags.iter().any(|(k, _)| *k == key) {
                    bags.push((key, d.r#where.clone()));
                }
            }
        }
        let mut by_place: Vec<(String, i64)> = Vec::new();
        for (_, w) in &bags {
            match by_place.iter_mut().find(|(n, _)| n == w) {
                Some(e) => e.1 += 1,
                None => by_place.push((w.clone(), 1)),
            }
        }
        let best = by_place.iter().max_by_key(|e| e.1).map(|e| e.0.clone());
        (num(bags.len() as i64), best.map_or("none yet".into(), |b| format!("most in {b}")))
    };
    let (white, white_sub) = bag("White");
    let (orange, orange_sub) = bag("Orange");
    let dungeons_sub = if kinds == 0 { "none yet".to_string() } else { format!("{kinds} different") };
    let play_sub =
        if sessions == 0 { "not played".to_string() } else { format!("{sessions} session{}", if sessions == 1 { "" } else { "s" }) };
    let mut tiles: Vec<widgets::Card<'_, ()>> = vec![
        Box::new(|ui: &mut Ui, _: &mut ()| tile(ui, p, "DUNGEONS", &num(today.len() as i64), &dungeons_sub)),
        Box::new(|ui: &mut Ui, _: &mut ()| tile(ui, p, "WHITE BAGS", &white, &white_sub)),
        Box::new(|ui: &mut Ui, _: &mut ()| tile(ui, p, "ORANGE BAGS", &orange, &orange_sub)),
        Box::new(|ui: &mut Ui, _: &mut ()| tile(ui, p, "PLAYTIME", &duration(play_ms), &play_sub)),
    ];
    if amulet {
        let all = app.amulet_saves.count();
        let today_n = app.amulet_saves.count_since(since);
        let sub = if today_n == 0 { "none today".to_string() } else { format!("{today_n} today") };
        tiles.push(Box::new(move |ui: &mut Ui, _: &mut ()| tile(ui, p, "AMULET SAVES", &num(all as i64), &sub)));
    }
    row(ui, p, 12.0, &mut (), tiles);
}

fn recent_drops(ui: &mut Ui, p: &Palette, app: &App, icons: &mut Icons, drops: &[Drop], now: i64) {
    heading(ui, p, "Recent drops");
    ui.add_space(6.0);
    if drops.is_empty() {
        muted(ui, "Drops of collectible items show up here.");
    }
    let data = app.world.data();
    for d in drops.iter().take(RECENT) {
        ui.add_space(6.0);
        let top = ui.cursor().min;
        let w = ui.available_width();
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            let tex = icons.object(ui.ctx(), data, d.item, 40);
            let (r, _) = ui.allocate_exact_size(vec2(44.0, 44.0), Sense::hover());
            if let Some(t) = tex {
                ui.painter().image(
                    t.id(),
                    egui::Rect::from_center_size(r.center(), vec2(40.0, 40.0)),
                    egui::Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
            ui.add_space(12.0);
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                let name = data.items.item(d.item).map_or(format!("Item {}", d.item), |i| i.name.clone());
                label(ui, &name, &text(18.0), p.ink);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                    if let Some(b) = bag_id(&d.bag) {
                        let tex = icons.object(ui.ctx(), data, b, 18);
                        widgets::icon(ui, tex.as_ref(), 18.0);
                        ui.add_space(4.0);
                    }
                    let bag = if d.bag.is_empty() { "Drop".to_string() } else { format!("{} bag", d.bag) };
                    let place = if d.r#where.is_empty() { String::new() } else { format!("  ·  {}", d.r#where) };
                    muted(ui, &format!("{bag}{place}"));
                });
            });
        });
        // how long ago, at the right, vertically centred on the line
        let h = 44.0;
        let font = text(18.0);
        let t = ago(now, d.time);
        crate::jtext::draw(ui.painter(), pos2(top.x + w, top.y + h / 2.0), egui::Align2::RIGHT_CENTER, &t, &font, p.ink_soft);
        // click: the item in Inventory
        let rr = egui::Rect::from_min_size(top, vec2(w, h));
        if ui
            .interact(rr, ui.scope_id().with(("open-item", d.item, d.time)), Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
        {
            crate::window::open_later(ui.ctx(), format!("Inventory:item:{}", d.item));
        }
        ui.add_space(6.0);
    }
}

fn recent_runs(ui: &mut Ui, p: &Palette, app: &App, icons: &mut Icons, runs: &[&lg_app::dps_history::Run], drops: &[Drop]) {
    heading(ui, p, "Recent runs");
    ui.add_space(6.0);
    if runs.is_empty() {
        muted(ui, "Dungeons you clear show up here.");
        return;
    }
    let data = app.world.data();
    let painter = ui.painter().clone();
    let f = text(18.0);
    let small = widgets::small();
    // the cells' texts first, to size the columns (GridBag: the dungeon column takes what is left)
    struct Cell {
        text: String,
        color: Color32,
    }
    let shown: Vec<&&lg_app::dps_history::Run> = runs.iter().take(RECENT).collect();
    let mut cells: Vec<[Cell; 3]> = Vec::new();
    for r in &shown {
        let peak = r
            .bosses
            .iter()
            .filter_map(|b| b.players.iter().find(|x| x.me).map(|me| f64::from(me.damage) / (b.fight_ms as f64 / 1000.0).max(1.0)))
            .fold(0.0, f64::max);
        let rank = if r.guild_hall {
            Cell { text: format!("{} loadout{}", r.bosses.len(), if r.bosses.len() == 1 { "" } else { "s" }), color: p.ink_soft }
        } else {
            let boss = r.main_boss();
            match boss.and_then(|b| b.players.iter().position(|x| x.me).map(|i| (i + 1, b.players.len()))) {
                None => Cell { text: "-".into(), color: fixed::MUTED },
                Some((rank, of)) => Cell {
                    text: format!("#{rank} of {of}"),
                    color: if rank == 1 {
                        fixed::GREEN
                    } else if rank <= 3 {
                        p.gold
                    } else {
                        p.ink_soft
                    },
                },
            }
        };
        cells.push([Cell { text: clock(r.duration_ms), color: p.ink_soft }, Cell { text: short_dps(peak), color: p.ink }, rank]);
    }
    let heads = ["TIME", "BOSS DPS", "RANK"];
    let mut widths = [0.0f32; 3];
    for (i, h) in heads.iter().enumerate() {
        widths[i] = crate::jtext::width(&painter, h, &small);
        for row in &cells {
            widths[i] = widths[i].max(crate::jtext::width(&painter, &row[i].text, &f));
        }
    }
    // the loot column: a bag icon per bag that dropped during the run
    let loot: Vec<Vec<i32>> = shown
        .iter()
        .map(|r| {
            let end = r.start + r.duration_ms + 5_000;
            let mut seen = Vec::new();
            let mut out = Vec::new();
            for d in drops {
                if d.time < r.start || d.time > end {
                    continue;
                }
                let key = format!("{}|{}", d.time / 1000, d.bag);
                if !seen.contains(&key) {
                    seen.push(key);
                    if let Some(b) = bag_id(&d.bag) {
                        out.push(b);
                    }
                }
            }
            out
        })
        .collect();
    let loot_w = loot
        .iter()
        .map(|l| if l.is_empty() { crate::jtext::width(&painter, "-", &f) } else { l.len() as f32 * 24.0 })
        .fold(crate::jtext::width(&painter, "LOOT", &small), f32::max);
    let total = ui.available_width();
    let fixed_w: f32 = widths.iter().map(|w| w + 14.0).sum::<f32>() + loot_w + 14.0;
    let first_w = (total - fixed_w).max(100.0);
    let xs = [0.0, first_w + 14.0, first_w + 14.0 + widths[0] + 14.0, first_w + 14.0 + widths[0] + 14.0 + widths[1] + 14.0];
    let loot_x = xs[3] + widths[2] + 14.0;
    let left = ui.cursor().min.x;
    // header
    let (r, _) = ui.allocate_exact_size(vec2(total, crate::jtext::height(&painter, &small) + 8.0), Sense::hover());
    let y = r.top() + 4.0;
    crate::jtext::draw(&painter, pos2(left, y), egui::Align2::LEFT_TOP, "DUNGEON", &small, p.ink_muted);
    for (i, h) in heads.iter().enumerate() {
        crate::jtext::draw(&painter, pos2(left + xs[i + 1], y), egui::Align2::LEFT_TOP, h, &small, p.ink_muted);
    }
    crate::jtext::draw(&painter, pos2(left + loot_x, y), egui::Align2::LEFT_TOP, "LOOT", &small, p.ink_muted);
    for (k, r) in shown.iter().enumerate() {
        let h = 34.0f32.max(crate::jtext::height(&painter, &f));
        let (row_r, row) = ui.allocate_exact_size(vec2(total, h + 8.0), Sense::click());
        // click: the run in Battles
        if row.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text("Open this run in Battles").clicked() {
            crate::window::open_later(ui.ctx(), format!("Battles:run:{}", r.start));
        }
        let cy = row_r.center().y;
        // the dungeon's portal, then its name
        let icon_id = if r.guild_hall { 480 } else { data.dungeons.portal_by_name.get(&r.dungeon).copied().unwrap_or(-1) };
        if let Some(t) = icons.object(ui.ctx(), data, icon_id, 34) {
            painter.image(
                t.id(),
                egui::Rect::from_min_size(pos2(left, cy - 17.0), vec2(34.0, 34.0)),
                egui::Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        }
        crate::jtext::draw(&painter, pos2(left + 42.0, cy), egui::Align2::LEFT_CENTER, &r.dungeon, &f, p.ink);
        for (i, c) in cells[k].iter().enumerate() {
            crate::jtext::draw(&painter, pos2(left + xs[i + 1], cy), egui::Align2::LEFT_CENTER, &c.text, &f, c.color);
        }
        if loot[k].is_empty() {
            crate::jtext::draw(&painter, pos2(left + loot_x, cy), egui::Align2::LEFT_CENTER, "-", &f, fixed::MUTED);
        } else {
            for (j, b) in loot[k].iter().enumerate() {
                if let Some(t) = icons.object(ui.ctx(), data, *b, 22) {
                    painter.image(
                        t.id(),
                        egui::Rect::from_min_size(pos2(left + loot_x + j as f32 * 24.0, cy - 11.0), vec2(22.0, 22.0)),
                        egui::Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                        Color32::WHITE,
                    );
                }
            }
        }
    }
}

/// The dungeons you entered most (Java `DungeonStatsPanel.topDungeons`), each with a bar against the first.
fn top_dungeons(ui: &mut Ui, p: &Palette, app: &App) {
    heading(ui, p, "Top dungeons");
    ui.add_space(6.0);
    let mut rows: Vec<(&str, i32)> =
        app.world.dungeon_stats.data.iter().filter(|(_, i)| i.entered > 0).map(|(n, i)| (n.as_str(), i.entered)).collect();
    rows.sort_by_key(|r| std::cmp::Reverse(r.1));
    rows.truncate(RECENT);
    if rows.is_empty() {
        muted(ui, "Run some dungeons to see them here.");
    }
    let max = rows.first().map_or(1, |r| r.1.max(1));
    let f = text(18.0);
    for (name, n) in rows {
        let w = ui.available_width();
        let (r, _) = ui.allocate_exact_size(vec2(w, crate::jtext::height(ui.painter(), &f)), Sense::hover());
        crate::jtext::draw(ui.painter(), r.min, egui::Align2::LEFT_TOP, name, &f, p.ink);
        crate::jtext::draw(ui.painter(), pos2(r.right(), r.top()), egui::Align2::RIGHT_TOP, &num(i64::from(n)), &f, p.ink_soft);
        ui.add_space(2.0);
        let (bar, _) = ui.allocate_exact_size(vec2(w, 10.0), Sense::hover());
        ui.painter().rect_filled(bar, 0.0, p.header);
        let share = f64::from(n) / f64::from(max);
        ui.painter().rect_filled(egui::Rect::from_min_size(bar.min, vec2((f64::from(w) * share).round() as f32, 10.0)), 0.0, p.gold);
        ui.add_space(8.0);
    }
}

/// Classes fully exalted, a cell per class, the class closest to it (Java `exaltation`).
fn exaltation(ui: &mut Ui, p: &Palette, app: &App) {
    heading(ui, p, "Exaltation");
    ui.add_space(6.0);
    if app.exalts.progress.is_empty() {
        muted(ui, "Visit the nexus once so your exaltations are loaded.");
        return;
    }
    let max = 8 * lg_app::exalt_store::THRESHOLDS.len() as i32;
    let classes = &app.world.data().classes;
    let totals: Vec<i32> = classes.iter().map(|c| lg_app::exalt_store::total(app.exalts.progress(c.id).map(|v| v.as_slice()))).collect();
    let full = totals.iter().filter(|t| **t == max).count();
    let mut closest: Option<usize> = None;
    for (i, t) in totals.iter().enumerate() {
        if *t != max && closest.is_none_or(|c| *t > totals[c]) {
            closest = Some(i);
        }
    }
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
        ui.add_space(8.0);
        let big = widgets::head_px(42.0);
        let r = label(ui, &full.to_string(), &big, p.ink).rect;
        ui.add_space(8.0);
        // FlowLayout centres the smaller text on the line
        let small = text(24.0);
        let t = format!("/ {} classes maxed", classes.len());
        let h = crate::jtext::height(ui.painter(), &small);
        let (s, _) = ui.allocate_exact_size(vec2(crate::jtext::width(ui.painter(), &t, &small), r.height()), Sense::hover());
        crate::jtext::draw(ui.painter(), pos2(s.left(), s.center().y - h / 2.0), egui::Align2::LEFT_TOP, &t, &small, p.ink_soft);
    });
    ui.add_space(6.0);
    let w = ui.available_width();
    let (strip, _) = ui.allocate_exact_size(vec2(w, 18.0), Sense::hover());
    let n = totals.len() as f32;
    if n > 0.0 {
        let cell = (w - 3.0 * (n - 1.0)) / n;
        for (i, t) in totals.iter().enumerate() {
            let c = if *t == max {
                p.gold
            } else if *t > 0 {
                p.gold_dark
            } else {
                p.header
            };
            let x = (i as f32 * (cell + 3.0)).round();
            ui.painter().rect_filled(egui::Rect::from_min_size(pos2(strip.left() + x, strip.top()), vec2(cell.round(), 18.0)), 0.0, c);
        }
    }
    ui.add_space(10.0);
    match closest {
        Some(i) => {
            let left = max - totals[i];
            let name = classes[i].name.clone();
            soft(ui, p, &format!("Closest next: {name}, {left} exaltation{} left", if left == 1 { "" } else { "s" }));
        }
        None => {
            soft(ui, p, "Every class is fully exalted!");
        }
    }
}
