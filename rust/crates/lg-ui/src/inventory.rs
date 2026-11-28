//! The Inventory tab (Java: `CollectionPanel`, `CollectionGrid`): pages Items, ST Sets, Log; on Items every
//! collectible item in a grid (yours bright with how many copies and the best rarity, the rest faded), the progress,
//! actions, and what you found lately. A column on the left picks whose finds are shown: all, or a game mode run's.

use crate::detail::{Rarity, enchant_badge, uv};
use crate::jtext;
use crate::profile::{Ctx, num};
use crate::theme::{Palette, fixed, mix};
use crate::widgets::{self, small, text};
use egui::{Align2, Color32, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};
use std::collections::HashMap;

const CELL: f32 = 34.0;
const ICON: f32 = 28.0;
const PAD: f32 = 6.0;
const BG: Color32 = Color32::from_rgb(5, 5, 5);
const COUNT_GOLD: Color32 = Color32::from_rgb(0xF5, 0xC5, 0x42);
const FILTERS: [&str; 4] = ["All", "Shiny", "Normal", "Reusable"];
const WHITE_BAG: i32 = 1292;
const RECENT: usize = 8;
const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

#[derive(Default)]
pub struct InventoryView {
    /// "Copy as image" under way: the grid drawn over the window a piece at a time, each piece grabbed.
    grab: Option<Grab>,
    /// "Copy as image" asked from outside the view (it starts on the next draw).
    copy_requested: bool,
    pub page: usize,
    pub filter: usize,
    pub search: String,
    /// The game mode character whose finds are shown (0: everything).
    pub upe_char: i32,
    /// The focus as last seen: when it changes, its run's finds are shown (and the whole collection again after it).
    focus_seen: Option<i32>,
    pub st_filter: usize,
    pub st_class: Option<String>,
    /// The ST set clicked on its card: the side shows what forging its missing pieces takes (its name).
    pub st_selected: Option<String>,
    /// The piece of the set clicked whose copies its bar shows alone (None: all four).
    pub st_piece: Option<i32>,
    /// An item in a set's bar clicked that is not of the set: it, the column it was in, its copy; shown in the next
    /// column (how far, fading in and out); the last one, for fading out.
    pub st_show: Option<(i32, usize, lg_app::loadout::Held)>,
    /// Crafts planned (while the app runs): what they take is no longer offered to other crafts.
    st_planned: Vec<Planned>,
    st_show_t: f32,
    /// How far the item shown was dragged from its place.
    st_show_off: egui::Vec2,
    st_show_last: Option<(i32, usize, lg_app::loadout::Held)>,
    /// How far each set's bar is open (the one clicked opening, the one before closing), by the set's name.
    st_bar_t: HashMap<String, f32>,
    /// Items and ST Sets melted into one page: the collection left, the sets right.
    pub melt: bool,
    /// Where the melt goes when it ends: 0 back to Items, 1 on to the sets (the items slide out left).
    pub melt_exit: usize,
    /// The Sets page (and the melt) shows the themed sets, else the ST sets: the toggle in its head flips them.
    pub sets_themed: bool,
    /// How far melted (0 apart, 1 one), moving towards `melt`.
    melt_t: f32,
    /// Parting from the melt: the moving edge the page it goes to is shown up to (its layout as it will stand).
    reveal: Option<f32>,
    /// How many of the collection's icons were made ahead (a few each frame), so the grid comes in whole.
    warmed: usize,
    /// The inventory optimization on: your spare copies framed in the collection, listed in the column on the left.
    pub opt: bool,
    /// Its list's order: 0 by kind (ST, UT) and how many spare, 1 by feed power, 2 by what dismantling gives.
    pub opt_sort: usize,
    /// The set whose pieces beat in the collection, and since when.
    beat: Option<(String, f64)>,
    /// A tour asks for a set to show: the sets page clicks the first it shows whose missing piece is craftable (else
    /// one with something to forge).
    pub tour_pick: bool,
    /// The key clicked on the Keys page: the side shows what it gives (a mystery key) or its dungeon.
    pub key_selected: Option<i32>,
    /// Whether it was clicked among the seasonal keys (its bar opens there only).
    pub key_seasonal: bool,
    /// The item whose history replaces the Recent list (Java `historyItem`).
    pub history: Option<i32>,
    /// An item opened from another tab, to be scrolled to and pulsed once the grid is drawn (Java `showItem`).
    pub show_item: Option<i32>,
    /// The gold box growing and fading around an item: which, since when (s), how long one opening takes (s), and
    /// whether it glows up three times (an item opened from another tab); and to scroll to it.
    pulse: Option<(i32, f64, f64, bool)>,
    scroll_to: Option<i32>,
}

/// The column of whose finds: its width.
const UPE_COLUMN_W: f32 = 4.0 + 96.0 + 4.0 + 6.0;

const PULSE_S: f64 = 2.0;
const CLICK_PULSE_S: f64 = 1.0;
const PULSE_GROW: f32 = 14.0;

/// An item opened from another tab glows up three times, each slower: a quarter, half, then the whole of one
/// opening's time (the last as a click's).
const GLOW_UPS: [f64; 3] = [0.25, 0.5, 1.0];

/// How far each glow-up starts before the one before it ends, as a share of that one's time: the second barely
/// overlaps the first, the third clearly overlaps the second.
const GLOW_OVERLAP: [f64; 3] = [0.0, 0.15, 0.45];

/// When each glow-up starts and how long it takes (s).
fn glow_ups(dur: f64) -> [(f64, f64); 3] {
    let mut out = [(0.0, 0.0); 3];
    let mut end: f64 = 0.0;
    let mut prev = 0.0;
    for (i, f) in GLOW_UPS.iter().enumerate() {
        let d = dur * f;
        let start = (end - prev * GLOW_OVERLAP[i]).max(0.0);
        out[i] = (start, d);
        end = start + d;
        prev = d;
    }
    out
}

/// How wide the optimization's column is.
const OPT_COLUMN_W: f32 = 380.0;

/// An item held more often than kept (the forge mode: the best copy, with a backup also the second best): its copies,
/// the most enchanted first, how many of them are kept, and which rarities count as spare (by enchantments, 0..4).
struct Spare {
    item: i32,
    copies: Vec<lg_app::loadout::Held>,
    keep: usize,
    counts: [bool; 5],
}

fn enchant_count(h: &lg_app::loadout::Held) -> usize {
    h.enchant_ids.as_ref().map_or(0, |e| e.iter().filter(|x| **x >= 0).count())
}

impl Spare {
    /// Whether the copy at `rank` is spare: not kept, of a rarity that counts.
    fn spare(&self, rank: usize) -> bool {
        rank >= self.keep && self.copies.get(rank).is_some_and(|h| self.counts[enchant_count(h).min(4)])
    }

    fn free(&self) -> usize {
        (0..self.copies.len()).filter(|r| self.spare(*r)).count()
    }
}

/// The forge mode and the items whose backup was let go, as the settings say.
fn forge_mode(props: &lg_app::properties::Properties) -> (lg_app::st_forge::Mode, std::collections::HashSet<i32>) {
    use lg_app::st_forge::{FREED_KEY, Mode};
    let freed = props.get(FREED_KEY).map(|v| v.split(',').filter_map(|x| x.trim().parse().ok()).collect()).unwrap_or_default();
    (Mode::of(props.get(Mode::KEY)), freed)
}

/// Every collectible item you hold more copies of than you keep, on your living characters and in your vaults, in the
/// order `sort` asks: 0 STs, then UTs, then the rest, each the most spare copies first; 1 the most feed power first;
/// 2 the most forge essence from dismantling first. Spare are the copies of the rarities considered for crafting (the
/// rarity button); for the forge (2) those left out instead, as they are not kept for crafting.
fn spares(app: &lg_app::App, props: &lg_app::properties::Properties, sort: usize) -> Vec<Spare> {
    let data = app.world.data();
    let (mode, freed) = forge_mode(props);
    let allowed = lg_app::st_forge::rarities(props.get(lg_app::st_forge::RARITIES_KEY));
    let living: HashMap<i32, bool> = app.characters.living().iter().map(|l| (l.char_id, l.seasonal)).collect();
    let copies = app.holdings.copies_of(&living, |id| data.items.collectible.contains(&id));
    let enchants = enchant_count;
    let counts = if sort == 2 { allowed.map(|a| !a) } else { allowed };
    let mut out: Vec<Spare> = copies
        .into_iter()
        .filter_map(|(item, mut copies)| {
            let keep = if mode == lg_app::st_forge::Mode::Backup && !freed.contains(&item) { 2 } else { 1 };
            if copies.len() <= keep {
                return None;
            }
            copies.sort_by_key(|h| std::cmp::Reverse(enchants(h)));
            // only copies of the rarities that count are spare
            if !copies[keep..].iter().any(|h| counts[enchants(h).min(4)]) {
                return None;
            }
            Some(Spare { item, copies, keep, counts })
        })
        .collect();
    let kind = |id: i32| {
        data.items.item(id).map_or(2, |i| {
            if i.is_st() {
                0
            } else if i.is_ut() {
                1
            } else {
                2
            }
        })
    };
    let feed = |id: i32| data.items.item(id).map_or(0, |i| i.feed_power.max(0));
    let name = |id: i32| data.items.item(id).map_or(String::new(), |i| i.name.clone());
    match sort {
        1 => out.sort_by(|a, b| feed(b.item).cmp(&feed(a.item)).then(b.free().cmp(&a.free())).then(name(a.item).cmp(&name(b.item)))),
        2 => out.sort_by(|a, b| {
            essence(data, b.item).cmp(&essence(data, a.item)).then(b.free().cmp(&a.free())).then(name(a.item).cmp(&name(b.item)))
        }),
        _ => out.sort_by(|a, b| kind(a.item).cmp(&kind(b.item)).then(b.free().cmp(&a.free())).then(name(a.item).cmp(&name(b.item)))),
    }
    out
}

/// What dismantling one copy gives, as one number: common essence once, rare twice, legendary four, mythical eight
/// times.
fn essence(data: &lg_gamedata::GameData, item: i32) -> i32 {
    data.dismantle.get(&item).map_or(0, |d| d[0] + 2 * d[1] + 4 * d[2] + 8 * d[3])
}

impl InventoryView {
    /// The Optimization section at the top of `at` (the right column, or above the ST sets when crafting): what to
    /// do with the spare copies, and Feed / Craft / Forge (Feed and Forge order the list, Craft melts Items and ST
    /// Sets; again: back). Gives its rect.
    #[allow(clippy::too_many_arguments)]
    fn opt_section(
        ui: &mut Ui,
        p: &Palette,
        at: Rect,
        opt_sort: &mut usize,
        melt: &mut bool,
        melt_exit: &mut usize,
        props: &lg_app::properties::Properties,
        asks: &mut Vec<crate::window::Ask>,
        rarity: &Rarity,
        opt: &mut bool,
    ) -> Rect {
        let painter = ui.painter().clone();
        let head = widgets::section_head_height(&painter);
        let lh16 = jtext::height(&painter, &text(16.0));
        let or = Rect::from_min_size(at.min, vec2(at.width(), head + 2.0 + 10.0 + 4.0 * (lh16 + 2.0) + 10.0 + 34.0 + 10.0));
        crate::guide::mark(ui, "inventory.opt", or);
        let sort = *opt_sort;
        let crafting = *melt;
        let mut choice: Option<usize> = None;
        // the forge mode, one setting for the whole app: the same switch as on the ST Sets page
        let (mode, _) = forge_mode(props);
        let allowed = lg_app::st_forge::rarities(props.get(lg_app::st_forge::RARITIES_KEY));
        let mut next_mode = None;
        let mut next_rarities = None;
        // the whole box opens or closes the spare copies (its buttons and switch keep their own clicks)
        let on = *opt;
        let box_resp = ui
            .interact(or, ui.scope_id().with("opt-box"), Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .on_hover_text(if on { "Hide your spare copies" } else { "Show your spare copies" });
        let lit = box_resp.hovered();
        widgets::section(
            ui,
            p,
            or,
            "Optimization",
            fixed::GREEN,
            |ui, head| {
                // the icon in place of the section's square, the title after it: gold while on
                let painter = ui.painter();
                let font = crate::theme::head(17.0);
                let strip = Rect::from_min_max(pos2(head.left(), head.top() - 1.0), pos2(head.right() + 3.0, head.bottom() + 1.0));
                painter.rect_filled(strip, 0.0, if on { mix(p.gold, p.header, 0.18) } else { p.header });
                let ink = if on {
                    p.gold
                } else if lit {
                    p.ink
                } else {
                    p.ink_soft
                };
                widgets::optimize_icon(painter, pos2(head.left() + 18.0, head.center().y), 22.0, ink);
                jtext::draw(painter, pos2(head.left() + 36.0, head.center().y), Align2::LEFT_CENTER, "OPTIMIZATION", &font, ink);
                let out = forge_switch(
                    ui,
                    p,
                    rarity,
                    pos2(head.right() - 6.0, head.top() + 1.0),
                    head.height() - 2.0,
                    mode,
                    allowed,
                    f32::INFINITY,
                );
                next_mode = out.mode;
                next_rarities = out.rarities;
            },
            |ui, r| {
                let painter = ui.painter().clone();
                let (x, w) = (r.left() + 10.0, r.width() - 20.0);
                let mut y = r.top() + 10.0;
                for l in [
                    "Your spare copies are framed and listed on the left.",
                    "Feed: the most feed power first.",
                    "Craft: the ST sets beside your items.",
                    "Forge: the rarities not for crafting, the worst first.",
                ] {
                    jtext::draw(
                        &painter,
                        pos2(x, y),
                        Align2::LEFT_TOP,
                        &widgets::fit(&painter, l, &text(16.0), w),
                        &text(16.0),
                        p.ink_soft,
                    );
                    y += lh16 + 2.0;
                }
                y += 10.0;
                let names = ["Feed", "Craft", "Forge"];
                let on = if crafting {
                    1
                } else {
                    match sort {
                        1 => 0,
                        2 => 2,
                        _ => usize::MAX,
                    }
                };
                let segs = widgets::segment_rects(ui.painter(), pos2(x, y), 34.0, &names, false);
                crate::guide::mark(ui, "inventory.opt.feed", segs[0]);
                crate::guide::mark(ui, "inventory.opt.craft", segs[1]);
                if let Some(i) = widgets::segmented(ui, p, pos2(x, y), 34.0, &names, on, false, &[]) {
                    choice = Some(i);
                }
            },
        );
        match choice {
            // a sort: the spare copies shown too
            Some(0) => {
                *opt_sort = if sort == 1 { 0 } else { 1 };
                *opt = true;
            }
            Some(2) => {
                *opt_sort = if sort == 2 { 0 } else { 2 };
                *opt = true;
            }
            // craft: the ST sets come in beside the items (again: they go)
            Some(_) => {
                *melt = !*melt;
                *melt_exit = 0;
            }
            None => {}
        }
        if let Some(m) = next_mode {
            asks.push(crate::window::Ask::Set(lg_app::st_forge::Mode::KEY.into(), m.value().into()));
        }
        if let Some(r) = next_rarities {
            asks.push(crate::window::Ask::Set(lg_app::st_forge::RARITIES_KEY.into(), lg_app::st_forge::rarities_value(r)));
        }
        // clickable: a gold edge under the mouse; on: a thicker gold frame
        if on {
            ui.painter().rect_stroke(or, 0.0, Stroke::new(2.0, p.gold), StrokeKind::Inside);
        } else if lit {
            ui.painter().rect_stroke(or, 0.0, Stroke::new(1.0, p.gold_hover), StrokeKind::Inside);
        }
        if box_resp.clicked() {
            *opt = !*opt;
        }
        or
    }

    /// The optimization's column: each item you hold spare copies of, where each copy is (the seasonal side left,
    /// the regular right; the copy kept with a gold lock, a backup with a green one, the spare ones in gold boxes),
    /// how many are spare and its feed power or forge essence.
    fn opt_column(&mut self, ui: &mut Ui, c: &mut Ctx, rarity: &Rarity, r: Rect) {
        let p = *c.p;
        let app = c.app;
        let data = app.world.data();
        let list = spares(app, c.props, self.opt_sort);
        let (mode, _) = forge_mode(c.props);
        let icons = &mut *c.icons;
        let close = std::cell::Cell::new(false);
        let title = match self.opt_sort {
            1 => "Spare Copies · By Feed Power",
            2 => "Spare Copies · By Forge Essence",
            _ => "Spare Copies",
        };
        widgets::section(
            ui,
            &p,
            r,
            title,
            fixed::GREEN,
            |ui, head| {
                // close: the optimization off
                let painter = ui.painter().clone();
                let xr = Rect::from_center_size(pos2(head.right() - 14.0, head.center().y), vec2(22.0, 22.0));
                let resp = ui
                    .interact(xr, ui.scope_id().with("opt-close"), Sense::click())
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .on_hover_text("Close the optimization");
                let ink = if resp.hovered() { p.gold } else { p.ink_muted };
                for (a, b) in [(vec2(-5.0, -5.0), vec2(5.0, 5.0)), (vec2(-5.0, 5.0), vec2(5.0, -5.0))] {
                    painter.line_segment([xr.center() + a, xr.center() + b], Stroke::new(2.0, ink));
                }
                if resp.clicked() {
                    close.set(true);
                }
            },
            |ui, r| {
                egui::ScrollArea::vertical().id_salt("opt-list").show(ui, |ui| {
                    let top = ui.cursor().min;
                    let painter = ui.painter().clone();
                    let w = r.width() - 12.0 - 16.0;
                    let x = top.x + 8.0;
                    let mut y = top.y + 8.0;
                    let (f16, fs) = (text(16.0), text(13.0));
                    if !app.holdings.known() {
                        jtext::draw(&painter, pos2(x, y), Align2::LEFT_TOP, "Log in once so your items are known.", &f16, fixed::MUTED);
                    } else if list.is_empty() {
                        jtext::draw(
                            &painter,
                            pos2(x, y),
                            Align2::LEFT_TOP,
                            "No spare copies: nothing to feed or dismantle.",
                            &f16,
                            fixed::MUTED,
                        );
                    }
                    const CHIP: f32 = 26.0;
                    for (k, sp) in list.iter().enumerate() {
                        if k > 0 {
                            painter.rect_filled(Rect::from_min_size(pos2(x, y - 5.0), vec2(w, 1.0)), 0.0, p.line);
                        }
                        // the item, how many are spare, its feed power or what dismantling it gives
                        if let Some(t) = icons.object(ui.ctx(), data, sp.item, 30) {
                            painter.image(t.id(), Rect::from_min_size(pos2(x, y), vec2(30.0, 30.0)), uv(), Color32::WHITE);
                        }
                        let it = data.items.item(sp.item);
                        let name = it.map_or(format!("Item {}", sp.item), |i| i.name.clone());
                        let tag = it.map_or(String::new(), |i| i.tier_tag());
                        let value = match self.opt_sort {
                            2 => format!("{} essence", essence(data, sp.item)),
                            _ => format!("{} FP", it.map_or(0, |i| i.feed_power.max(0))),
                        };
                        let vw = jtext::width(&painter, &value, &fs);
                        let nt = widgets::fit(&painter, &format!("{name} {tag}"), &f16, w - 40.0 - vw - 50.0);
                        jtext::draw(&painter, pos2(x + 38.0, y + 8.0), Align2::LEFT_CENTER, &nt, &f16, p.ink);
                        jtext::draw(&painter, pos2(x + w, y + 8.0), Align2::RIGHT_CENTER, &value, &fs, p.gold);
                        let free = format!("{} spare of {}", sp.free(), sp.copies.len());
                        jtext::draw(&painter, pos2(x + 38.0, y + 24.0), Align2::LEFT_CENTER, &free, &fs, fixed::MUTED);
                        y += 34.0;
                        // its copies: seasonal left, regular right
                        let half = w / 2.0;
                        let mut rows_h: f32 = 0.0;
                        for (side, seasonal) in [(0.0, true), (1.0, false)] {
                            let sx = x + side * half;
                            let per = (((half - 4.0) / (CHIP + 3.0)).floor() as usize).max(1);
                            let mut list: Vec<(usize, &lg_app::loadout::Held)> =
                                sp.copies.iter().enumerate().filter(|(_, h)| (h.seasonal == Some(true)) == seasonal).collect();
                            // for the forge the worst first: what goes first
                            if self.opt_sort == 2 {
                                list.reverse();
                            }
                            if list.is_empty() {
                                jtext::draw(
                                    &painter,
                                    pos2(sx, y + CHIP / 2.0),
                                    Align2::LEFT_CENTER,
                                    if seasonal { "seasonal: none" } else { "regular: none" },
                                    &fs,
                                    fixed::MUTED,
                                );
                                rows_h = rows_h.max(CHIP);
                                continue;
                            }
                            for (n, (rank, h)) in list.iter().enumerate() {
                                let cr = Rect::from_min_size(
                                    pos2(sx + (n % per) as f32 * (CHIP + 3.0), y + (n / per) as f32 * (CHIP + 3.0)),
                                    vec2(CHIP, CHIP),
                                );
                                copy_chip(ui, &painter, &p, icons, app, rarity, h, sp.item, cr, ("opt-copy", sp.item, *rank));
                                if mode != lg_app::st_forge::Mode::Off || *rank == 0 {
                                    if *rank == 0 {
                                        lock(&painter, cr.left_top() + vec2(5.0, 6.0), 10.0, p.gold, false);
                                    } else if *rank < sp.keep {
                                        lock(&painter, cr.left_top() + vec2(5.0, 6.0), 10.0, fixed::GREEN, false);
                                    } else if sp.spare(*rank) {
                                        painter.rect_stroke(cr.expand(1.0), 0.0, Stroke::new(2.0, p.gold), StrokeKind::Outside);
                                    }
                                }
                            }
                            rows_h = rows_h.max(list.len().div_ceil(per) as f32 * (CHIP + 3.0));
                        }
                        // a click on the item: it in the grid, pulsing
                        let row = Rect::from_min_max(pos2(x, y - 34.0), pos2(x + w, y));
                        if ui
                            .interact(row, ui.scope_id().with(("opt-row", sp.item)), Sense::click())
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .on_hover_text("Show it in the collection")
                            .clicked()
                        {
                            self.show_item = Some(sp.item);
                        }
                        y += rows_h + 12.0;
                    }
                    ui.allocate_exact_size(vec2(r.width() - 12.0, y - top.y), Sense::hover());
                });
            },
        );
        if close.get() {
            self.opt = false;
        }
    }
}

/// A set's pieces beating in the melted collection, `elapsed` s after the set was clicked: the second and third of an
/// opened item's glow-ups, then 1 s of rest from when the third has mostly faded, over and over. Each box's place as
/// in `pulse_ts`.
fn heartbeat(elapsed: f64) -> Vec<f32> {
    let g = glow_ups(PULSE_S);
    let from = g[1].0;
    // the rest counted from when the third box has mostly faded (its faint tail may run into the next beat)
    let cycle = g[2].0 + 0.6 * g[2].1 - from + 1.0;
    let k = (elapsed / cycle).floor();
    let mut out = Vec::new();
    for back in [0.0, 1.0] {
        let e = from + elapsed - (k - back) * cycle;
        if k - back < 0.0 {
            continue;
        }
        out.extend(g[1..].iter().filter(|(s, d)| e >= *s && e < s + d).map(|(s, d)| ((e - s) / d).clamp(0.0, 1.0) as f32));
    }
    out
}

/// Items and ST Sets melting into one tab (`t` 0 apart, 1 one; parting plays it backwards), drawn over the tabs: gold
/// pours from Items over the gap into ST Sets, which fills from its left, then the gap closes into the one box with
/// "ITEMS AND ST SETS".
fn melt_tabs(ui: &Ui, p: &Palette, at: egui::Pos2, h: f32, t: f32, from_sets: bool) {
    let painter = ui.painter().clone();
    let font = small();
    let w0 = jtext::width(&painter, "ITEMS", &font) + 30.0;
    let w1 = jtext::width(&painter, "SETS", &font) + 30.0;
    let r0 = Rect::from_min_size(at, vec2(w0, h));
    let r1 = Rect::from_min_size(pos2(at.x + w0 + 8.0, at.y), vec2(w1, h));
    let gold = p.gold;
    let ease = |v: f32| {
        let v = v.clamp(0.0, 1.0);
        v * v * (3.0 - 2.0 * v)
    };
    // the two tabs drawn under it covered
    painter.rect_filled(r0.union(r1).expand(1.0), 0.0, p.sand);
    let box_ = |r: Rect, fill: Color32, label: &str, ink: Color32| {
        painter.rect(r, 0.0, fill, Stroke::new(1.5, Color32::BLACK), StrokeKind::Inside);
        jtext::draw(&painter, r.center(), Align2::CENTER_CENTER, label, &font, ink);
    };
    if t >= 0.9 {
        let one = r0.union(r1);
        painter.rect(one, 0.0, gold, Stroke::new(1.5, Color32::BLACK), StrokeKind::Inside);
        jtext::draw(
            &painter,
            one.center(),
            Align2::CENTER_CENTER,
            "ITEMS AND SETS",
            &font,
            p.on_accent.gamma_multiply(ease((t - 0.9) / 0.1)),
        );
        return;
    }
    // from Items: Items gold, pouring right into Sets; from (or to) Sets: the other way
    let (src, dst, src_l, dst_l) = if from_sets { (r1, r0, "SETS", "ITEMS") } else { (r0, r1, "ITEMS", "SETS") };
    box_(src, gold, src_l, p.on_accent);
    box_(dst, p.paper, dst_l, p.ink_soft);
    // the stream over the gap, thickening at the end until it closes it
    let stream = ease(t / 0.25);
    let sh = 6.0 + 24.0 * ease((t - 0.55) / 0.35);
    let (a, b) = if from_sets { (r1.left() - 10.0 * stream, r1.left() + 1.5) } else { (r0.right() - 1.5, r0.right() + 10.0 * stream) };
    painter.rect_filled(Rect::from_min_max(pos2(a, r0.center().y - sh / 2.0), pos2(b, r0.center().y + sh / 2.0)), 3.0, gold);
    // the other filling from the side it is poured into, its front rounded
    let fill = ease((t - 0.2) / 0.5);
    if fill > 0.0 {
        let fw = (dst.width() - 3.0) * fill;
        let fr = if from_sets {
            Rect::from_min_max(pos2(dst.right() - 1.5 - fw, dst.top() + 1.5), pos2(dst.right() - 1.5, dst.bottom() - 1.5))
        } else {
            Rect::from_min_size(dst.min + vec2(1.5, 1.5), vec2(fw, h - 3.0))
        };
        painter.rect_filled(fr, 0.0, gold);
        if fill < 1.0 {
            let front = if from_sets { fr.left() } else { fr.right() };
            painter.circle_filled(pos2(front, fr.center().y), (h - 3.0) / 2.0 * (1.0 - fill).min(0.5), gold);
        }
        painter.rect_stroke(dst, 0.0, Stroke::new(1.5, Color32::BLACK), StrokeKind::Inside);
        jtext::draw(&painter, dst.center(), Align2::CENTER_CENTER, dst_l, &font, if fill > 0.5 { p.on_accent } else { p.ink_soft });
    }
}

/// How long a pulse lasts: one opening, or the three glow-ups.
fn pulse_length(dur: f64, again: bool) -> f64 {
    if again { glow_ups(dur).iter().map(|(s, d)| s + d).fold(0.0, f64::max) } else { dur }
}

/// The boxes showing `elapsed` seconds in, each at where it is (0 the box at the item, 1 grown and faded).
fn pulse_ts(elapsed: f64, dur: f64, again: bool) -> Vec<f32> {
    if !again {
        return vec![(elapsed / dur).clamp(0.0, 1.0) as f32];
    }
    glow_ups(dur)
        .iter()
        .filter(|(s, d)| elapsed >= *s && elapsed < s + d)
        .map(|(s, d)| ((elapsed - s) / d).clamp(0.0, 1.0) as f32)
        .collect()
}

/// What you hold and how much, as the Items page counts it (Java `collected`, `held`, `bestEnchants`).
struct Owned {
    /// The collectible items you hold, in the collection's order.
    items: Vec<i32>,
    set: std::collections::HashSet<i32>,
    best: HashMap<i32, i32>,
}

fn owned(app: &lg_app::App, upe_char: i32) -> Owned {
    let data = app.world.data();
    if upe_char != 0 {
        let g = gathered(app, upe_char);
        let items: Vec<i32> = g.keys().copied().collect();
        return Owned { set: items.iter().copied().collect(), items, best: g.iter().map(|(k, v)| (*k, v.1)).collect() };
    }
    let living = lg_app::loadout::living(app);
    let time = app.zone.local_date_time(app.world.now());
    let hctx = lg_app::holdings::Ctx { data, now: app.world.now(), time: &time };
    let known = app.holdings.known();
    let (counts, best) = if known { app.holdings.counts_and_best(&hctx, &living) } else { (HashMap::new(), HashMap::new()) };
    let mut best_of = HashMap::new();
    let mut items = Vec::new();
    for (id, e) in app.collection.all() {
        let n = if known { counts.get(&id).copied().unwrap_or(0) } else { e.held() };
        if n == 0 || !data.items.collectible.contains(&id) {
            continue;
        }
        items.push(id);
        best_of.insert(
            id,
            match best.get(&id) {
                Some(b) if known && *b >= 0 => *b,
                _ => e.max_enchants(),
            },
        );
    }
    Owned { set: items.iter().copied().collect(), items, best: best_of }
}

/// A button as FlatLaf draws it: header colour, black frame, the text in the button font (danger: red).
pub fn button(ui: &Ui, p: &Palette, r: Rect, label: &str, danger: bool, id: &str) -> bool {
    let painter = ui.painter();
    let resp = ui.interact(r, ui.scope_id().with(("button", id)), Sense::click());
    let (bg, fg, frame) =
        if danger { (p.paper, p.red, p.red) } else { (if resp.hovered() { p.sand } else { p.header }, p.ink, Color32::BLACK) };
    painter.rect(r, 0.0, bg, Stroke::new(1.5, frame), StrokeKind::Inside);
    jtext::draw(painter, r.center(), Align2::CENTER_CENTER, &label.to_uppercase(), &widgets::small(), fg);
    resp.clicked()
}

impl InventoryView {
    pub fn show(&mut self, ui: &mut Ui, c: &mut Ctx, rarity: &Rarity) {
        let p = *c.p;
        let app = c.app;
        let data = app.world.data();
        let all = ui.max_rect().shrink(8.0);
        let painter = ui.painter().clone();
        // the collection's icons made ahead, a few each frame: a grid sliding in has them all (made while it slid,
        // they popped in one by one)
        if self.warmed < data.items.collectible.len() {
            for id in data.items.collectible.iter().skip(self.warmed).take(60) {
                let _ = c.icons.object(ui.ctx(), data, *id, ICON as u32);
            }
            self.warmed += 60;
            ui.ctx().request_repaint();
        }
        // the focus: its character's own finds while it is on (a run's gathered items, else what it found)
        let focus = crate::focus::focused_char(app, c.props);
        let focus_id = focus.map(|ch| ch.char_id);
        if focus_id != self.focus_seen {
            if let Some(id) = focus_id {
                self.upe_char = id;
            } else if self.focus_seen == Some(self.upe_char) {
                self.upe_char = 0;
            }
            self.focus_seen = focus_id;
        }
        // the column of whose finds: sliding open and shut (`col_t` 0 shut, 1 open); with it open the white bag
        // stands over its middle and the pages start where the collection does, shut the bag is at the left and the
        // pages after it
        let column_on = c.props.get("collectionUpeColumn") == Some("true");
        let col_t = ui.ctx().animate_bool_with_time(ui.scope_id().with("upe-column-open"), column_on, 0.22);
        let col_t = col_t * col_t * (3.0 - 2.0 * col_t);
        let lerp = |a: f32, b: f32| a + (b - a) * col_t;
        let bag_x = lerp(all.left() + 8.0, all.left() + 4.0 + UPE_COLUMN_W / 2.0 - 4.0 - 17.0);
        let pages_x = lerp(all.left() + 8.0 + 34.0 + 8.0, all.left() + UPE_COLUMN_W);
        // the white bag (opens the column of whose finds), then the pages
        let bag_r = Rect::from_min_size(pos2(bag_x, all.top() + 1.0), vec2(34.0, 34.0));
        if let Some(t) = c.icons.object(ui.ctx(), data, WHITE_BAG, 34) {
            painter.image(t.id(), bag_r, uv(), Color32::WHITE);
        }
        // click: the column of whose finds are shown, on or off (remembered)
        if ui
            .interact(bag_r, ui.scope_id().with("upe-bag"), Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .on_hover_text("Show the whole collection or a UPE character's own finds")
            .clicked()
        {
            c.asks.push(crate::window::Ask::Set("collectionUpeColumn".into(), (!column_on).to_string()));
        }
        const PAGES: [&str; 4] = ["Items", "Sets", "Keys", "Log"];
        // Sets from Items: the two melt into one page; Sets again: the plain sets; Items: back to the items
        if let Some(i) = widgets::segmented(ui, &p, pos2(pages_x, all.top()), 36.0, &PAGES, self.page, false, &[]) {
            match (self.melt, i) {
                (false, 1) if self.page == 0 => {
                    self.melt = true;
                    self.melt_exit = 0;
                }
                (true, 1) => {
                    self.melt = false;
                    self.melt_exit = 1;
                    self.page = 1;
                }
                // on from the melt to Keys or Log: the melted box pours into it, the melt simply gone
                (true, _) if i >= 2 => {
                    self.melt = false;
                    self.melt_exit = 0;
                    self.melt_t = 0.0;
                    self.reveal = None;
                    self.page = i;
                    widgets::segmented_pour_merged(ui, pos2(pages_x, all.top()), &PAGES, 0, 1, "Items and Sets", i);
                }
                (true, _) => {
                    self.melt = false;
                    self.melt_exit = 0;
                    self.page = i;
                }
                // from the plain ST Sets, Items: first the two melted (the items slide in from the left); Items again
                // then the items alone
                (false, 0) if self.page == 1 => {
                    self.melt = true;
                    self.melt_exit = 1;
                    self.page = 0;
                }
                _ => self.page = i,
            }
        }
        let dt = ui.input(|i| i.stable_dt).min(0.1);
        // the melt has its own pour: the tabs' one not for it
        if self.melt || self.melt_t > 0.0 {
            widgets::segmented_settle(ui, pos2(pages_x, all.top()), &PAGES, self.page);
        }
        let goal = if self.melt { 1.0 } else { 0.0 };
        let speed = widgets::POUR_S;
        self.melt_t = if self.melt_t < goal { (self.melt_t + dt / speed).min(goal) } else { (self.melt_t - dt / speed).max(goal) };
        let mt = self.melt_t * self.melt_t * (3.0 - 2.0 * self.melt_t);
        if mt > 0.0 && self.page <= 1 {
            if self.melt && self.melt_t < 1.0 {
                // melting in: gold flows on from the tab left into the other, the first staying filled; the two
                // show as one once it is there
                let (from, to) = if self.melt_exit == 0 { (0, 1) } else { (1, 0) };
                widgets::pour_spread(ui, &p, pos2(pages_x, all.top()), 36.0, &PAGES, false, from, to, self.melt_t);
            } else {
                // parting: the two run apart again from the side the sets are on
                melt_tabs(ui, &p, pos2(pages_x, all.top()), 36.0, mt, self.melt_exit == 1);
            }
        }
        // the forge resources, right-aligned on the same row: the regular side at the right, the seasonal one before it
        let pages_end = pages_x + widgets::segmented_width(ui.painter(), &PAGES, false) + 10.0;
        crate::guide::mark(ui, "inventory.pages", Rect::from_min_max(pos2(pages_x, all.top()), pos2(pages_end - 10.0, all.top() + 36.0)));
        if let Some(r) = widgets::segment_rects(ui.painter(), pos2(pages_x, all.top()), 36.0, &PAGES, false).get(1) {
            crate::guide::mark(ui, "inventory.st.tab", *r);
        }
        resource_row(ui, &p, c.icons, app, pos2(all.right(), all.top()), 36.0, pages_end, focus.map(|ch| ch.seasonal));
        let body = Rect::from_min_max(pos2(all.left(), all.top() + 36.0 + 8.0), all.max);
        // another page: it slides in, except into and out of Items and ST Sets as one (that has its own motion)
        let shown = if self.melt { 0 } else { self.page };
        let (mut slid, mut body) = widgets::section_slide(ui, &p, shown, !self.melt && self.melt_t <= 0.0, body);
        let ui = &mut slid;
        // the optimization: its list in a column of its own, sliding out from the left
        let ot = ui.ctx().animate_bool_with_time(ui.scope_id().with("opt-column"), self.opt && self.page == 0 && !self.melt, 0.22);
        let ot = ot * ot * (3.0 - 2.0 * ot);
        if ot > 0.0 {
            let shown = OPT_COLUMN_W * ot;
            let at =
                Rect::from_min_max(pos2(body.left() + shown - OPT_COLUMN_W, body.top()), pos2(body.left() + shown - 8.0, body.bottom()));
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(at));
            child.set_clip_rect(Rect::from_min_max(body.min, pos2(body.left() + shown, body.bottom())).intersect(ui.clip_rect()));
            self.opt_column(&mut child, c, rarity, at);
            if ot >= 1.0 {
                crate::guide::mark(ui, "inventory.opt.column", at);
            }
            body = Rect::from_min_max(pos2(body.left() + shown, body.top()), body.max);
            if ot < 1.0 {
                ui.ctx().request_repaint();
            }
        } else if col_t > 0.0 {
            // it slides out from under the left edge
            let shown = UPE_COLUMN_W * col_t;
            let at = Rect::from_min_max(pos2(body.left() + shown - UPE_COLUMN_W, body.top()), pos2(body.left() + shown, body.bottom()));
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(at));
            child.set_clip_rect(Rect::from_min_max(body.min, pos2(body.left() + shown, body.bottom())).intersect(ui.clip_rect()));
            self.upe_column(&mut child, c, at);
            body = Rect::from_min_max(pos2(body.left() + shown, body.top()), body.max);
            if col_t < 1.0 {
                ui.ctx().request_repaint();
            }
        }
        // melted (or melting, or parting): the collection squeezed to the left, the sets coming in from the right;
        // parting to the sets the collection slides out to the left
        if mt > 0.0 && self.page <= 1 {
            let mid = body.left() + (body.width() * 0.5).round();
            // from or to the plain ST Sets the items slide at the left; from or to Items the sets come and go at the right
            let (items_r, sets_r) = if self.melt_exit == 0 {
                let split = body.right() + (mid - body.right()) * mt;
                (
                    Rect::from_min_max(body.min, pos2(split, body.bottom())),
                    Rect::from_min_max(pos2(split, body.top()), pos2(split + body.right() - mid, body.bottom())),
                )
            } else {
                let split = body.left() + (mid - body.left()) * mt;
                (
                    Rect::from_min_max(pos2(split - (mid - body.left()), body.top()), pos2(split, body.bottom())),
                    Rect::from_min_max(pos2(split, body.top()), body.max),
                )
            };
            // the set clicked: its pieces beat in the collection
            let beat = self.st_selected.clone().filter(|_| self.melt);
            if beat.as_ref() != self.beat.as_ref().map(|b| &b.0) {
                self.beat = beat.map(|b| (b, ui.input(|i| i.time)));
            }
            // parting: the page it goes to drawn as it will stand (its right column too), shown up to the moving edge:
            // back to Items the collection widens to its place, then its column comes in after it; on to ST Sets the
            // sets' page slides in from the right, its column last
            let parting = !self.melt;
            for (r, sets) in [(items_r, false), (sets_r, true)] {
                let shown = r.intersect(body);
                if shown.width() < 1.0 {
                    continue;
                }
                let whole = parting && sets == (self.melt_exit == 1);
                let mut child = ui.new_child(egui::UiBuilder::new().max_rect(if whole { body } else { r }));
                child.set_clip_rect(shown.intersect(ui.clip_rect()));
                if whole {
                    self.reveal = Some(if sets { r.left() } else { r.right() });
                    if sets {
                        let themed = self.sets_themed;
                        self.st_sets(&mut child, c, rarity, body, false, themed);
                    } else {
                        self.items(&mut child, c, rarity, body, false);
                    }
                    self.reveal = None;
                    continue;
                }
                if sets {
                    // a gap from the collection, as between the sections elsewhere
                    let r = Rect::from_min_max(pos2(r.left() + 8.0, r.top()), r.max);
                    let themed = self.sets_themed;
                    self.st_sets(&mut child, c, rarity, r, true, themed);
                } else {
                    self.items(&mut child, c, rarity, r, true);
                }
            }
            if mt < 1.0 {
                ui.ctx().request_repaint();
            }
            return;
        }
        self.beat = None;
        match self.page {
            1 => {
                let themed = self.sets_themed;
                self.st_sets(ui, c, rarity, body, false, themed)
            }
            2 => self.keys(ui, c, body),
            3 => self.log(ui, c, body),
            _ => self.items(ui, c, rarity, body, false),
        }
    }

    /// The column picking whose finds are shown: all, then per game mode its characters (Java `renderUpeColumn`).
    fn upe_column(&mut self, ui: &mut Ui, c: &mut Ctx, body: Rect) -> f32 {
        let p = *c.p;
        let app = c.app;
        let data = app.world.data();
        let painter = ui.painter().clone();
        let w = UPE_COLUMN_W;
        let x = body.left() + 4.0;
        let mut y = body.top() + 4.0;
        let f = small();
        // the newest run first (Java `markedRuns`); a dead character's only when it gathered at least 5 items for
        // the grid, so short-lived runs do not crowd the column
        let mut runs: Vec<&lg_app::upe::Run> = app
            .upe
            .runs
            .values()
            .filter(|r| r.marked)
            .filter(|r| {
                app.characters
                    .get(r.char_id)
                    .is_some_and(|ch| ch.alive || r.gathered.iter().filter(|g| data.items.collectible.contains(&g.item)).count() >= 5)
            })
            .collect();
        runs.sort_by_key(|r| std::cmp::Reverse(r.char_id));
        // the characters in the focus row (added with their eye) that have no run in the column: their own finds
        let in_focus: Vec<&lg_app::characters::Char> =
            crate::focus::char_candidates(app, c.props).into_iter().filter(|ch| !runs.iter().any(|r| r.char_id == ch.char_id)).collect();
        if self.upe_char != 0
            && !runs.iter().any(|r| r.char_id == self.upe_char)
            && !in_focus.iter().any(|ch| ch.char_id == self.upe_char)
            && self.focus_seen != Some(self.upe_char)
        {
            self.upe_char = 0; // it is not in the column (any more)
        }
        // the gold frame of the entry shown: drawn under the entries once their places are known, gliding from the
        // one shown before to the one clicked
        let frame_slot = painter.add(egui::Shape::Noop);
        let target = std::cell::Cell::new(None::<Rect>);
        // a click follows the focus: a character in the focus row becomes the focus again; any other entry (All, a
        // character not in the row) leaves it
        let focusable: Vec<i32> = crate::focus::candidates(app)
            .iter()
            .map(|(r, _)| r.char_id)
            .chain(crate::focus::char_candidates(app, c.props).iter().map(|ch| ch.char_id))
            .collect();
        let focus_on = self.focus_seen.is_some();
        let mut entry = |y: &mut f32, tex: Option<egui::TextureHandle>, name: &str, id: i32, faded: bool, tip: &[widgets::TipLine]| {
            let shown = self.upe_char == id;
            let lh = jtext::height(&painter, &f);
            let h = 4.0 + 40.0 + lh + 4.0 + 4.0;
            let r = Rect::from_min_size(pos2(x + (w - 8.0 - 96.0) / 2.0, *y + 3.0), vec2(96.0, h));
            if shown {
                target.set(Some(r));
            }
            if let Some(t) = tex {
                painter.image(
                    t.id(),
                    Rect::from_min_size(pos2(r.center().x - 20.0, r.top() + 6.0), vec2(40.0, 40.0)),
                    uv(),
                    Color32::WHITE,
                );
            }
            jtext::draw(
                &painter,
                pos2(r.center().x, r.top() + 6.0 + 40.0),
                Align2::CENTER_TOP,
                name,
                &f,
                if faded { fixed::MUTED } else { p.ink_soft },
            );
            if ui
                .interact(r, ui.scope_id().with(("upe-entry", id)), Sense::click())
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .clicked()
            {
                self.upe_char = id;
                if id != 0 && focusable.contains(&id) {
                    if self.focus_seen != Some(id) {
                        crate::window::ask_later(
                            ui.ctx(),
                            crate::window::Ask::Set(crate::focus::KEY.into(), crate::focus::value_for(app, Some(id))),
                        );
                    }
                } else if focus_on {
                    crate::window::ask_later(ui.ctx(), crate::window::Ask::Set(crate::focus::KEY.into(), String::new()));
                }
            }
            widgets::rich_tip(ui, r, ("upe-entry-tip", id), tip);
            *y += h + 6.0;
        };
        let bag = c.icons.object(ui.ctx(), data, WHITE_BAG, 40);
        entry(&mut y, bag, "All", 0, false, &[vec![("Your whole collection".to_string(), None, false)]]);
        for mode in lg_app::upe::MODES {
            let in_mode: Vec<_> = runs.iter().filter(|r| r.mode == mode).collect();
            if in_mode.is_empty() {
                continue;
            }
            y += 10.0;
            let t = jtext::draw(&painter, pos2(x + 6.0, y), Align2::LEFT_TOP, &lg_app::upe::label(mode), &f, p.gold);
            y = t.bottom() + 2.0;
            painter.rect_filled(Rect::from_min_size(pos2(x + 6.0, y), vec2(w - 8.0 - 12.0, 2.0)), 0.0, p.gold);
            y += 2.0 + 4.0;
            for r in in_mode {
                let ch = app.characters.get(r.char_id).expect("checked");
                let skin = if ch.skin > 0 { ch.skin } else { ch.class_type };
                let faded = r.broken.is_some() || !ch.alive;
                let tex = if faded {
                    c.icons.skin_faded(ui.ctx(), data, skin, ch.tex1, ch.tex2, 40)
                } else {
                    c.icons.skin(ui.ctx(), data, skin, ch.tex1, ch.tex2, 40)
                };
                let n = r.gathered.len();
                let mut tip: Vec<widgets::TipLine> = vec![
                    vec![(ch.class_name.clone(), None, true), (format!(" ({})", lg_app::upe::label(mode)), None, false)],
                    vec![(format!("{n} item{} gathered", if n == 1 { "" } else { "s" }), None, false)],
                ];
                for l in lg_app::upe::describe(&app.zone, Some(r)).lines() {
                    tip.push(vec![(l.to_string(), None, false)]);
                }
                entry(&mut y, tex, &ch.class_name, r.char_id, faded, &tip);
            }
        }
        for (heading, group) in [("Focus", &in_focus)] {
            if group.is_empty() {
                continue;
            }
            y += 10.0;
            let t = jtext::draw(&painter, pos2(x + 6.0, y), Align2::LEFT_TOP, heading, &f, p.gold);
            y = t.bottom() + 2.0;
            painter.rect_filled(Rect::from_min_size(pos2(x + 6.0, y), vec2(w - 8.0 - 12.0, 2.0)), 0.0, p.gold);
            y += 2.0 + 4.0;
            for ch in group.iter() {
                let skin = if ch.skin > 0 { ch.skin } else { ch.class_type };
                let tex = c.icons.skin(ui.ctx(), data, skin, ch.tex1, ch.tex2, 40);
                let n: i32 = found_by(app, ch.char_id).values().map(|v| v.0).sum();
                let tip: Vec<widgets::TipLine> = vec![
                    vec![(ch.class_name.clone(), None, true), (if ch.seasonal { " (seasonal)" } else { "" }.to_string(), None, false)],
                    vec![(format!("{n} item{} found while played", if n == 1 { "" } else { "s" }), None, false)],
                    vec![("Drops count for the character you were playing when they came.".to_string(), Some(fixed::MUTED), false)],
                ];
                entry(&mut y, tex, &ch.class_name, ch.char_id, false, &tip);
            }
        }
        if runs.is_empty() && in_focus.is_empty() {
            jtext::draw(&painter, pos2(x + w / 2.0, y + 8.0), Align2::CENTER_TOP, "No game mode runs or focus yet", &f, fixed::MUTED);
        }
        if let Some(to) = target.get() {
            // a third of the way every 15 ms, however long the frame took
            let id = egui::Id::unique("lg-upe-column-frame");
            let now = ui.input(|i| i.time);
            let (from, at) = ui.ctx().data(|d| d.get_temp::<(Rect, f64)>(id)).unwrap_or((to, now));
            let k = (1.0 - 0.67f64.powf(((now - at).clamp(0.0, 1.0)) / 0.015)) as f32;
            let lerp = |a: f32, b: f32| a + (b - a) * k;
            let mut shown = Rect::from_min_max(
                pos2(lerp(from.left(), to.left()), lerp(from.top(), to.top())),
                pos2(lerp(from.right(), to.right()), lerp(from.bottom(), to.bottom())),
            );
            if (shown.top() - to.top()).abs() < 0.5 && (shown.bottom() - to.bottom()).abs() < 0.5 {
                shown = to;
            } else {
                ui.ctx().request_repaint();
            }
            ui.ctx().data_mut(|d| d.insert_temp(id, (shown, now)));
            painter.set(
                frame_slot,
                egui::Shape::Vec(vec![
                    egui::epaint::RectShape::filled(shown, 0.0, mix(p.gold, p.paper, 0.25)).with_round_to_pixels(false).into(),
                    egui::epaint::RectShape::stroke(shown, 0.0, Stroke::new(2.0, p.gold), StrokeKind::Inside)
                        .with_round_to_pixels(false)
                        .into(),
                ]),
            );
        }
        w
    }

    /// The Items page: the grid beside progress, actions and recent finds.
    fn items(&mut self, ui: &mut Ui, c: &mut Ctx, rarity: &Rarity, body: Rect, grid_only: bool) {
        let p = *c.p;
        let app = c.app;
        let data = app.world.data();
        let living = lg_app::loadout::living(app);
        // copies you hold and the best rarity per item: Holdings once it knows, else the collection's
        let time = app.zone.local_date_time(app.world.now());
        let hctx = lg_app::holdings::Ctx { data, now: app.world.now(), time: &time };
        let (counts, best) =
            if app.holdings.known() { app.holdings.counts_and_best(&hctx, &living) } else { (HashMap::new(), HashMap::new()) };
        let known = app.holdings.known();
        let upe_items: HashMap<i32, (i32, i32, String, String)> = gathered(app, self.upe_char);
        let held = |id: i32| -> i32 {
            if self.upe_char != 0 {
                return upe_items.get(&id).map_or(0, |e| e.0);
            }
            if known { counts.get(&id).copied().unwrap_or(0) } else { app.collection.entry(id).map_or(0, |e| e.held()) }
        };
        let best_of = |id: i32| -> i32 {
            if self.upe_char != 0 {
                return upe_items.get(&id).map_or(0, |e| e.1);
            }
            match best.get(&id) {
                Some(b) if known && *b >= 0 => *b,
                _ => app.collection.entry(id).map_or(0, |e| e.max_enchants()),
            }
        };
        // what you hold of the collectible items (Java `collected`), in the collection's order
        let owned: Vec<i32> = if self.upe_char != 0 {
            upe_items.keys().copied().collect()
        } else {
            app.collection
                .all()
                .map(|(id, _)| id)
                .filter(|id| data.items.item(*id).is_some_and(|_| data.items.collectible.contains(id)) && held(*id) > 0)
                .collect()
        };
        let owned_set: std::collections::HashSet<i32> = owned.iter().copied().collect();
        let universe: Vec<i32> = data
            .items
            .collectible
            .iter()
            .copied()
            .filter(|id| {
                let it = data.items.item(*id).expect("collectible items are known");
                match self.filter {
                    3 => it.quickslot_only,
                    1 => it.is_shiny(),
                    2 => !it.is_shiny(),
                    _ => true,
                }
            })
            .collect();
        let q = self.search.trim().to_lowercase();
        // Java `matching`: name, type (slot), UT / ST / T14, "reusable" / "quick slot", or an enchantment found on it
        let has_enchant = |id: i32| {
            if self.upe_char != 0 && !owned_set.contains(&id) {
                return false;
            }
            app.collection.entry(id).filter(|_| owned_set.contains(&id)).is_some_and(|e| {
                e.history
                    .iter()
                    .flatten()
                    .filter_map(|f| f.enchant_ids.as_ref())
                    .flatten()
                    .any(|eid| *eid >= 0 && data.enchant_looks.get(eid).is_some_and(|l| l.name.to_lowercase().contains(&q)))
            })
        };
        let grid: Vec<i32> = universe
            .into_iter()
            .filter(|id| {
                q.is_empty()
                    || data.items.item(*id).is_some_and(|i| {
                        i.name.to_lowercase().contains(&q)
                            || i.slot_name().to_lowercase().contains(&q)
                            || i.tier_tag().to_lowercase().replace(['(', ')'], "") == q
                            || (!i.tier.is_empty() && q == format!("t{}", i.tier))
                            || (i.quickslot_only
                                && ("reusable".starts_with(&q) || "quick slot".starts_with(&q) || "quickslot".starts_with(&q)))
                    })
                    || has_enchant(*id)
            })
            .collect();
        // an item opened from elsewhere: its history, and the item pulsing in the grid; hidden by the filter or the
        // search, those are cleared first (Java `showHistory`)
        let time_s = ui.input(|i| i.time);
        if let Some(id) = self.show_item {
            self.history = Some(id);
            if grid.contains(&id) {
                self.pulse = Some((id, time_s, PULSE_S, true));
                self.scroll_to = Some(id);
                self.show_item = None;
            } else if self.filter != 0 || !self.search.is_empty() {
                self.filter = 0;
                self.search.clear();
                ui.ctx().request_repaint();
            } else {
                self.show_item = None;
            }
        }

        // the split: the side 380 px, the grid snapped down to whole columns
        let overhead = 2.0 + 12.0;
        let base = overhead + 2.0 * PAD;
        let (main, side) = if grid_only {
            (body, Rect::NOTHING)
        } else {
            widgets::side_split(ui, c.p, c.props, "collectionGrid", body, 380.0, 200.0, 560.0, Some((base, CELL)))
        };
        // parting back to here: the grid as wide as up to the moving edge (its column after it, shown as the edge passes)
        let main = match self.reveal.filter(|_| !grid_only) {
            Some(edge) => Rect::from_min_max(main.min, pos2(main.right().min(edge).max(main.left() + CELL), main.bottom())),
            None => main,
        };
        let cols = ((main.width() - base + 0.001) / CELL).floor().max(1.0);
        // melted with a set clicked: its pieces beat (two glow-ups, then 1 s of rest), the rest dimmed
        let beat_pieces: Vec<i32> = self
            .beat
            .as_ref()
            .filter(|_| grid_only)
            .and_then(|(n, _)| data.st_sets.iter().chain(&data.themed_sets).find(|s| s.name == *n))
            .map(|s| s.pieces.to_vec())
            .unwrap_or_default();
        let beat_since = self.beat.as_ref().map_or(0.0, |b| b.1);
        // the optimization: the items you hold spare copies of
        let spare_items: std::collections::HashSet<i32> =
            if self.opt { spares(app, c.props, self.opt_sort).into_iter().map(|s| s.item).collect() } else { Default::default() };
        let title = if self.upe_char != 0 {
            let name = match (app.characters.get(self.upe_char), app.upe.runs.get(&self.upe_char).filter(|r| r.marked)) {
                (Some(ch), Some(r)) => format!("{} ({})", ch.class_name, lg_app::upe::label(&r.mode)),
                (Some(ch), None) => format!("{} (found while played)", ch.class_name),
                _ => "UPE".to_string(),
            };
            format!("Collection · {name}")
        } else {
            "Collection".to_string()
        };
        let mut filter = self.filter;
        // the grid's cells as last drawn, for "Copy as image"
        let grid_cells: std::cell::RefCell<(Vec<Cell>, usize)> = Default::default();
        let mut search = self.search.clone();
        let icons = &mut *c.icons;
        let clicked_cell = std::cell::Cell::new(None);
        let pulse = self.pulse.filter(|(_, start, dur, again)| time_s - start < pulse_length(*dur, *again));
        let scroll_to = self.scroll_to.take();
        crate::guide::mark(ui, "inventory.grid", main);
        widgets::section(
            ui,
            &p,
            main,
            &title,
            fixed::GOLD,
            |ui, head| {
                let painter = ui.painter().clone();
                let h = head.height();
                let fw = widgets::segmented_width(&painter, &FILTERS, true);
                let fx = head.right() - fw;
                if let Some(i) = widgets::segmented(ui, &p, pos2(fx, head.top()), h, &FILTERS, filter, true, &[]) {
                    filter = i;
                }
                // the search box
                // narrower when the section is (melted): the title keeps its room
                let sw = if head.width() < 760.0 { 150.0 } else { 220.0 };
                let sr = Rect::from_min_size(pos2(fx - 8.0 - sw, head.top() + 3.0), vec2(sw, h - 6.0));
                crate::widgets::search_field(ui, &p, sr, &mut search, "Search items or enchants", "collection-search");
                widgets::tip(
                    ui,
                    sr,
                    "search-tip",
                    "Item name, type (e.g. Katana, Ring), UT / ST / T14, or an enchantment on items you own",
                );
                widgets::tip(
                    ui,
                    Rect::from_min_size(pos2(fx, head.top()), vec2(fw, h)),
                    "filter-tip",
                    "Reusable: items used again and again from a quick slot, e.g. the Coral Juice Refinery",
                );
            },
            |ui, r| {
                egui::ScrollArea::vertical().id_salt("collection-grid").show(ui, |ui| {
                    let top = ui.cursor().min;
                    let gw = r.width() - 12.0;
                    let rows = (grid.len() as f32 / cols).ceil();
                    let gh = rows * CELL + 2.0 * PAD;
                    let painter = ui.painter().clone();
                    painter.rect_filled(Rect::from_min_size(top, vec2(gw, gh.max(r.height()))), 0.0, BG);
                    let cells: Vec<Cell> =
                        grid.iter().map(|id| Cell { id: *id, have: owned_set.contains(id), best: best_of(*id), held: held(*id) }).collect();
                    paint_cells(&painter, ui.ctx(), icons, data, rarity, &cells, cols as usize, top);
                    let n = cols as usize;
                    let cell_rect = |i: usize| {
                        Rect::from_min_size(
                            pos2(top.x + PAD + (i % n) as f32 * CELL, top.y + PAD + (i / n) as f32 * CELL),
                            vec2(CELL, CELL),
                        )
                    };
                    let (_, resp) = ui.allocate_exact_size(vec2(gw, gh), Sense::click());
                    // hover: a light frame; a click: the item's history and a short pulse
                    let at = |q: egui::Pos2| {
                        let (cx, cy) = ((q.x - top.x - PAD) / CELL, (q.y - top.y - PAD) / CELL);
                        let i = cy.floor() as usize * n + cx.floor() as usize;
                        (cx >= 0.0 && cy >= 0.0 && cx < cols && i < cells.len()).then_some(i)
                    };
                    // the hover light (Java `paintLight`): a soft glow under the mouse that lights up the cell
                    // borders near it, fading in (120 ms) and out (350 ms)
                    let lid = egui::Id::unique("lg-grid-light");
                    let (mut lp, mut la, lt) = ui.ctx().data(|d| d.get_temp::<(egui::Pos2, f32, f64)>(lid)).unwrap_or((top, 0.0, time_s));
                    let dt = ((time_s - lt) * 1000.0) as f32;
                    let on = resp.hovered();
                    if let Some(q) = resp.hover_pos() {
                        lp = q;
                    }
                    la = if on { (la + dt / 120.0).min(1.0) } else { (la - dt / 350.0).max(0.0) };
                    ui.ctx().data_mut(|d| d.insert_temp(lid, (lp, la, time_s)));
                    if la > 0.0 {
                        const RADIUS: f32 = 90.0;
                        let white = |k: f32| Color32::from_white_alpha((255.0 * k * la).round() as u8);
                        let ring = |r: f32| {
                            (0..48).map(move |i| {
                                let a = i as f32 / 48.0 * std::f32::consts::TAU;
                                lp + vec2(a.cos(), a.sin()) * r
                            })
                        };
                        let mut mesh = egui::Mesh::default();
                        mesh.colored_vertex(lp, white(0.13));
                        for q in ring(RADIUS * 0.45) {
                            mesh.colored_vertex(q, white(0.05));
                        }
                        for q in ring(RADIUS) {
                            mesh.colored_vertex(q, Color32::TRANSPARENT);
                        }
                        for i in 0..48u32 {
                            let j = (i + 1) % 48;
                            mesh.add_triangle(0, 1 + i, 1 + j);
                            mesh.add_triangle(1 + i, 49 + i, 49 + j);
                            mesh.add_triangle(1 + i, 49 + j, 1 + j);
                        }
                        painter.add(egui::Shape::mesh(mesh));
                        for i in 0..cells.len() {
                            let cr = cell_rect(i);
                            let d = (cr.center() - lp).length() / RADIUS;
                            if d >= 1.0 {
                                continue;
                            }
                            let strength = (1.0 - d) * (1.0 - d);
                            painter.rect_stroke(
                                cr.shrink(0.5),
                                0.0,
                                Stroke::new(1.0, white((0.38 * strength).min(1.0))),
                                StrokeKind::Inside,
                            );
                        }
                        if la < 1.0 {
                            ui.ctx().request_repaint();
                        }
                    }
                    if let Some(i) = resp.hover_pos().and_then(at) {
                        // the cell under the mouse: a light gold frame
                        let light_gold = crate::theme::mix(p.gold, Color32::WHITE, 0.5);
                        painter.rect_stroke(cell_rect(i).shrink(1.0), 0.0, Stroke::new(2.0, light_gold), StrokeKind::Inside);
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        if resp.clicked() {
                            clicked_cell.set(Some(cells[i].id));
                        }
                    }
                    // the item's name and count beside the mouse (Java `CollectionGrid` tip): every move hides it, it
                    // shows once the mouse rests on an item for 200 ms and fades in over 150 ms
                    let tid = egui::Id::unique("lg-grid-tip");
                    // (where the mouse rests, since when, when the tip showed)
                    let (mut rest, mut since, mut shown) =
                        ui.ctx().data(|d| d.get_temp::<(egui::Pos2, f64, Option<f64>)>(tid)).unwrap_or((egui::Pos2::ZERO, time_s, None));
                    match resp.hover_pos() {
                        Some(q) if ui.input(|i| i.pointer.any_down()) => {
                            rest = q;
                            since = f64::INFINITY; // pressed: no tip until the mouse moves again
                            shown = None;
                        }
                        Some(q) if q != rest => {
                            rest = q;
                            since = time_s;
                            shown = None;
                        }
                        Some(_) => {}
                        None => {
                            since = f64::INFINITY;
                            shown = None;
                        }
                    }
                    if shown.is_none() && since.is_finite() {
                        let wait = since + 0.2 - time_s;
                        if wait <= 0.0 {
                            shown = Some(time_s);
                        } else {
                            ui.ctx().request_repaint_after_secs(wait as f32);
                        }
                    }
                    ui.ctx().data_mut(|d| d.insert_temp(tid, (rest, since, shown)));
                    if let Some(at_s) = shown
                        && let Some(i) = at(rest)
                    {
                        let alpha = ((time_s - at_s) / 0.15).clamp(0.0, 1.0) as f32;
                        if alpha < 1.0 {
                            ui.ctx().request_repaint();
                        }
                        let it = data.items.item(cells[i].id);
                        let name = it.map_or(String::new(), |it| {
                            if it.is_shiny() && !it.name.contains("Shiny") { format!("{} (Shiny)", it.name) } else { it.name.clone() }
                        });
                        let (sub, sub_c) = if app.collection.entry(cells[i].id).is_none() && cells[i].held == 0 {
                            ("Not found yet".to_string(), fixed::MUTED)
                        } else {
                            (format!("Count: {}", cells[i].held), Color32::from_rgb(0xF5, 0xC5, 0x42))
                        };
                        let f = text(18.0);
                        let tip_painter =
                            ui.ctx().layer_painter(egui::LayerId::new(egui::Order::Tooltip, tid)).with_clip_rect(painter.clip_rect());
                        let lh = jtext::height(&tip_painter, &f);
                        let (pad_x, pad_y, gap) = (8.0, 5.0, 1.0);
                        let w = jtext::width(&tip_painter, &name, &f).max(jtext::width(&tip_painter, &sub, &f)) + 2.0 * pad_x;
                        let h = 2.0 * lh + gap + 2.0 * pad_y;
                        // below-right of the mouse, flipped to stay inside the visible part of the grid
                        let vis = painter.clip_rect();
                        let mut x = rest.x + 14.0;
                        let mut y = rest.y + 18.0;
                        if x + w > vis.right() - 2.0 {
                            x = rest.x - w - 8.0;
                        }
                        if y + h > vis.bottom() - 2.0 {
                            y = rest.y - h - 8.0;
                        }
                        let b = Rect::from_min_size(pos2(x.max(vis.left() + 2.0), y.max(vis.top() + 2.0)), vec2(w, h));
                        let fade = |c: Color32| c.gamma_multiply(alpha);
                        tip_painter.rect_filled(b, 0.0, fade(p.paper));
                        tip_painter.rect_stroke(b, 0.0, Stroke::new(1.0, fade(p.line)), StrokeKind::Inside);
                        jtext::draw(&tip_painter, pos2(b.left() + pad_x, b.top() + pad_y), Align2::LEFT_TOP, &name, &f, fade(p.ink));
                        jtext::draw(
                            &tip_painter,
                            pos2(b.left() + pad_x, b.top() + pad_y + lh + gap),
                            Align2::LEFT_TOP,
                            &sub,
                            &f,
                            fade(sub_c),
                        );
                    }
                    if !spare_items.is_empty() {
                        for (i, cell) in cells.iter().enumerate() {
                            if spare_items.contains(&cell.id) {
                                painter.rect_stroke(cell_rect(i).shrink(1.0), 0.0, Stroke::new(2.0, p.gold), StrokeKind::Inside);
                            } else {
                                painter.rect_filled(cell_rect(i), 0.0, Color32::from_black_alpha(110));
                            }
                        }
                    }
                    if !beat_pieces.is_empty() {
                        for (i, cell) in cells.iter().enumerate() {
                            if !beat_pieces.contains(&cell.id) {
                                painter.rect_filled(cell_rect(i), 0.0, Color32::from_black_alpha(160));
                            }
                        }
                        for t in heartbeat(time_s - beat_since) {
                            let eased = 1.0 - (1.0 - t) * (1.0 - t);
                            let gold = Color32::from_rgb(0xF5, 0xC5, 0x42).gamma_multiply(1.0 - t);
                            for (i, cell) in cells.iter().enumerate() {
                                if beat_pieces.contains(&cell.id) {
                                    widgets::smooth_stroke(
                                        &painter,
                                        cell_rect(i).expand(eased * PULSE_GROW),
                                        0.0,
                                        Stroke::new(5.0, gold),
                                        StrokeKind::Middle,
                                    );
                                }
                            }
                        }
                        ui.ctx().request_repaint();
                    }
                    if let Some((id, start, dur, again)) = pulse
                        && let Some(i) = cells.iter().position(|c| c.id == id)
                    {
                        for t in pulse_ts(time_s - start, dur, again) {
                            let eased = 1.0 - (1.0 - t) * (1.0 - t);
                            let gold = Color32::from_rgb(0xF5, 0xC5, 0x42).gamma_multiply(1.0 - t);
                            widgets::smooth_stroke(
                                &painter,
                                cell_rect(i).expand(eased * PULSE_GROW),
                                0.0,
                                Stroke::new(5.0, gold),
                                StrokeKind::Middle,
                            );
                        }
                        if scroll_to == Some(id) {
                            ui.scroll_to_rect(cell_rect(i), Some(egui::Align::Center));
                        }
                        ui.ctx().request_repaint();
                    }
                    *grid_cells.borrow_mut() = (cells, cols as usize);
                });
            },
        );
        self.filter = filter;
        self.search = search;
        self.pulse = pulse;
        if let Some(id) = clicked_cell.get() {
            self.history = Some(id);
            self.pulse = Some((id, time_s, CLICK_PULSE_S, false));
            // melted: a piece of the set clicked shows alone in its bar; any other item, all four again
            if grid_only {
                self.st_piece = beat_pieces.contains(&id).then_some(id);
            }
        }
        if grid_only {
            return;
        }

        // the side: progress and actions on top, recent finds below
        let painter = ui.painter().clone();
        let lh = jtext::height(&painter, &text(18.0));
        let head = widgets::section_head_height(&painter);
        let have = grid.iter().filter(|id| owned_set.contains(id)).count();
        let pct = if grid.is_empty() { 0.0 } else { 100.0 * have as f64 / grid.len() as f64 };
        let mut side = side;
        // the Optimization section, always on top of the side; a click on it shows or hides the spare copies
        {
            let was_melt = self.melt;
            let or = Self::opt_section(
                ui,
                &p,
                side,
                &mut self.opt_sort,
                &mut self.melt,
                &mut self.melt_exit,
                c.props,
                c.asks,
                rarity,
                &mut self.opt,
            );
            // crafting melts in the ST sets
            if self.melt && !was_melt {
                self.sets_themed = false;
            }
            side = Rect::from_min_max(pos2(side.left(), or.bottom() + 8.0), side.max);
        }
        let prog = Rect::from_min_size(
            side.min,
            vec2(side.width(), head + 2.0 + 20.0 + jtext::height(&painter, &small()) + 4.0 + 12.0 + 4.0 + lh),
        );
        widgets::section(
            ui,
            &p,
            prog,
            "Progress",
            fixed::GREEN,
            |_, _| {},
            |ui, r| {
                let painter = ui.painter();
                let (x, w) = (r.left() + 10.0, r.width() - 20.0);
                let t = jtext::draw(painter, pos2(x, r.top() + 10.0), Align2::LEFT_TOP, "COLLECTED", &small(), p.ink_muted);
                let by = t.bottom() + 4.0;
                painter.rect_filled(Rect::from_min_size(pos2(x, by), vec2(w, 12.0)), 0.0, p.line);
                painter.rect_filled(Rect::from_min_size(pos2(x, by), vec2((w * pct as f32 / 100.0).round(), 12.0)), 0.0, p.gold);
                jtext::draw(
                    painter,
                    pos2(x, by + 12.0 + 4.0),
                    Align2::LEFT_TOP,
                    &format!("{have} / {}   {pct:.1}%", grid.len()),
                    &text(18.0),
                    p.ink,
                );
            },
        );
        let bh = 36.0;
        let acts =
            Rect::from_min_size(pos2(side.left(), prog.bottom() + 8.0), vec2(side.width(), head + 2.0 + 20.0 + 3.0 * bh + 2.0 * 6.0));
        let upe_shown = self.upe_char != 0;
        let mut clicked: Option<usize> = None;
        widgets::section(
            ui,
            &p,
            acts,
            "Actions",
            fixed::BLUE,
            |_, _| {},
            |ui, r| {
                let (x, w) = (r.left() + 10.0, r.width() - 20.0);
                let mut y = r.top() + 10.0;
                for (i, (label, danger)) in
                    [("Copy as text", false), ("Copy as image", false), ("Clear collection", true)].iter().enumerate()
                {
                    if *danger && upe_shown {
                        continue;
                    }
                    if button(ui, &p, Rect::from_min_size(pos2(x, y), vec2(w, bh)), label, *danger, &format!("inv-{i}")) {
                        clicked = Some(i);
                    }
                    y += bh + 6.0;
                }
            },
        );
        if clicked == Some(1) || std::mem::take(&mut self.copy_requested) {
            // Java `copyImage`: the grid as one image, with the columns it shows now
            let (cells, cols) = grid_cells.take();
            self.grab = Some(Grab { cells, cols: cols.max(1), chunk: 0, frames: 0, pieces: Default::default() });
        }
        match clicked {
            // Java `copyText`: "- Name (Shiny) xN" per item held, sorted, under a head line
            Some(0) => {
                let mut lines: Vec<String> = owned
                    .iter()
                    .map(|id| {
                        let item = data.items.item(*id);
                        let mut line = item
                            .filter(|i| !i.name.is_empty())
                            .map_or_else(|| data.object_name(*id).map_or(format!("Item #{id}"), str::to_string), |i| i.name.clone());
                        if item.is_some_and(|i| i.is_shiny()) && !line.contains("Shiny") {
                            line += " (Shiny)";
                        }
                        let n = held(*id);
                        if n > 1 {
                            line += &format!(" x{n}");
                        }
                        line
                    })
                    .collect();
                lines.sort_by_key(|l| l.to_lowercase());
                let mut t = format!("ROTMG Collection \u{2014} {} items\n", owned.len());
                for l in lines {
                    t += &format!("- {l}\n");
                }
                ui.ctx().copy_text(t);
                c.asks.push(crate::window::Ask::Confirm {
                    title: "Copy as text".into(),
                    text: "Collection copied to the clipboard as text.".into(),
                    yes: "OK".into(),
                    no: String::new(),
                    then: Box::new(crate::window::Ask::Nothing),
                });
            }
            Some(2) => c.asks.push(crate::window::Ask::Confirm {
                title: "Clear collection".into(),
                text: "Delete your whole collection history? This cannot be undone.".into(),
                yes: "Yes".into(),
                no: "No".into(),
                then: Box::new(crate::window::Ask::ClearCollection),
            }),
            _ => {}
        }
        let recent = Rect::from_min_max(pos2(side.left(), acts.bottom() + 8.0), side.max);
        let icons = &mut *c.icons;
        let mut by_last: Vec<(i32, String, String, i32)> = if self.upe_char != 0 {
            upe_items.iter().map(|(id, e)| (*id, e.3.clone(), e.2.clone(), e.1)).collect()
        } else {
            owned
                .iter()
                .filter_map(|id| {
                    app.collection
                        .entry(*id)
                        .map(|e| (*id, e.last.clone().unwrap_or_else(|| "null".into()), e.last_date().to_string(), newest_enchants(e)))
                })
                .collect()
        };
        by_last.sort_by(|a, b| b.1.cmp(&a.1));
        let today = app.zone.local_date_time(app.world.now().max(c.now));
        // a picked item's history in place of the Recent list (Java `renderHistory`); "Recent" goes back
        if let Some(item) = self.history.filter(|id| data.items.item(*id).is_some()) {
            let mut back = false;
            let c_now = c.now;
            let count = held(item);
            widgets::section(
                ui,
                &p,
                recent,
                "History",
                fixed::PURPLE,
                |ui, head| {
                    let bw = jtext::width(ui.painter(), "RECENT", &small()) + 32.0;
                    back = button(
                        ui,
                        &p,
                        Rect::from_min_size(pos2(head.right() - bw, head.top()), vec2(bw, head.height())),
                        "Recent",
                        false,
                        "back-recent",
                    );
                },
                |ui, r| {
                    crate::item_history::show(
                        ui,
                        &p,
                        app,
                        icons,
                        rarity,
                        item,
                        count,
                        c_now,
                        r,
                        app.upe.runs.get(&self.upe_char).filter(|r| r.marked),
                    )
                },
            );
            if back {
                self.history = None;
            }
            return;
        }
        let picked_recent = std::cell::Cell::new(None);
        widgets::section(
            ui,
            &p,
            recent,
            "Recent",
            fixed::PURPLE,
            |_, _| {},
            |ui, r| {
                let painter = ui.painter().clone();
                let (x, w) = (r.left() + 10.0, r.width() - 20.0);
                let mut y = r.top() + 8.0;
                if by_last.is_empty() {
                    jtext::draw(&painter, pos2(x, y), Align2::LEFT_TOP, "Nothing collected yet.", &text(18.0), p.ink);
                }
                let mut day: Option<String> = None;
                for (i, (id, _, date, ench)) in by_last.iter().take(RECENT).enumerate() {
                    if day.as_deref() != Some(date.as_str()) {
                        day = Some(date.clone());
                        if i > 0 {
                            y += 4.0;
                            painter.rect_filled(Rect::from_min_size(pos2(x, y), vec2(w, 1.0)), 0.0, p.line);
                            y += 2.0;
                        }
                        y += 4.0;
                        let t = jtext::draw(&painter, pos2(x, y), Align2::LEFT_TOP, &day_label(date, &today), &small(), fixed::MUTED);
                        y = t.bottom() + 4.0;
                    }
                    let rh = 36.0;
                    if let Some(t) = icons.object(ui.ctx(), data, *id, 32) {
                        let ir = Rect::from_min_size(pos2(x, y + (rh - 32.0) / 2.0), vec2(32.0, 32.0));
                        painter.image(t.id(), ir, uv(), Color32::WHITE);
                        enchant_badge(&painter, rarity, ir.right_bottom() - vec2(1.0, 1.0), (*ench).max(0) as usize, 5.0);
                    }
                    let name = data.items.item(*id).map_or(format!("Item {id}"), |it| it.name.clone());
                    jtext::draw(
                        &painter,
                        pos2(x + 32.0 + 10.0, y + rh / 2.0),
                        Align2::LEFT_CENTER,
                        &widgets::fit(&painter, &name, &text(18.0), w - 42.0),
                        &text(18.0),
                        p.ink,
                    );
                    // click: the item in the grid (scrolled to, pulsing) and its history
                    let row = Rect::from_min_size(pos2(x, y), vec2(w, rh));
                    if ui
                        .interact(row, ui.scope_id().with(("recent", id)), Sense::click())
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .clicked()
                    {
                        picked_recent.set(Some(*id));
                    }
                    y += rh + 4.0;
                }
            },
        );
        if let Some(id) = picked_recent.get() {
            self.show_item = Some(id);
        }
    }
}

impl InventoryView {
    /// The Log page: every item you hold, newest find first: when first found, what, how many (Java `logView`).
    /// Inventory > Keys: every dungeon key and mystery key you hold, on your characters and in your vaults, the
    /// regular side and the seasonal one apart (a gold bar over each, as Battles' days), most held first; each card
    /// says where its copies are. With a character picked in the column, only its keys.
    fn keys(&mut self, ui: &mut Ui, c: &mut Ctx, body: Rect) {
        const CARD_MIN_WIDTH: f32 = 250.0;
        const GAP: f32 = 8.0;
        const PLACES: usize = 4;
        let p = *c.p;
        let app = c.app;
        let data = app.world.data();
        let is_key =
            |id: i32| data.items.item(id).is_some_and(|i| i.labels.contains("KEY") || i.name.to_lowercase().ends_with("mystery key"));
        let living: HashMap<i32, bool> = app.characters.living().iter().map(|l| (l.char_id, l.seasonal)).collect();
        let mut copies = app.holdings.copies_of(&living, is_key);
        if self.upe_char != 0 {
            for v in copies.values_mut() {
                v.retain(|h| h.char_id == self.upe_char);
            }
            copies.retain(|_, v| !v.is_empty());
        }
        let name = |id: i32| data.items.item(id).map_or(format!("Item {id}"), |i| i.name.clone());
        // per side: each key with that side's copies (a place whose side is not known counts as regular)
        let side_of = |seasonal: bool| {
            let mut keys: Vec<(i32, Vec<lg_app::loadout::Held>)> = copies
                .iter()
                .map(|(id, held)| (*id, held.iter().filter(|h| (h.seasonal == Some(true)) == seasonal).cloned().collect::<Vec<_>>()))
                .filter(|(_, held)| !held.is_empty())
                .collect();
            keys.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(name(a.0).cmp(&name(b.0))));
            keys
        };
        let groups = [("Regular", side_of(false)), ("Seasonal", side_of(true))];
        let selected = self.key_selected;
        let selected_seasonal = self.key_seasonal;
        let mut pick: Option<Option<i32>> = None;
        let mut pick_seasonal = false;
        // how many of each key you hold (as the cards count them): the bars' tiles show it
        let held_n: HashMap<i32, usize> = copies.iter().map(|(id, h)| (*id, h.len())).collect();
        let overhead = 2.0 + 12.0;
        let base = overhead + 20.0 - GAP;
        let (main, side) = widgets::side_split(ui, c.p, c.props, "keys", body, 280.0, 220.0, 200.0, Some((base, CARD_MIN_WIDTH + GAP)));
        let cols = (((main.width() - base + 0.001) / (CARD_MIN_WIDTH + GAP)).floor()).max(1.0) as usize;
        let icons = &mut *c.icons;
        let title = if self.upe_char != 0 {
            app.characters.get(self.upe_char).map_or("Keys".to_string(), |ch| format!("Keys · {}", ch.class_name))
        } else {
            "Keys".to_string()
        };
        widgets::section(
            ui,
            &p,
            main,
            &title,
            fixed::GOLD,
            |_, _| {},
            |ui, r| {
                egui::ScrollArea::vertical().id_salt("keys").show(ui, |ui| {
                    let top = ui.cursor().min;
                    let painter = ui.painter().clone();
                    let x = top.x + 10.0;
                    let full = r.width() - 12.0 - 20.0;
                    let cw = ((full - (cols as f32 - 1.0) * GAP) / cols as f32).floor();
                    let (f18, f16) = (text(18.0), text(16.0));
                    let (lh, lh16) = (jtext::height(&painter, &f18), jtext::height(&painter, &f16));
                    let mut y = top.y + 4.0;
                    if groups.iter().all(|(_, k)| k.is_empty()) {
                        let msg = if app.holdings.known() {
                            "No keys on your characters or in your vaults."
                        } else {
                            "Shown once Loot Gobbler has seen your characters (log in) and your vault."
                        };
                        jtext::draw(&painter, pos2(x, y + 6.0), Align2::LEFT_TOP, msg, &f18, fixed::MUTED);
                    }
                    for (label, keys) in &groups {
                        if keys.is_empty() {
                            continue;
                        }
                        // the group's bar, as a day in Battles: its name in gold, a gold line under it
                        let n: usize = keys.iter().map(|(_, h)| h.len()).sum();
                        y += 6.0;
                        let t = jtext::draw(&painter, pos2(x + 6.0, y), Align2::LEFT_TOP, label, &f18, p.gold);
                        jtext::draw(
                            &painter,
                            pos2(t.right() + 10.0, t.center().y),
                            Align2::LEFT_CENTER,
                            &format!("{n} key{}", if n == 1 { "" } else { "s" }),
                            &f16,
                            fixed::MUTED,
                        );
                        y = t.bottom() + 4.0;
                        painter.rect_filled(Rect::from_min_size(pos2(x + 4.0, y), vec2(full - 8.0, 2.0)), 0.0, p.gold);
                        y += 2.0 + 10.0;
                        // a key's places, the same ones together: "3x Vault · Gift chest"
                        let places = |held: &[lg_app::loadout::Held]| {
                            let mut out: Vec<(String, usize)> = Vec::new();
                            for h in held {
                                let pl = place_of(app, h);
                                match out.iter_mut().find(|(x, _)| *x == pl) {
                                    Some(e) => e.1 += 1,
                                    None => out.push((pl, 1)),
                                }
                            }
                            out
                        };
                        for row in keys.chunks(cols) {
                            let lines = |held: &[lg_app::loadout::Held]| {
                                let n = places(held).len();
                                n.min(PLACES) + usize::from(n > PLACES)
                            };
                            let most = row.iter().map(|(_, h)| lines(h)).max().unwrap_or(0) as f32;
                            let ch = 8.0 + (lh + 2.0 + most * lh16).max(40.0) + 8.0;
                            for (k, (id, held)) in row.iter().enumerate() {
                                let cr = Rect::from_min_size(pos2(x + k as f32 * (cw + GAP), y), vec2(cw, ch));
                                painter.rect_filled(cr, 0.0, p.header);
                                painter.rect_stroke(cr, 0.0, Stroke::new(1.0, Color32::BLACK), StrokeKind::Inside);
                                if let Some(t) = icons.object(ui.ctx(), data, *id, 40) {
                                    painter.image(
                                        t.id(),
                                        Rect::from_min_size(pos2(cr.left() + 8.0, cr.top() + 8.0), vec2(40.0, 40.0)),
                                        uv(),
                                        Color32::WHITE,
                                    );
                                }
                                // the count at the right, the name, and below it where the copies are
                                let count = held.len().to_string();
                                let cf = widgets::head_px(30.0);
                                let cwid = jtext::width(&painter, &count, &cf);
                                jtext::draw(&painter, pos2(cr.right() - 10.0, cr.top() + 8.0), Align2::RIGHT_TOP, &count, &cf, p.gold);
                                let tx = cr.left() + 8.0 + 40.0 + 10.0;
                                let room = cr.right() - 10.0 - cwid - 10.0 - tx;
                                let nm = widgets::fit(&painter, &name(*id), &f18, room);
                                jtext::draw(&painter, pos2(tx, cr.top() + 8.0), Align2::LEFT_TOP, &nm, &f18, p.ink);
                                let mut ly = cr.top() + 8.0 + lh + 2.0;
                                let place_room = room;
                                let pl = places(held);
                                for (place, n) in pl.iter().take(PLACES) {
                                    let s = if *n > 1 { format!("{n}x {place}") } else { place.clone() };
                                    let line = widgets::fit(&painter, &s, &f16, place_room);
                                    jtext::draw(&painter, pos2(tx, ly), Align2::LEFT_TOP, &line, &f16, fixed::MUTED);
                                    ly += lh16;
                                }
                                if pl.len() > PLACES {
                                    let rest: usize = pl.iter().skip(PLACES).map(|(_, n)| n).sum();
                                    let more = format!("and {rest} more elsewhere");
                                    jtext::draw(&painter, pos2(tx, ly), Align2::LEFT_TOP, &more, &f16, fixed::MUTED);
                                }
                                let resp = ui
                                    .interact(cr, ui.scope_id().with(("key-card", *id, *label)), Sense::click())
                                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                                    .on_hover_text(if mystery_keys(data, *id).is_some() {
                                        format!("{}: click for the keys it can give", name(*id))
                                    } else {
                                        format!("{}: click for what it opens", name(*id))
                                    });
                                let on = selected == Some(*id) && selected_seasonal == (*label == "Seasonal");
                                if on || resp.hovered() {
                                    painter.rect_stroke(cr, 0.0, Stroke::new(2.0, p.gold), StrokeKind::Inside);
                                }
                                if resp.clicked() {
                                    let here = *label == "Seasonal";
                                    pick = Some(if on { None } else { Some(*id) });
                                    pick_seasonal = here;
                                }
                            }
                            y += ch + GAP;
                            // a mystery key clicked: a bar under its row with the keys it can give, opening and
                            // closing smoothly (the most open one of the row)
                            let mut bar: Option<(usize, i32, f32)> = None;
                            for (k, (id, _)) in row.iter().enumerate() {
                                if mystery_keys(data, *id).is_none() {
                                    continue;
                                }
                                let open = selected == Some(*id) && selected_seasonal == (*label == "Seasonal");
                                let t = ui.ctx().animate_bool_with_time(ui.scope_id().with(("key-bar", *id, *label)), open, 0.22);
                                if t > 0.0 && bar.is_none_or(|b| t > b.2) {
                                    bar = Some((k, *id, t));
                                }
                            }
                            if let Some((k, id, t)) = bar {
                                let t = t * t * (3.0 - 2.0 * t);
                                let full_h =
                                    key_bar(ui, &p, icons, app, &held_n, id, Rect::from_min_size(pos2(x, y), vec2(full, 0.0)), false);
                                let br = Rect::from_min_size(pos2(x, y), vec2(full, (full_h * t).round()));
                                // the joint up to its card, then the bar framed in gold
                                let card = Rect::from_min_size(pos2(x + k as f32 * (cw + GAP), y - GAP - ch), vec2(cw, ch));
                                painter.rect_filled(
                                    Rect::from_min_max(pos2(card.left(), card.bottom()), pos2(card.right(), br.top() + 2.0)),
                                    0.0,
                                    p.gold,
                                );
                                painter.rect_filled(br, 0.0, p.header);
                                let mut clipped = ui.new_child(egui::UiBuilder::new().max_rect(br));
                                clipped.set_clip_rect(br.intersect(ui.clip_rect()));
                                key_bar(&clipped, &p, icons, app, &held_n, id, br, true);
                                painter.rect_stroke(br, 0.0, Stroke::new(2.0, p.gold), StrokeKind::Inside);
                                y += br.height() + GAP * t;
                                if t < 1.0 {
                                    ui.ctx().request_repaint();
                                }
                            }
                        }
                    }
                    ui.allocate_exact_size(vec2(r.width() - 12.0, y + 10.0 - top.y), Sense::hover());
                });
            },
        );
        if let Some(k) = pick {
            self.key_selected = k;
            self.key_seasonal = pick_seasonal;
        }
        // the side: how many, per side
        let painter = ui.painter().clone();
        let lh = jtext::height(&painter, &text(18.0));
        let head = widgets::section_head_height(&painter);
        let rows: Vec<(String, usize)> = groups
            .iter()
            .flat_map(|(label, keys)| {
                [(format!("{label} keys"), keys.iter().map(|(_, h)| h.len()).sum()), (format!("{label} kinds"), keys.len())]
            })
            .collect();
        let sum = Rect::from_min_size(side.min, vec2(side.width(), head + 2.0 + 16.0 + rows.len() as f32 * (lh + 2.0)));
        widgets::section(
            ui,
            &p,
            sum,
            "Your Keys",
            fixed::GREEN,
            |_, _| {},
            |ui, r| {
                let painter = ui.painter();
                let (x, mut y) = (r.left() + 10.0, r.top() + 8.0);
                for (label, n) in &rows {
                    jtext::draw(painter, pos2(x, y), Align2::LEFT_TOP, label, &text(18.0), p.ink_soft);
                    jtext::draw(painter, pos2(r.right() - 22.0, y), Align2::RIGHT_TOP, &n.to_string(), &text(18.0), p.ink);
                    y += lh + 2.0;
                }
            },
        );
        // the key clicked: what a mystery key can give, or a dungeon key's dungeon
        let dr = Rect::from_min_max(pos2(side.left(), sum.bottom() + 8.0), side.max);
        let icons = &mut *c.icons;
        let now = c.now;
        let chosen = self.key_selected;
        widgets::section(
            ui,
            &p,
            dr,
            "Key Details",
            fixed::BLUE,
            |_, _| {},
            |ui, r| {
                egui::ScrollArea::vertical().id_salt("key-details").show(ui, |ui| {
                    let top = ui.cursor().min;
                    let w = r.width() - 20.0 - 12.0;
                    let h = match chosen {
                        Some(id) => key_details(ui, &p, icons, app, id, now, pos2(top.x + 10.0, top.y + 8.0), w),
                        None => {
                            let painter = ui.painter();
                            jtext::draw(
                                painter,
                                pos2(top.x + 10.0, top.y + 8.0),
                                Align2::LEFT_TOP,
                                "Click a key to see what it opens.",
                                &text(18.0),
                                fixed::MUTED,
                            );
                            jtext::height(painter, &text(18.0))
                        }
                    };
                    ui.allocate_exact_size(vec2(r.width() - 12.0, h + 16.0), Sense::hover());
                });
            },
        );
    }

    fn log(&mut self, ui: &mut Ui, c: &mut Ctx, body: Rect) {
        let p = *c.p;
        let app = c.app;
        let data = app.world.data();
        let o = owned(app, self.upe_char);
        let mut rows: Vec<(i32, String, String, i32)> = o
            .items
            .iter()
            .filter_map(|id| {
                app.collection.entry(*id).map(|e| {
                    (
                        *id,
                        e.last.clone().unwrap_or_else(|| "null".into()),
                        e.first.as_deref().map_or(String::new(), |f| f[..f.len().min(10)].to_string()),
                        e.count,
                    )
                })
            })
            .collect();
        rows.sort_by(|a, b| b.1.cmp(&a.1));
        let icons = &mut *c.icons;
        widgets::section(
            ui,
            &p,
            body,
            "Collection · Log",
            fixed::GOLD,
            |_, _| {},
            |ui, r| {
                let painter = ui.painter().clone();
                let f = text(18.0);
                // the table header, then the rows (36 px, every second one in the header colour)
                let hh = jtext::height(&painter, &small()) + 8.0;
                let head = Rect::from_min_size(r.min, vec2(r.width(), hh));
                painter.rect_filled(head, 0.0, p.header);
                painter.rect_filled(Rect::from_min_size(pos2(head.left(), head.bottom() - 1.0), vec2(head.width(), 1.0)), 0.0, p.line);
                let w = r.width() - 12.0;
                let cols = [(0.0, 120.0, "Date"), (120.0, 44.0, ""), (164.0, w - 164.0 - 80.0, "Item"), (w - 80.0, 80.0, "Count")];
                for (x, cw, t) in cols {
                    jtext::draw(&painter, pos2(r.left() + x + 10.0, head.center().y), Align2::LEFT_CENTER, t, &small(), p.ink_soft);
                    if x > 0.0 {
                        painter.rect_filled(Rect::from_min_size(pos2(r.left() + x, head.top() + 4.0), vec2(1.0, hh - 8.0)), 0.0, p.line);
                    }
                    let _ = cw;
                }
                let rr = Rect::from_min_max(pos2(r.left(), head.bottom()), r.max);
                let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rr));
                child.set_clip_rect(rr.intersect(ui.clip_rect()));
                egui::ScrollArea::vertical().id_salt("collection-log").show(&mut child, |ui| {
                    let top = ui.cursor().min;
                    let painter = ui.painter().clone();
                    for (i, (id, _, first, count)) in rows.iter().enumerate() {
                        let y = top.y + i as f32 * 36.0;
                        let row = Rect::from_min_size(pos2(top.x, y), vec2(w, 36.0));
                        painter.rect_filled(row, 0.0, if i % 2 == 1 { p.header } else { p.paper });
                        jtext::draw(&painter, pos2(row.left() + 10.0, row.center().y), Align2::LEFT_CENTER, first, &f, p.ink);
                        if let Some(t) = icons.object(ui.ctx(), data, *id, ICON as u32) {
                            painter.image(
                                t.id(),
                                Rect::from_min_size(pos2(row.left() + 120.0 + 10.0, row.center().y - ICON / 2.0), vec2(ICON, ICON)),
                                uv(),
                                Color32::WHITE,
                            );
                        }
                        let name = data.items.item(*id).map_or(format!("Item {id}"), |it| it.name.clone());
                        jtext::draw(&painter, pos2(row.left() + 164.0 + 10.0, row.center().y), Align2::LEFT_CENTER, &name, &f, p.ink);
                        jtext::draw(
                            &painter,
                            pos2(row.right() - 10.0, row.center().y),
                            Align2::RIGHT_CENTER,
                            &format!("x{count}"),
                            &f,
                            fixed::GOLD,
                        );
                    }
                    // the table fills the view (Java `setFillsViewportHeight`)
                    let h = (rows.len() as f32 * 36.0).max(rr.height());
                    painter.rect_filled(
                        Rect::from_min_max(pos2(top.x, top.y + rows.len() as f32 * 36.0), pos2(top.x + w, top.y + h)),
                        0.0,
                        p.paper,
                    );
                    ui.allocate_exact_size(vec2(w, rows.len() as f32 * 36.0), Sense::hover());
                });
            },
        );
    }

    /// The ST Sets page (Java `StSetsView`): a card per set with its four pieces, beside the progress, the sets one
    /// piece away and the sets per class.
    /// `themed`: the Themed Sets page instead, built the same (no forge: the pieces are untiered items).
    fn st_sets(&mut self, ui: &mut Ui, c: &mut Ctx, rarity: &Rarity, body: Rect, grid_only: bool, themed: bool) {
        const CARD_MIN_WIDTH: f32 = 330.0;
        const GAP: f32 = 8.0;
        const TILE: f32 = 44.0;
        let p = *c.p;
        let app = c.app;
        let data = app.world.data();
        let o = owned(app, self.upe_char);
        let has = |id: i32| o.set.contains(&id);
        let sets = if themed { &data.themed_sets } else { &data.st_sets };
        let n_of = |s: &lg_gamedata::st_sets::StSet| s.owned(&has);
        let token_of =
            |s: &lg_gamedata::st_sets::StSet| s.pieces.iter().find_map(|id| data.set_tokens.get(id).map(|t| t.token)).unwrap_or(0);
        // the missing pieces the forge can make now from what you hold (tokens, a spare ST to dismantle); with a focus,
        // from what its character can reach: a game mode run only its own things, another character also its side's
        // vault
        let focus = crate::focus::focused_char(app, c.props);
        let run_focus = crate::focus::current(app, c.props).is_some();
        // the forge suggestions: off, as a collector (the best copy of each ST item kept), or with a backup kept too
        use lg_app::st_forge::{FREED_KEY, Mode};
        let mode = Mode::of(c.props.get(Mode::KEY));
        let allowed = lg_app::st_forge::rarities(c.props.get(lg_app::st_forge::RARITIES_KEY));
        let mut next_rarities: Option<[bool; 5]> = None;
        let freed: std::collections::HashSet<i32> =
            c.props.get(FREED_KEY).map(|v| v.split(',').filter_map(|x| x.trim().parse().ok()).collect()).unwrap_or_default();
        let stock = match focus {
            Some(ch) if run_focus => lg_app::st_forge::Stock::of(app, ch.char_id),
            Some(ch) => lg_app::st_forge::Stock::reachable(app, ch.char_id, ch.seasonal),
            None => lg_app::st_forge::Stock::of(app, self.upe_char),
        }
        .keeping(mode, freed.clone())
        .with_rarities(allowed);
        // crafts planned: what they take is not offered again
        let taken: Vec<(i32, lg_app::loadout::Held)> = self.st_planned.iter().flat_map(|pl| pl.taken.iter().cloned()).collect();
        let stock = stock.without(&taken);
        let planned_now = self.st_planned.clone();
        let mut plan_click: Option<Result<Planned, (i32, bool)>> = None;
        let craftable: std::collections::HashSet<i32> =
            sets.iter().flat_map(|s| stock.plan(data, s, &has)).filter(|c| c.craftable).map(|c| c.piece).collect();
        let picked = (self.upe_char != 0).then(|| app.characters.get(self.upe_char)).flatten();
        let class_only: Option<String> = focus.or(picked).map(|ch| ch.class_name.clone());
        let mut shown: Vec<&lg_gamedata::st_sets::StSet> = sets
            .iter()
            .filter(|s| {
                // Tokens: only the sets the forge crafts from a token
                let ok = themed || self.st_filter != 1 || token_of(s) != 0;
                // a character in focus or picked in the column: its class's sets only
                let focus_ok = class_only.as_deref().is_none_or(|k| s.class_name == k);
                ok && focus_ok && self.st_class.as_ref().is_none_or(|k| *k == s.class_name)
            })
            .collect();
        shown.sort_by(|a, b| n_of(b).cmp(&n_of(a)).then(a.title.cmp(&b.title)));
        // a tour's set: picked from what this page counts and shows
        if !themed && std::mem::take(&mut self.tour_pick) {
            let pick = shown
                .iter()
                .find(|s| s.pieces.iter().any(|p| craftable.contains(p)))
                .or_else(|| shown.iter().find(|s| stock.plan(data, s, &has).iter().any(|c| !c.tokens.is_empty())))
                .or_else(|| shown.iter().find(|s| !stock.plan(data, s, &has).is_empty()));
            if let Some(s) = pick {
                self.st_selected = Some(s.name.clone());
                self.st_piece = None;
            }
        }
        // low-high-low, once every 2.8 s
        let time = ui.input(|i| i.time);
        let blink = (0.5 - 0.5 * (std::f64::consts::TAU * time / 2.8).cos()) as f32;
        // the suggestions off: nothing blinks
        let craftable = if mode == Mode::Off { std::collections::HashSet::new() } else { craftable };
        if !craftable.is_empty() {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(33));
        }
        let selected_name = self.st_selected.clone();
        let mut pick: Option<Option<String>> = None;
        // the split: the side 280 px, the cards snapped down to whole columns
        let overhead = 2.0 + 12.0;
        let base = overhead + 20.0 - GAP;
        let (main, side) = if grid_only {
            (body, Rect::NOTHING)
        } else {
            widgets::side_split(ui, c.p, c.props, "stSets", body, 280.0, 220.0, 200.0, Some((base, CARD_MIN_WIDTH + GAP)))
        };
        // parting on to here: the page as it will stand, slid along with the moving edge (its column comes in last)
        let (main, side) = match self.reveal.filter(|_| !grid_only) {
            Some(edge) => {
                let by = vec2(edge - body.left(), 0.0);
                (main.translate(by), side.translate(by))
            }
            None => (main, side),
        };
        let cols = (((main.width() - base + 0.001) / (CARD_MIN_WIDTH + GAP)).floor()).max(1.0);
        let filters = ["All", "Tokens"];
        let mut filter = self.st_filter.min(1);
        let mut next_mode: Option<Mode> = None;
        let mut switch_to: Option<bool> = None;
        let mut freed_click: Option<i32> = None;
        let mut piece_pick: Option<Option<i32>> = None;
        let st_piece = self.st_piece;
        // the bars: the set clicked opening, any other closing
        let dt = ui.input(|i| i.stable_dt).min(0.1);
        if let Some(n) = &self.st_selected {
            self.st_bar_t.entry(n.clone()).or_insert(0.0);
        }
        for (n, t) in self.st_bar_t.iter_mut() {
            let open = self.st_selected.as_deref() == Some(n.as_str());
            *t = if open { (*t + dt / 0.22).min(1.0) } else { (*t - dt / 0.22).max(0.0) };
        }
        self.st_bar_t.retain(|n, t| *t > 0.0 || self.st_selected.as_deref() == Some(n.as_str()));
        if self.st_bar_t.values().any(|t| *t > 0.0 && *t < 1.0) {
            ui.ctx().request_repaint();
        }
        // the item shown: fading in, or out once let go (then the last one)
        self.st_show_t = if self.st_show.is_some() { (self.st_show_t + dt / 0.18).min(1.0) } else { (self.st_show_t - dt / 0.18).max(0.0) };
        if self.st_show.is_some() {
            self.st_show_last = self.st_show.clone();
        }
        if self.st_show_t > 0.0 && self.st_show_t < 1.0 {
            ui.ctx().request_repaint();
        }
        let show_now = self.st_show.clone().or_else(|| self.st_show_last.clone().filter(|_| self.st_show_t > 0.0));
        let show_t = self.st_show_t;
        let show_off = self.st_show_off;
        let mut show_drag = vec2(0.0, 0.0);
        let mut show_pick: Option<(i32, usize, lg_app::loadout::Held)> = None;
        let mut show_card: Option<Rect> = None;
        let bars: Vec<(String, f32)> = self.st_bar_t.iter().map(|(n, t)| (n.clone(), t * t * (3.0 - 2.0 * t))).collect();
        let icons = &mut *c.icons;
        widgets::section(
            ui,
            &p,
            main,
            &format!("{} ·", focus.map_or("Collection", |ch| ch.class_name.as_str())),
            fixed::GOLD,
            |ui, head| {
                let painter = ui.painter().clone();
                let h = head.height();
                // after the title, which sets: ST or themed
                let title_w = jtext::width(
                    &painter,
                    &format!("{} ·", focus.map_or("Collection", |ch| ch.class_name.as_str())).to_uppercase(),
                    &crate::theme::head(17.0),
                );
                let kinds = ["ST", "Themed"];
                let kinds_end = head.left() + 26.0 + title_w + 10.0 + widgets::segmented_width(&painter, &kinds, true);
                if let Some(i) = widgets::segmented(
                    ui,
                    &p,
                    pos2(head.left() + 26.0 + title_w + 10.0, head.top()),
                    h,
                    &kinds,
                    usize::from(themed),
                    true,
                    &[],
                ) {
                    switch_to = Some(i == 1);
                }
                // at the right: All / Tokens (the ST sets only), and before them Collector Mode
                let fx = if themed {
                    head.right() + 8.0
                } else {
                    let fw = widgets::segmented_width(&painter, &filters, true);
                    let fx = head.right() - fw;
                    if let Some(i) = widgets::segmented(ui, &p, pos2(fx, head.top()), h, &filters, filter, true, &[]) {
                        filter = i;
                    }
                    fx
                };
                // the forge suggestions: a pill with a switch of three places (a click: the next)
                // narrow: without its label (it shows on hover)
                let room = fx - 8.0 - (kinds_end + 10.0);
                let out = forge_switch(ui, &p, rarity, pos2(fx - 8.0, head.top() + 1.0), h - 2.0, mode, allowed, room);
                if out.mode.is_some() {
                    next_mode = out.mode;
                }
                if out.rarities.is_some() {
                    next_rarities = out.rarities;
                }
            },
            |ui, r| {
                egui::ScrollArea::vertical().id_salt(if themed { "themed-sets" } else { "st-sets" }).show(ui, |ui| {
                    let top = ui.cursor().min;
                    let painter = ui.painter().clone();
                    let cw = ((r.width() - 12.0 - 20.0 - (cols - 1.0) * GAP) / cols).floor();
                    let lh = jtext::height(&painter, &text(18.0));
                    let ch = 6.0 + 72.0 + 2.0 + lh + 2.0 + 6.0;
                    if shown.is_empty() {
                        jtext::draw(
                            &painter,
                            top + vec2(10.0, 10.0),
                            Align2::LEFT_TOP,
                            if sets.is_empty() { "Loading sets..." } else { "No sets match." },
                            &text(18.0),
                            fixed::MUTED,
                        );
                    }
                    // the bars under the sets: as tall as a card and some, the one clicked opening, the one before closing;
                    // per row only the most open one
                    let mut open_bars: Vec<(f32, usize, f32, f32)> = Vec::new();
                    for (name, t) in &bars {
                        if *t <= 0.0 {
                            continue;
                        }
                        let Some(i) = shown.iter().position(|s| s.name == *name) else { continue };
                        let row = (i as f32 / cols).floor();
                        let flow = mode != Mode::Off && !stock.plan(data, shown[i], &has).is_empty();
                        let plan_i = if mode == Mode::Off { Vec::new() } else { stock.plan(data, shown[i], &has) };
                        let full_h = bar_body_h(app, &plan_i, shown[i], st_piece, &painter) + if flow { FLOW_H } else { 0.0 };
                        match open_bars.iter_mut().find(|b| b.0 == row) {
                            Some(b) if b.2 >= *t => {}
                            Some(b) => *b = (row, i, *t, full_h),
                            None => open_bars.push((row, i, *t, full_h)),
                        }
                    }
                    let grown = |b: &(f32, usize, f32, f32)| (b.3 * b.2).round() + GAP * b.2;
                    let shift = |row: f32| open_bars.iter().filter(|b| b.0 < row).map(grown).sum::<f32>();
                    let extra: f32 = open_bars.iter().map(grown).sum();
                    // a click on the grid's blank space closes it
                    let rows_n = (shown.len() as f32 / cols).ceil();
                    let blank = Rect::from_min_size(top, vec2(r.width() - 12.0, 20.0 + rows_n * (ch + GAP) + extra));
                    if ui.interact(blank, ui.scope_id().with("st-blank"), Sense::click()).clicked() && show_now.is_none() {
                        pick = Some(None);
                    }
                    let mut bar_cards: HashMap<usize, Rect> = HashMap::new();
                    for (i, s) in shown.iter().enumerate() {
                        let (col, row) = (i as f32 % cols, (i as f32 / cols).floor());
                        let cr = Rect::from_min_size(
                            pos2(top.x + 10.0 + col * (cw + GAP), top.y + 10.0 + row * (ch + GAP) + shift(row)),
                            vec2(cw, ch),
                        );
                        if open_bars.iter().any(|b| b.1 == i) {
                            bar_cards.insert(i, cr);
                        }
                        let n = n_of(s);
                        let done = n == 4;
                        painter.rect_filled(cr, 0.0, if done { mix(p.gold, p.paper, 0.45) } else { p.header });
                        painter.rect_stroke(
                            cr,
                            0.0,
                            Stroke::new(if done { 2.0 } else { 1.0 }, if done { p.gold } else { Color32::BLACK }),
                            StrokeKind::Inside,
                        );
                        // a click on the card: the side shows what forging its missing pieces takes (again: hidden)
                        let selected = selected_name.as_deref() == Some(s.name.as_str());
                        if ui
                            .interact(cr, ui.scope_id().with(("st-card", &s.name)), Sense::click())
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .clicked()
                        {
                            pick = Some(if selected { None } else { Some(s.name.clone()) });
                            piece_pick = Some(None);
                        }
                        if s.skin != 0
                            && let Some(t) = icons.object(ui.ctx(), data, s.skin, 64)
                        {
                            painter.image(
                                t.id(),
                                Rect::from_center_size(pos2(cr.left() + 6.0 + 36.0, cr.top() + 6.0 + 36.0), vec2(64.0, 64.0)),
                                uv(),
                                Color32::WHITE,
                            );
                        }
                        // the four pieces beside the skin
                        let tx = cr.left() + 6.0 + 72.0 + 10.0;
                        let ty = cr.top() + 6.0 + (72.0 - TILE) / 2.0;
                        for (k, id) in s.pieces.iter().enumerate() {
                            let tr = Rect::from_min_size(pos2(tx + k as f32 * (TILE + 6.0), ty), vec2(TILE, TILE));
                            let have = has(*id);
                            // a missing piece whose craft is planned: a soft green under it
                            let planned_here = !have && planned_now.iter().any(|pl| pl.piece == *id);
                            painter.rect_filled(
                                tr,
                                0.0,
                                if have {
                                    mix(p.gold, Color32::from_rgb(0xFF, 0xF6, 0xC0), 0.7)
                                } else if planned_here {
                                    mix(fixed::GREEN, Color32::from_rgb(5, 5, 5), 0.35)
                                } else {
                                    Color32::from_rgb(5, 5, 5)
                                },
                            );
                            if let Some(t) = icons.object(ui.ctx(), data, *id, (TILE - 10.0) as u32) {
                                painter.image(
                                    t.id(),
                                    Rect::from_center_size(tr.center(), vec2(TILE - 10.0, TILE - 10.0)),
                                    uv(),
                                    if have { Color32::WHITE } else { Color32::from_white_alpha(64) },
                                );
                            }
                            if have && app.collection.entry(*id).is_some() {
                                enchant_badge(
                                    &painter,
                                    rarity,
                                    pos2(tr.right() - 3.0, tr.bottom() - 3.0),
                                    o.best.get(id).copied().unwrap_or(0).max(0) as usize,
                                    5.0,
                                );
                            }
                            painter.rect_stroke(tr, 0.0, Stroke::new(1.0, Color32::BLACK), StrokeKind::Inside);
                            // a missing piece you can forge now: a gold glow fading up and down
                            let forge = !have && craftable.contains(id);
                            if forge {
                                painter.rect_filled(tr, 0.0, p.gold.gamma_multiply(0.15 + 0.45 * blink));
                                if let Some(t) = icons.object(ui.ctx(), data, *id, (TILE - 10.0) as u32) {
                                    painter.image(
                                        t.id(),
                                        Rect::from_center_size(tr.center(), vec2(TILE - 10.0, TILE - 10.0)),
                                        uv(),
                                        Color32::from_white_alpha((64.0 + 191.0 * blink) as u8),
                                    );
                                }
                                painter.rect_stroke(
                                    tr,
                                    0.0,
                                    Stroke::new(2.0, p.gold.gamma_multiply(0.3 + 0.7 * blink)),
                                    StrokeKind::Inside,
                                );
                            }
                            // hover: a gold frame; click: the piece on the Items page with its history (Java `Tile`), or
                            // for a piece you can forge, what it takes and where that is (the side)
                            let item = data.items.item(*id);
                            let tip = format!(
                                "{}  ({}{})",
                                item.map_or(format!("Item {id}"), |i| i.name.clone()),
                                item.map_or("", |i| i.slot_name()),
                                if have {
                                    ""
                                } else if forge {
                                    " - missing, you can forge it now"
                                } else {
                                    " - missing"
                                }
                            );
                            let resp = ui
                                .interact(tr, ui.scope_id().with(("st-piece", *id, k)), Sense::click())
                                .on_hover_cursor(egui::CursorIcon::PointingHand)
                                .on_hover_text(tip);
                            if resp.hovered() {
                                painter.rect_stroke(tr, 0.0, Stroke::new(2.0, p.gold), StrokeKind::Inside);
                            }
                            // the piece clicked (shown alone in the bar): framed
                            if selected && st_piece == Some(*id) {
                                painter.rect_stroke(tr.expand(2.0), 0.0, Stroke::new(3.0, p.gold_hover), StrokeKind::Inside);
                            }
                            // click: the set's bar with this piece alone (again: all four)
                            if resp.clicked() {
                                pick = Some(Some(s.name.clone()));
                                piece_pick = Some(if selected && st_piece == Some(*id) { None } else { Some(*id) });
                            }
                        }
                        let token = token_of(s);
                        // the set: its name, its bonus, and the token the forge crafts its pieces from
                        let mut tip: Vec<widgets::TipLine> = vec![vec![(s.name.clone(), None, true)], vec![(s.bonus.clone(), None, false)]];
                        if let Some(t) = (token != 0).then(|| data.items.item(token)).flatten() {
                            tip.push(vec![(format!("Craftable in the forge with the {}", t.name), Some(fixed::MUTED), false)]);
                        }
                        widgets::rich_tip(ui, cr, ("st-set", &s.name), &tip);
                        if token != 0
                            && let Some(t) = icons.object(ui.ctx(), data, token, 30)
                        {
                            painter.image(
                                t.id(),
                                Rect::from_min_size(pos2(cr.right() - 30.0 - 10.0, cr.top() + 8.0), vec2(30.0, 30.0)),
                                uv(),
                                Color32::WHITE,
                            );
                        }
                        // along the bottom: the name (underlined, smaller to fit), the class and count
                        let by = cr.bottom() - 6.0 - lh / 2.0 - 1.0;
                        let cls = format!("{}{}", s.class_name, if done { String::new() } else { format!("   {n} / 4") });
                        let cls_w = 8.0 + 6.0 + jtext::width(&painter, &cls, &text(18.0));
                        let cx = cr.right() - 10.0 - cls_w;
                        painter.rect_filled(
                            Rect::from_min_size(pos2(cx, by - 4.0), vec2(8.0, 8.0)),
                            0.0,
                            crate::classes::class_color(&s.class_name),
                        );
                        jtext::draw(&painter, pos2(cx + 14.0, by), Align2::LEFT_CENTER, &cls, &text(18.0), p.ink);
                        let room = cx - 10.0 - (cr.left() + 6.0);
                        let mut size = 18.0;
                        while jtext::width(&painter, &s.title, &text(size)) > room && size > 12.0 {
                            size -= 1.0;
                        }
                        let t = widgets::fit(&painter, &s.title, &text(size), room);
                        let tr = jtext::draw(&painter, pos2(cr.left() + 6.0, by), Align2::LEFT_CENTER, &t, &text(size), p.ink);
                        painter.rect_filled(Rect::from_min_size(pos2(tr.left(), tr.bottom() - 1.0), vec2(tr.width(), 1.0)), 0.0, p.ink);
                        if selected {
                            painter.rect_stroke(cr, 0.0, Stroke::new(3.0, p.gold_hover), StrokeKind::Inside);
                        }
                    }
                    for b in &open_bars {
                        let (row, i, t, full_h) = *b;
                        let Some(card) = bar_cards.get(&i).copied() else { continue };
                        let bar = Rect::from_min_size(
                            pos2(top.x + 10.0, card.bottom() + GAP * t),
                            vec2(cols * (cw + GAP) - GAP, (full_h * t).round()),
                        );
                        // the joint to the card: sliding over when another set of the row is clicked (the bar open)
                        let jx = ui.ctx().animate_value_with_time(
                            ui.scope_id().with(("st-joint", row as i32)),
                            card.left(),
                            if t >= 1.0 { 0.22 } else { 0.0 },
                        );
                        let done = n_of(shown[i]) == 4;
                        let card_fill = if done { mix(p.gold, p.paper, 0.45) } else { p.header };
                        let look = BarLook { piece: st_piece, card, joint_x: jx, bar, full_h, t, mode, card_fill };
                        let show = show_now.as_ref().map(|s| (s, show_t, show_off));
                        let out = st_bar(ui, &p, icons, app, rarity, shown[i], &stock, &has, look, show, &planned_now);
                        if out.plan.is_some() {
                            plan_click = out.plan;
                        }
                        if let Some(id) = out.toggled {
                            freed_click = Some(id);
                        }
                        if out.show.is_some() {
                            show_pick = out.show;
                        }
                        if out.card.is_some() {
                            show_card = out.card;
                        }
                        show_drag += out.drag;
                    }
                    let rows = (shown.len() as f32 / cols).ceil();
                    ui.allocate_exact_size(vec2(r.width() - 12.0, 20.0 + rows * (ch + GAP) + extra), Sense::hover());
                });
            },
        );
        if !themed {
            self.st_filter = filter;
        }
        if let Some(t) = switch_to
            && t != self.sets_themed
        {
            self.sets_themed = t;
            self.st_selected = None;
            self.st_piece = None;
            self.st_show = None;
        }
        // a craft planned, or no longer
        match plan_click {
            Some(Ok(pl)) => self.st_planned.push(pl),
            Some(Err((piece, seasonal))) => self.st_planned.retain(|pl| !(pl.piece == piece && pl.seasonal == seasonal)),
            None => {}
        }
        // an item clicked in the bar: shown; a click anywhere else (not on what is shown) lets it go
        self.st_show_off += show_drag;
        if show_pick.is_some() {
            self.st_show = show_pick;
            self.st_show_t = self.st_show_t.min(0.0);
            self.st_show_off = vec2(0.0, 0.0);
        } else if self.st_show.is_some()
            && ui.input(|i| i.pointer.primary_clicked())
            && !ui.input(|i| i.pointer.interact_pos()).is_some_and(|q| show_card.is_some_and(|c| c.contains(q)))
        {
            self.st_show = None;
        }
        if pick.is_some() {
            self.st_show = None;
        }
        // another set of the same row: its bar stays open (only the joint slides over)
        if let (Some(Some(new)), Some(old)) = (&pick, &self.st_selected)
            && new != old
            && let (Some(a), Some(b)) = (shown.iter().position(|s| s.name == *old), shown.iter().position(|s| s.name == *new))
            && (a as f32 / cols).floor() == (b as f32 / cols).floor()
            && let Some(t) = self.st_bar_t.remove(old)
        {
            self.st_bar_t.insert(new.clone(), t);
        }
        if let Some(m) = next_mode {
            c.asks.push(crate::window::Ask::Set(Mode::KEY.into(), m.value().into()));
        }
        if let Some(r) = next_rarities {
            c.asks.push(crate::window::Ask::Set(lg_app::st_forge::RARITIES_KEY.into(), lg_app::st_forge::rarities_value(r)));
        }
        // a backup's lock clicked: let go (it can be dismantled) or kept again
        if let Some(id) = freed_click {
            let mut f = freed.clone();
            if !f.remove(&id) {
                f.insert(id);
            }
            let mut ids: Vec<i32> = f.into_iter().collect();
            ids.sort_unstable();
            let v = ids.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(",");
            c.asks.push(crate::window::Ask::Set(FREED_KEY.into(), v));
        }
        if let Some(pp) = piece_pick {
            self.st_piece = pp;
            // melted with the items: the piece found in the collection too (scrolled to, pulsing)
            if let Some(id) = pp
                && self.melt
            {
                self.show_item = Some(id);
            }
        }
        if let Some(sel) = pick {
            if sel.is_none() {
                self.st_piece = None;
            }
            self.st_selected = sel;
        }

        if grid_only {
            return;
        }
        // the side: set progress, almost complete, by class
        let painter = ui.painter().clone();
        let lh = jtext::height(&painter, &text(18.0));
        let sh = jtext::height(&painter, &small());
        let head = widgets::section_head_height(&painter);
        let complete = sets.iter().filter(|s| n_of(s) == 4).count();
        let pieces: usize = sets.iter().map(&n_of).sum();
        let prog = Rect::from_min_size(side.min, vec2(side.width(), head + 2.0 + 20.0 + 2.0 * (sh + 10.0 + lh) + 6.0));
        widgets::section(
            ui,
            &p,
            prog,
            "Set Progress",
            fixed::GREEN,
            |_, _| {},
            |ui, r| {
                let painter = ui.painter();
                let (x, w) = (r.left() + 10.0, r.width() - 20.0);
                let mut y = r.top() + 10.0;
                for (k, (cap, have, total, color)) in
                    [("SETS COMPLETE", complete, sets.len(), p.gold), ("PIECES COLLECTED", pieces, sets.len() * 4, fixed::GREEN)]
                        .iter()
                        .enumerate()
                {
                    if k > 0 {
                        y += 6.0;
                    }
                    let t = jtext::draw(painter, pos2(x, y), Align2::LEFT_TOP, cap, &small(), p.ink_muted);
                    y = t.bottom();
                    let pct = if *total == 0 { 0.0 } else { 100.0 * *have as f64 / *total as f64 };
                    painter.rect_filled(Rect::from_min_size(pos2(x, y), vec2(w, 10.0)), 0.0, p.line);
                    painter.rect_filled(Rect::from_min_size(pos2(x, y), vec2((w * pct as f32 / 100.0).round(), 10.0)), 0.0, *color);
                    y += 10.0;
                    let t =
                        jtext::draw(painter, pos2(x, y), Align2::LEFT_TOP, &format!("{have} / {total}   {pct:.1}%"), &text(18.0), p.ink);
                    y = t.bottom();
                }
            },
        );
        // the set clicked: what forging its missing pieces takes, and where those things are
        let chosen = self.st_selected.as_deref().and_then(|n| sets.iter().find(|s| s.name == n));
        let mut below = prog.bottom();
        if let Some(set) = chosen.filter(|_| !themed) {
            let plan = stock.plan(data, set, &has);
            let w = side.width() - 20.0 - 12.0;
            let content = forge_rows(ui, &p, &mut *c.icons, app, rarity, set, &plan, n_of(set), pos2(0.0, 0.0), w, false);
            let room = (side.bottom() - below - 8.0 - 140.0).max(head + 60.0);
            let fr = Rect::from_min_size(pos2(side.left(), below + 8.0), vec2(side.width(), (head + 2.0 + 16.0 + content).min(room)));
            let icons = &mut *c.icons;
            let mut close = false;
            widgets::section(
                ui,
                &p,
                fr,
                "Forge",
                fixed::GOLD,
                |ui, head| {
                    let painter = ui.painter().clone();
                    let xr = Rect::from_center_size(pos2(head.right() - 14.0, head.center().y), vec2(22.0, 22.0));
                    let resp = ui.interact(xr, ui.scope_id().with("st-forge-close"), Sense::click()).on_hover_text("Close");
                    let ink = if resp.hovered() { p.gold } else { p.ink_muted };
                    for (a, b) in [(vec2(-5.0, -5.0), vec2(5.0, 5.0)), (vec2(-5.0, 5.0), vec2(5.0, -5.0))] {
                        painter.line_segment([xr.center() + a, xr.center() + b], Stroke::new(2.0, ink));
                    }
                    close = resp.clicked();
                },
                |ui, r| {
                    egui::ScrollArea::vertical().id_salt("st-forge").show(ui, |ui| {
                        let top = ui.cursor().min;
                        let h = forge_rows(ui, &p, icons, app, rarity, set, &plan, n_of(set), pos2(top.x + 10.0, top.y + 8.0), w, true);
                        ui.allocate_exact_size(vec2(r.width() - 12.0, h + 16.0), Sense::hover());
                    });
                },
            );
            if close {
                self.st_selected = None;
            }
            below = fr.bottom();
        }
        let by_class = Rect::from_min_max(pos2(side.left(), below + 8.0), side.max);
        widgets::section(
            ui,
            &p,
            by_class,
            "By Class",
            fixed::PURPLE,
            |_, _| {},
            |ui, r| {
                let painter = ui.painter();
                let mut per: std::collections::BTreeMap<&str, (usize, usize)> = std::collections::BTreeMap::new();
                for s in sets {
                    let v = per.entry(s.class_name.as_str()).or_default();
                    v.1 += 1;
                    if n_of(s) == 4 {
                        v.0 += 1;
                    }
                }
                let (x, w) = (r.left() + 10.0, r.width() - 20.0 - 12.0);
                let mut y = r.top() + 8.0;
                for (cls, (done, total)) in per {
                    painter.rect_filled(
                        Rect::from_min_size(pos2(x, y + lh / 2.0 - 4.0), vec2(8.0, 8.0)),
                        0.0,
                        crate::classes::class_color(cls),
                    );
                    jtext::draw(painter, pos2(x + 16.0, y + lh / 2.0), Align2::LEFT_CENTER, cls, &text(18.0), p.ink);
                    jtext::draw(painter, pos2(x + w, y + lh / 2.0), Align2::RIGHT_CENTER, &format!("{done} / {total}"), &text(18.0), p.ink);
                    y += lh + 2.0;
                    painter.rect_filled(Rect::from_min_size(pos2(x, y), vec2(w, 5.0)), 0.0, p.line);
                    painter.rect_filled(
                        Rect::from_min_size(pos2(x, y), vec2(if total == 0 { 0.0 } else { w * done as f32 / total as f32 }, 5.0)),
                        0.0,
                        p.gold,
                    );
                    y += 5.0 + 5.0;
                }
            },
        );
    }
}

/// The newest copy's enchantment count (Java `recentRow`).
fn newest_enchants(e: &lg_app::collection::Entry) -> i32 {
    for f in e.history.iter().flatten().rev() {
        if f.is_removal() {
            continue;
        }
        return f.enchant_ids.as_ref().map_or(f.enchants.max(0), |v| v.len() as i32);
    }
    0
}

/// "Today", "Yesterday" or "16 Nov" (Java `dayLine`), upper case.
fn day_label(date: &str, now_local: &str) -> String {
    if date.len() < 10 {
        return date.to_uppercase();
    }
    let today = &now_local[..10.min(now_local.len())];
    let days = |d: &str| -> Option<i64> {
        let (y, m, dd) = (d[0..4].parse::<i64>().ok()?, d[5..7].parse::<i64>().ok()?, d[8..10].parse::<i64>().ok()?);
        Some(y * 372 + m * 31 + dd)
    };
    let text = if date == today {
        "Today".to_string()
    } else if days(today).zip(days(date)).is_some_and(|(t, d)| t - d == 1) {
        "Yesterday".to_string()
    } else {
        let m: usize = date[5..7].parse().unwrap_or(1);
        format!("{} {}", date[8..10].trim_start_matches('0'), MONTHS[m.saturating_sub(1).min(11)])
    };
    text.to_uppercase()
}

/// A game mode run's finds as collection entries (Java `gathered`): per item its copies, the most enchantments,
/// the last find's time and date.
fn gathered(app: &lg_app::App, char_id: i32) -> HashMap<i32, (i32, i32, String, String)> {
    let mut out: HashMap<i32, (i32, i32, String, String)> = HashMap::new();
    // a game mode run only when marked as one (the app keeps a record of every character it saw)
    let Some(run) = app.upe.runs.get(&char_id).filter(|r| r.marked) else { return found_by(app, char_id) };
    let data = app.world.data();
    for g in &run.gathered {
        if !data.items.collectible.contains(&g.item) {
            continue;
        }
        let time = app.zone.local_date_time(g.at);
        let e = out.entry(g.item).or_insert((0, 0, String::new(), String::new()));
        e.0 += 1;
        e.1 = e.1.max(g.enchants.as_ref().map_or(0, |v| v.len() as i32));
        e.2 = time[..10.min(time.len())].to_string();
        e.3 = time;
    }
    out
}

/// A character's own finds (a focus character in the column, no game mode run): the drops that came while it was played.
/// A drop belongs to the characters played around then (a stretch of play by its fame samples, a little slack at
/// either end; a lone sample is no play); where two were, to the one with a sample nearest the drop. As `gathered`:
/// item -> (copies, best enchantments, date, time of the newest).
/// Worked out again only when the records it reads changed (it reads every find and every character's play: done each
/// frame, a long-played character's view kept loading).
fn found_by(app: &lg_app::App, char_id: i32) -> HashMap<i32, (i32, i32, String, String)> {
    type Found = HashMap<i32, (i32, i32, String, String)>;
    type Key = (usize, usize, usize, usize);
    type Cache = HashMap<i32, (Key, Found)>;
    thread_local! {
        static CACHE: std::cell::RefCell<Cache> = std::cell::RefCell::new(HashMap::new());
    }
    // what it depends on, counted: the finds, the owners noted, the characters' samples
    let finds: usize = app.collection.all().map(|(_, e)| e.history.as_ref().map_or(0, Vec::len)).sum();
    let samples: usize = app.characters.all().map(|c| c.samples.len()).sum();
    let key = (finds, app.collection.owners.len(), samples, app.characters.all().count());
    if let Some(hit) = CACHE.with(|c| c.borrow().get(&char_id).filter(|(k, _)| *k == key).map(|(_, f)| f.clone())) {
        return hit;
    }
    let found = work_out_found_by(app, char_id);
    CACHE.with(|c| c.borrow_mut().insert(char_id, (key, found.clone())));
    found
}

fn work_out_found_by(app: &lg_app::App, char_id: i32) -> HashMap<i32, (i32, i32, String, String)> {
    const SLACK_MS: i64 = 2 * 60_000;
    let mut out: HashMap<i32, (i32, i32, String, String)> = HashMap::new();
    if app.characters.get(char_id).is_none() {
        return out;
    }
    // every character's stretches of play, and its sample times (sorted) to settle a tie
    let played: Vec<(i32, Vec<lg_app::characters::Session>, Vec<i64>)> = app
        .characters
        .all()
        .map(|c| {
            let sessions: Vec<_> = lg_app::characters::Characters::sessions(c, 0).into_iter().filter(|s| s.end > s.start).collect();
            let mut times: Vec<i64> = c.samples.iter().map(|s| s.t).collect();
            times.sort_unstable();
            (c.char_id, sessions, times)
        })
        .filter(|(_, s, _)| !s.is_empty())
        .collect();
    let nearest = |times: &[i64], t: i64| {
        let i = times.partition_point(|x| *x < t);
        [i.checked_sub(1), Some(i)].into_iter().flatten().filter_map(|k| times.get(k)).map(|x| (x - t).abs()).min().unwrap_or(i64::MAX)
    };
    let data = app.world.data();
    for (id, e) in app.collection.all() {
        if !data.items.collectible.contains(&id) {
            continue;
        }
        for f in e.history.iter().flatten() {
            // found in its own inventory (not dropped): whose it was is written with it
            if f.source != lg_app::collection::DROP {
                if !f.is_removal() && f.seen.as_ref().is_some_and(|w| w.char_id == char_id) {
                    let v = out.entry(id).or_insert((0, 0, String::new(), String::new()));
                    v.0 += 1;
                    v.1 = v.1.max(f.enchants.max(0));
                    if f.time > v.3 {
                        v.2 = f.time[..10.min(f.time.len())].to_string();
                        v.3 = f.time.clone();
                    }
                }
                continue;
            }
            let owner = played
                .iter()
                .filter(|(_, sessions, _)| sessions.iter().any(|s| f.at >= s.start - SLACK_MS && f.at <= s.end + SLACK_MS))
                .min_by_key(|(_, _, times)| nearest(times, f.at))
                .map(|(c, _, _)| *c);
            // whose it was: noted when it came, else the one played around then
            let noted = app.collection.owners.iter().find(|(i, at, _)| *i == id && *at == f.at).map(|o| o.2);
            if noted.or(owner) != Some(char_id) {
                continue;
            }
            let v = out.entry(id).or_insert((0, 0, String::new(), String::new()));
            v.0 += 1;
            v.1 = v.1.max(f.enchants.max(0));
            if f.time > v.3 {
                v.2 = f.time[..10.min(f.time.len())].to_string();
                v.3 = f.time.clone();
            }
        }
    }
    out
}

/// One item of the collection grid.
#[derive(Clone, Copy)]
struct Cell {
    id: i32,
    have: bool,
    best: i32,
    held: i32,
}

/// Java `CollectionGrid.paintCells`: per item its icon (faded when you have none), and for those you have the
/// best copy's enchantment badge and how many you hold (gold pixel digits outlined in black), the grid's corner at
/// `top` (the rows outside the painter's clip are skipped).
#[allow(clippy::too_many_arguments)]
fn paint_cells(
    painter: &egui::Painter,
    ctx: &egui::Context,
    icons: &mut crate::icons::Icons,
    data: &lg_gamedata::GameData,
    rarity: &Rarity,
    cells: &[Cell],
    cols: usize,
    top: egui::Pos2,
) {
    let count_f = text(16.0);
    let clip = painter.clip_rect();
    for (i, c) in cells.iter().enumerate() {
        let (col, row) = ((i % cols) as f32, (i / cols) as f32);
        let (x, y) = (top.x + PAD + col * CELL, top.y + PAD + row * CELL);
        if y > clip.bottom() || y + CELL < clip.top() || x > clip.right() || x + CELL < clip.left() {
            continue;
        }
        if let Some(t) = icons.object(ctx, data, c.id, ICON as u32) {
            let tint = if c.have { Color32::WHITE } else { Color32::from_white_alpha(56) };
            painter.image(
                t.id(),
                Rect::from_min_size(pos2(x + (CELL - ICON) / 2.0, y + (CELL - ICON) / 2.0), vec2(ICON, ICON)),
                uv(),
                tint,
            );
        }
        if c.have {
            enchant_badge(painter, rarity, pos2(x + CELL - 3.0, y + CELL - 3.0), c.best.max(0) as usize, 5.0);
            if c.held > 0 {
                let t = c.held.to_string();
                let asc = (16.0f32 * 1050.0 / 1350.0).ceil();
                let (tx, ty) = (x + 2.0, y + CELL - 3.0 - asc);
                for dx in -1..=1 {
                    for dy in -1..=1 {
                        if dx != 0 || dy != 0 {
                            jtext::draw(painter, pos2(tx + dx as f32, ty + dy as f32), Align2::LEFT_TOP, &t, &count_f, Color32::BLACK);
                        }
                    }
                }
                jtext::draw(painter, pos2(tx, ty), Align2::LEFT_TOP, &t, &count_f, COUNT_GOLD);
            }
        }
    }
}

type Pieces = std::sync::Arc<std::sync::Mutex<Vec<Option<egui::ColorImage>>>>;

/// "Copy as image" under way.
struct Grab {
    cells: Vec<Cell>,
    cols: usize,
    /// The piece drawn now, and for how many frames it has been.
    chunk: usize,
    frames: u8,
    pieces: Pieces,
}

impl InventoryView {
    /// One step of the grab: the next piece of the grid drawn over the window's top left (one pixel each, as Java's
    /// image), grabbed from the frame; all there: put together, copied, said so.
    /// Starts "Copy as image" as the button does (for the app's test hook).
    pub fn copy_image_now(&mut self) {
        self.copy_requested = true;
    }

    pub fn grab_step(&mut self, ui: &Ui, c: &mut Ctx, rarity: &Rarity) {
        let Some(g) = &mut self.grab else { return };
        let ctx = ui.ctx().clone();
        let ppp = ctx.pixels_per_point();
        let rows = g.cells.len().div_ceil(g.cols).max(1);
        // in points (the image comes out at the screen's scale: Java's size on a plain screen)
        let (w_pt, h_pt) = (g.cols as f32 * CELL + 2.0 * PAD, rows as f32 * CELL + 2.0 * PAD);
        let screen = ctx.content_rect();
        // a piece: as many whole rows as the window holds
        let chunk_pt = (((screen.height() - 2.0 * PAD) / CELL).floor().max(1.0)) * CELL;
        let chunks = (h_pt / chunk_pt).ceil() as usize;
        {
            let pieces = g.pieces.lock().expect("pieces");
            if pieces.len() == chunks && pieces.iter().all(Option::is_some) {
                drop(pieces);
                let pieces = std::mem::take(&mut *g.pieces.lock().expect("pieces"));
                let (w, h) = ((w_pt * ppp).round() as usize, (h_pt * ppp).round() as usize);
                let mut px = vec![Color32::TRANSPARENT; w * h];
                let mut y0 = 0;
                for im in pieces.into_iter().flatten() {
                    for y in 0..im.size[1].min(h.saturating_sub(y0)) {
                        for x in 0..im.size[0].min(w) {
                            px[(y0 + y) * w + x] = im.pixels[y * im.size[0] + x];
                        }
                    }
                    y0 += im.size[1];
                }
                ctx.copy_image(egui::ColorImage::new([w, h], px));
                self.grab = None;
                c.asks.push(crate::window::Ask::Confirm {
                    title: "Copy as image".into(),
                    text: "Collection grid copied to the clipboard as an image.".into(),
                    yes: "OK".into(),
                    no: String::new(),
                    then: Box::new(crate::window::Ask::Nothing),
                });
                return;
            }
        }
        if g.chunk >= chunks {
            ctx.request_repaint();
            return;
        }
        // the piece over the window's top left
        let off = g.chunk as f32 * chunk_pt;
        let piece_h = chunk_pt.min(h_pt - off);
        let area = Rect::from_min_size(screen.min, vec2(w_pt, piece_h));
        let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::unique("lg-grid-grab"))).with_clip_rect(area);
        painter.rect_filled(area, 0.0, BG);
        paint_cells(&painter, &ctx, c.icons, c.app.world.data(), rarity, &g.cells, g.cols, screen.min - vec2(0.0, off));
        if g.frames == 0 {
            let (pieces, chunk, n) = (g.pieces.clone(), g.chunk, chunks);
            let c2 = ctx.clone();
            let (wp, hp) = ((w_pt * ppp).round() as usize, (piece_h * ppp).round() as usize);
            let origin = (screen.min.x * ppp, screen.min.y * ppp);
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::ScreenshotCallback::new(move |frame| {
                let (x0, y0) = (origin.0.round() as usize, origin.1.round() as usize);
                let (w, h) = (wp.min(frame.size[0].saturating_sub(x0)), hp.min(frame.size[1].saturating_sub(y0)));
                let mut px = Vec::with_capacity(w * h);
                for y in 0..h {
                    px.extend_from_slice(&frame.pixels[(y0 + y) * frame.size[0] + x0..(y0 + y) * frame.size[0] + x0 + w]);
                }
                let mut p = pieces.lock().expect("pieces");
                if p.len() < n {
                    p.resize(n, None);
                }
                p[chunk] = Some(egui::ColorImage::new([w, h], px));
                c2.request_repaint();
            })));
        }
        g.frames += 1;
        if g.frames >= 2 {
            g.frames = 0;
            g.chunk += 1;
        }
        ctx.request_repaint();
    }
}

/// A forge resource's look: its name, and how it is drawn (an item's sprite, or a gem in its colour).
enum Look {
    /// A sprite of a sheet, and whether an item's green "+" is cut from it.
    Sheet(&'static str, i32, bool),
    Flame,
    Unknown,
}

/// Materials by kind (decoded 2026-11-22; named as the game's Material Cap Extenders, whose sprites show them):
/// 1 Common (silver), 2 Rare (gold), 3 Legendary (red), 4 Mythical (crystal).
fn material(kind: i32) -> (String, Look) {
    match kind {
        1 => ("Common material".into(), Look::Sheet("enchantments8x8", 58, true)),
        2 => ("Rare material".into(), Look::Sheet("enchantments8x8", 59, true)),
        3 => ("Legendary material".into(), Look::Sheet("enchantments8x8", 60, true)),
        4 => ("Mythical material".into(), Look::Sheet("enchantments8x8", 61, true)),
        k => (format!("Material {k}"), Look::Unknown),
    }
}

/// Dust by kind: 1 green, 4 purple (its maximum grew with a Purple Dust Cap Extender), 5 red.
fn dust(kind: i32) -> (String, Look) {
    match kind {
        1 => ("Green dust".into(), Look::Sheet("enchantments16x16", 0, false)),
        4 => ("Purple dust".into(), Look::Sheet("enchantments16x16", 38, false)), // the Purple Dust Charm's sprite
        5 => ("Red dust".into(), Look::Sheet("enchantments16x16", 37, false)),    // kind 5 fell while red dust was spent (2026-11-22)
        k => (format!("Dust kind {k}"), Look::Unknown),
    }
}

/// A material's name (for the game mode run's bank).
pub fn material_name(kind: i32) -> String {
    material(kind).0.to_lowercase()
}

/// A dust kind's short name (for the game mode run's bank).
pub fn dust_name(kind: i32) -> String {
    match kind {
        1 => "green dust".into(),
        4 => "purple dust".into(),
        5 => "red dust".into(),
        k => format!("dust {k}"),
    }
}

/// The Forgefire's flame (the game's, cut from its forge window).
fn forgefire(ctx: &egui::Context) -> egui::TextureHandle {
    let id = egui::Id::unique("lg-forgefire");
    if let Some(t) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(id)) {
        return t;
    }
    let img = image::load_from_memory(include_bytes!("../../../assets/icon/forgefire.png")).map(|i| i.to_rgba8()).unwrap_or_default();
    let t = ctx.load_texture(
        "forgefire",
        egui::ColorImage::from_rgba_unmultiplied([img.width() as usize, img.height() as usize], img.as_raw()),
        egui::TextureOptions::NEAREST,
    );
    ctx.data_mut(|d| d.insert_temp(id, t.clone()));
    t
}

/// The order the game shows them in: materials blue, red, gold, silver; dust green, red, purple.
pub(crate) const MATERIAL_ORDER: [i32; 4] = [4, 3, 2, 1];
pub(crate) const DUST_ORDER: [i32; 3] = [1, 5, 4];

/// Where a kind goes in its order (kinds not in it after those that are).
pub(crate) fn place(order: &[i32], kind: i32) -> (usize, i32) {
    (order.iter().position(|k| *k == kind).unwrap_or(order.len()), kind)
}

/// The forge resources of both sides (as last seen), right to left from `right`, not past `left_limit`. With a
/// focus, only its side (`Some(true)` the seasonal one).
#[allow(clippy::too_many_arguments)]
fn resource_row(
    ui: &Ui,
    p: &Palette,
    icons: &mut crate::icons::Icons,
    app: &lg_app::App,
    right: egui::Pos2,
    h: f32,
    left_limit: f32,
    only_seasonal: Option<bool>,
) {
    let painter = ui.painter().clone();
    let f = text(18.0);
    let small_f = widgets::small();
    const ICON: f32 = 22.0;
    let mut x = right.x - 4.0;
    let cy = right.y + h / 2.0;
    for (label, seasonal, side) in [("Regular", false, app.resources.regular.as_ref()), ("Seasonal", true, app.resources.seasonal.as_ref())]
    {
        let Some(side) = side else { continue };
        if only_seasonal.is_some_and(|s| s != seasonal) {
            continue;
        }
        let mut entries: Vec<(String, Look, i32, Option<i32>)> = Vec::new();
        let mut materials: Vec<_> = side.materials.iter().collect();
        materials.sort_by_key(|(k, _)| place(&MATERIAL_ORDER, **k));
        for (k, (v, max)) in materials {
            let (n, look) = material(*k);
            entries.push((n, look, *v, Some(*max)));
        }
        let mut dusts: Vec<_> = side.dust.iter().collect();
        dusts.sort_by_key(|(k, _)| place(&DUST_ORDER, **k));
        for (k, (v, max)) in dusts {
            if *v == 0 && matches!(k, 2 | 3) {
                continue; // empty kinds the game does not show either
            }
            let (n, look) = dust(*k);
            entries.push((n, look, *v, Some(*max)));
        }
        if let Some(fire) = side.forgefire {
            entries.push(("Forgefire".into(), Look::Flame, fire, None));
        }
        let widths: Vec<f32> = entries.iter().map(|e| ICON + 4.0 + jtext::width(&painter, &num(i64::from(e.2)), &f) + 14.0).collect();
        let label_w = jtext::width(&painter, &label.to_uppercase(), &small_f) + 10.0;
        let total: f32 = widths.iter().sum::<f32>() + label_w;
        if x - total < left_limit {
            break; // no room: the side further left is left out
        }
        for (e, w) in entries.iter().zip(&widths).rev() {
            x -= w;
            let r = Rect::from_min_size(pos2(x, cy - ICON / 2.0), vec2(ICON, ICON));
            match &e.1 {
                Look::Sheet(sheet, index, cut) => {
                    let t = if *cut {
                        icons.sheet_without_plus(ui.ctx(), sheet, *index, ICON as u32)
                    } else {
                        icons.sheet(ui.ctx(), sheet, *index, ICON as u32)
                    };
                    if let Some(t) = t {
                        painter.image(t.id(), r, uv(), Color32::WHITE);
                    }
                }
                Look::Flame => {
                    // the game's own flame (28 x 52): a little taller than the row's icons, it is narrow
                    let t = forgefire(ui.ctx());
                    let fh = ICON + 4.0;
                    painter.image(t.id(), Rect::from_center_size(r.center(), vec2(fh * 28.0 / 52.0, fh)), uv(), Color32::WHITE);
                }
                Look::Unknown => {
                    painter.circle(r.center(), ICON / 2.0 - 3.0, p.header, Stroke::new(1.5, fixed::MUTED));
                    jtext::draw(&painter, r.center(), Align2::CENTER_CENTER, "?", &small_f, fixed::MUTED);
                }
            }
            let full = e.3.is_some_and(|m| m > 0 && e.2 >= m);
            let tr = jtext::draw(
                &painter,
                pos2(r.right() + 4.0, cy),
                Align2::LEFT_CENTER,
                &num(i64::from(e.2)),
                &f,
                if full { fixed::GOLD } else { p.ink },
            );
            let tip = match e.3 {
                Some(m) if m > 0 => format!("{}: {} / {} ({})", e.0, num(i64::from(e.2)), num(i64::from(m)), label.to_lowercase()),
                _ => format!("{}: {} ({})", e.0, num(i64::from(e.2)), label.to_lowercase()),
            };
            widgets::tip(ui, r.union(tr), ("resource", label, e.0.as_str()), &tip);
        }
        x -= label_w;
        jtext::draw(&painter, pos2(x + 4.0, cy), Align2::LEFT_CENTER, &label.to_uppercase(), &small_f, fixed::MUTED);
        x -= 18.0;
    }
}

/// The key rarity label of the dungeon keys a mystery key gives (by its id in the game data), and whether that is
/// only assumed: the game data says only "a random Key".
fn mystery_label(key: &str) -> Option<(&'static str, bool)> {
    match key {
        "Common Mystery Key" => Some(("COMMON_KEY", false)),
        "Rare Mystery Key" => Some(("RARE_KEY", false)),
        "Epic Mystery Key" => Some(("EPIC_KEY", false)),
        "Legendary Mystery Key" => Some(("LEGEND_KEY", false)),
        "Exalted Mystery Key" => Some(("LEGEND_KEY", true)),
        _ => None,
    }
}

/// A mystery key's possible dungeon keys (with their dungeons, by name) and whether that list is assumed (the game
/// data does not say for it); None for any other key.
fn mystery_keys(data: &lg_gamedata::GameData, id: i32) -> Option<(Vec<(i32, &String)>, bool)> {
    let item = data.items.item(id)?;
    if !item.name.to_lowercase().ends_with("mystery key") {
        return None;
    }
    let (label, assumed) = mystery_label(&item.key)?;
    let mut keys: Vec<(i32, &String)> = data
        .key_dungeons
        .iter()
        .filter(|(k, _)| data.items.item(**k).is_some_and(|i| i.labels.contains(label)))
        .map(|(k, d)| (*k, d))
        .collect();
    keys.sort_by(|a, b| a.1.cmp(b.1));
    Some((keys, assumed))
}

/// The bar under a mystery key clicked on the Keys page: the keys it can give as tiles (the key, its dungeon, how many
/// you hold, the dungeon's difficulty), as many across as fit `r`'s width. Drawn when `draw`; gives its full height.
#[allow(clippy::too_many_arguments)]
fn key_bar(
    ui: &Ui,
    p: &Palette,
    icons: &mut crate::icons::Icons,
    app: &lg_app::App,
    held_n: &HashMap<i32, usize>,
    id: i32,
    r: Rect,
    draw: bool,
) -> f32 {
    const TILE_W: f32 = 230.0;
    const TILE_H: f32 = 44.0;
    const GAP: f32 = 6.0;
    let painter = ui.painter().clone();
    let data = app.world.data();
    let Some((keys, assumed)) = mystery_keys(data, id) else { return 0.0 };
    let (f16, fs) = (text(16.0), small());
    let (lh16, lhs) = (jtext::height(&painter, &f16), jtext::height(&painter, &fs));
    let (x, w) = (r.left() + 12.0, r.width() - 24.0);
    let mut y = r.top() + 10.0;
    let head = format!(
        "GIVES ONE OF THESE {} KEYS{}",
        keys.len(),
        if assumed { "  ·  assumed: the same as the Aspirant Exalted Mystery Key (the game data does not say)" } else { "" }
    );
    if draw {
        jtext::draw(&painter, pos2(x, y), Align2::LEFT_TOP, &widgets::fit(&painter, &head, &fs, w), &fs, p.ink_muted);
    }
    y += lhs + 8.0;
    let cols = (((w + GAP) / (TILE_W + GAP)).floor() as usize).max(1);
    let tile_w = ((w - (cols as f32 - 1.0) * GAP) / cols as f32).floor();
    let held = |k: i32| held_n.get(&k).copied().unwrap_or(0);
    for (i, (k, dungeon)) in keys.iter().enumerate() {
        let tr =
            Rect::from_min_size(pos2(x + (i % cols) as f32 * (tile_w + GAP), y + (i / cols) as f32 * (TILE_H + GAP)), vec2(tile_w, TILE_H));
        if !draw {
            continue;
        }
        let n = held(*k);
        painter.rect_filled(tr, 0.0, if n > 0 { mix(p.gold, p.paper, 0.7) } else { p.paper });
        painter.rect_stroke(tr, 0.0, Stroke::new(1.0, if n > 0 { p.gold } else { Color32::BLACK }), StrokeKind::Inside);
        if let Some(t) = icons.object(ui.ctx(), data, *k, 32) {
            painter.image(t.id(), Rect::from_min_size(pos2(tr.left() + 6.0, tr.center().y - 16.0), vec2(32.0, 32.0)), uv(), Color32::WHITE);
        }
        let tx = tr.left() + 6.0 + 32.0 + 8.0;
        let room = tr.right() - 6.0 - tx;
        let top = tr.center().y - lh16;
        jtext::draw(&painter, pos2(tx, top), Align2::LEFT_TOP, &widgets::fit(&painter, dungeon, &f16, room), &f16, p.ink);
        let diff = data.difficulty.get(dungeon.as_str()).map_or("?".to_string(), |v| format!("{v}"));
        let sub = if n > 0 { format!("{n} held  ·  difficulty {diff}") } else { format!("difficulty {diff}") };
        jtext::draw(
            &painter,
            pos2(tx, top + lh16),
            Align2::LEFT_TOP,
            &widgets::fit(&painter, &sub, &f16, room),
            &f16,
            if n > 0 { p.gold } else { fixed::MUTED },
        );
    }
    let rows = keys.len().div_ceil(cols) as f32;
    y += rows * (TILE_H + GAP) - GAP + 12.0;
    y - r.top()
}

/// The Keys page's details for one key, drawn from `at` in width `w`: a mystery key's possible dungeon keys, or a
/// dungeon key's dungeon (difficulty, your runs there, the key's text, the UT and ST items that drop there, the ones
/// you have lit). Gives the height.
#[allow(clippy::too_many_arguments)]
fn key_details(ui: &Ui, p: &Palette, icons: &mut crate::icons::Icons, app: &lg_app::App, id: i32, now: i64, at: egui::Pos2, w: f32) -> f32 {
    let painter = ui.painter().clone();
    let data = app.world.data();
    let (f18, f16, fs) = (text(18.0), text(16.0), small());
    let (lh, lh16, lhs) = (jtext::height(&painter, &f18), jtext::height(&painter, &f16), jtext::height(&painter, &fs));
    let (x, mut y) = (at.x, at.y);
    let Some(item) = data.items.item(id) else { return 0.0 };
    let held = |k: i32| app.collection.entry(k).map_or(0, |e| e.held());
    let line = |y: &mut f32, s: &str, f: &egui::FontId, color: Color32, h: f32| {
        let t = widgets::fit(&painter, s, f, w);
        jtext::draw(&painter, pos2(x, *y), Align2::LEFT_TOP, &t, f, color);
        *y += h;
    };
    let difficulty = |d: &str| data.difficulty.get(d).map_or("?".to_string(), |v| format!("{v}"));
    // a mystery key: the dungeon keys it can give
    if item.name.to_lowercase().ends_with("mystery key") {
        line(&mut y, &item.name, &f18, p.gold, lh + 4.0);
        let Some((label, assumed)) = mystery_label(&item.key) else {
            line(&mut y, "The game data does not say which keys it gives.", &f16, fixed::MUTED, lh16);
            return y - at.y;
        };
        let mut keys: Vec<(i32, &String)> = data
            .key_dungeons
            .iter()
            .filter(|(k, _)| data.items.item(**k).is_some_and(|i| i.labels.contains(label)))
            .map(|(k, d)| (*k, d))
            .collect();
        keys.sort_by(|a, b| a.1.cmp(b.1));
        line(&mut y, &format!("GIVES ONE OF THESE {} KEYS", keys.len()), &fs, p.ink_muted, lhs + 4.0);
        if assumed {
            let g = painter.layout(
                "Assumed: the same keys as the Aspirant Exalted Mystery Key (the game data does not say).".into(),
                f16.clone(),
                fixed::MUTED,
                w,
            );
            let gh = g.size().y;
            painter.galley(pos2(x, y), g, fixed::MUTED);
            y += gh + 4.0;
        }
        for (k, dungeon) in keys {
            let rh = 26f32.max(lh);
            if let Some(t) = icons.object(ui.ctx(), data, k, 24) {
                painter.image(t.id(), Rect::from_min_size(pos2(x, y + (rh - 24.0) / 2.0), vec2(24.0, 24.0)), uv(), Color32::WHITE);
            }
            let n = held(k);
            let right = format!("{}{}", if n > 0 { format!("{n} held  ·  ") } else { String::new() }, difficulty(dungeon));
            let rw = jtext::width(&painter, &right, &f16);
            jtext::draw(&painter, pos2(x + w, y + rh / 2.0), Align2::RIGHT_CENTER, &right, &f16, if n > 0 { p.gold } else { fixed::MUTED });
            let nm = widgets::fit(&painter, dungeon, &f16, w - 32.0 - rw - 8.0);
            jtext::draw(&painter, pos2(x + 32.0, y + rh / 2.0), Align2::LEFT_CENTER, &nm, &f16, p.ink);
            y += rh + 2.0;
        }
        line(&mut y, "Right: how many you hold, and the dungeon's difficulty.", &f16, fixed::MUTED, lh16);
        return y - at.y;
    }
    // a dungeon key: its dungeon
    let Some(dungeon) = data.key_dungeons.get(&id) else {
        line(&mut y, &item.name, &f18, p.gold, lh + 4.0);
        line(&mut y, "No dungeon known for this key.", &f16, fixed::MUTED, lh16);
        return y - at.y;
    };
    if let Some(t) = data.dungeons.portal_by_name.get(dungeon).and_then(|pid| icons.object(ui.ctx(), data, *pid, 40)) {
        painter.image(t.id(), Rect::from_min_size(pos2(x, y), vec2(40.0, 40.0)), uv(), Color32::WHITE);
    }
    let nm = widgets::fit(&painter, dungeon, &f18, w - 48.0);
    jtext::draw(&painter, pos2(x + 48.0, y + 2.0), Align2::LEFT_TOP, &nm, &f18, p.gold);
    jtext::draw(
        &painter,
        pos2(x + 48.0, y + 2.0 + lh),
        Align2::LEFT_TOP,
        &widgets::fit(&painter, &item.name, &f16, w - 48.0),
        &f16,
        fixed::MUTED,
    );
    y += 40.0f32.max(lh + lh16 + 2.0) + 8.0;
    line(&mut y, &format!("Difficulty: {}", difficulty(dungeon)), &f16, p.ink, lh16);
    // your own runs there (where it is found is not in the game's files)
    let runs: Vec<i64> = app.dungeon_visits.visits.iter().filter(|v| v.dungeon == *dungeon).map(|v| v.start).collect();
    let mine = match runs.iter().max() {
        Some(last) => format!("Your runs: {} (last {})", runs.len(), crate::profile::ago(now, *last)),
        None => "Your runs: none yet".to_string(),
    };
    line(&mut y, &mine, &f16, p.ink, lh16 + 4.0);
    let desc = item.description.replace("\\n", "\n");
    let desc = desc.split("\n").next().unwrap_or("").trim().to_string();
    if !desc.is_empty() {
        let g = painter.layout(desc, f16.clone(), fixed::MUTED, w);
        let gh = g.size().y;
        painter.galley(pos2(x, y), g, fixed::MUTED);
        y += gh + 8.0;
    }
    // what drops there: the UT and ST items the game data says come from this dungeon (only some dungeons say), and
    // any you found there yourself; the ones you have lit
    let mut drops: Vec<i32> =
        data.items.collectible.iter().copied().filter(|k| data.items.item(*k).and_then(|i| i.origin()) == Some(dungeon.as_str())).collect();
    for (k, e) in app.collection.all() {
        let found_here =
            e.history.iter().flatten().any(|f| f.source == lg_app::collection::DROP && f.r#where.eq_ignore_ascii_case(dungeon));
        if found_here && !drops.contains(&k) && data.items.collectible.contains(&k) {
            drops.push(k);
        }
    }
    drops.sort_unstable();
    let got = drops.iter().filter(|k| app.collection.entry(**k).is_some()).count();
    line(&mut y, &format!("DROPS HERE: {got} OF {} COLLECTED", drops.len()), &fs, p.ink_muted, lhs + 4.0);
    if drops.is_empty() {
        line(
            &mut y,
            "No UT or ST items known from here yet (the game data names few dungeons' drops; yours fill in).",
            &f16,
            fixed::MUTED,
            lh16,
        );
    }
    const CELL: f32 = 36.0;
    let per = ((w / CELL).floor() as usize).max(1);
    for (k, item_id) in drops.iter().enumerate() {
        let cr = Rect::from_min_size(pos2(x + (k % per) as f32 * CELL, y + (k / per) as f32 * CELL), vec2(CELL - 4.0, CELL - 4.0));
        let have = app.collection.entry(*item_id).is_some();
        painter.rect_filled(cr, 0.0, if have { mix(p.gold, p.paper, 0.6) } else { Color32::from_rgb(5, 5, 5) });
        if let Some(t) = icons.object(ui.ctx(), data, *item_id, 26) {
            painter.image(
                t.id(),
                Rect::from_center_size(cr.center(), vec2(26.0, 26.0)),
                uv(),
                if have { Color32::WHITE } else { Color32::from_white_alpha(64) },
            );
        }
        let nm = data.items.item(*item_id).map_or(String::new(), |i| i.name.clone());
        widgets::tip(ui, cr, ("key-drop", *item_id), &format!("{nm}{}", if have { "" } else { " - not collected yet" }));
    }
    y += drops.len().div_ceil(per) as f32 * CELL;
    y - at.y
}

/// The bar's piece columns: room above the piece, below it to the side's caption, the icons' size, the copies shown
/// per side (a piece clicked: all, scrolled), the room between them.
const BAR_TOP: f32 = 22.0;
const BAR_GAP: f32 = 24.0;
const ICON_B: f32 = 36.0;
const BAR_ROWS: usize = 3;
const BAR_ROW_GAP: f32 = 10.0;

/// How tall the bar's piece columns are for a set: the most rows a side of a piece shows (a missing piece the forge
/// makes: its tokens and the spare), with `one` the piece clicked alone.
fn bar_body_h(
    app: &lg_app::App,
    plan: &[lg_app::st_forge::Piece],
    set: &lg_gamedata::st_sets::StSet,
    one: Option<i32>,
    painter: &egui::Painter,
) -> f32 {
    let lh16 = jtext::height(painter, &text(16.0));
    let living: HashMap<i32, bool> = app.characters.living().iter().map(|l| (l.char_id, l.seasonal)).collect();
    let copies = app.holdings.copies_of(&living, |id| set.pieces.contains(&id));
    // a piece clicked scrolls in its column: the bar keeps its height
    let _ = one;
    let max = BAR_ROWS;
    let mut sides_h: f32 = 0.0;
    for id in set.pieces.iter() {
        let forge = plan.iter().any(|f| f.piece == *id);
        let n = |seasonal: bool| copies.get(id).map_or(0, |c| c.iter().filter(|h| (h.seasonal == Some(true)) == seasonal).count());
        sides_h = sides_h.max(side_h(forge, n(true), max, lh16) + SIDE_GAP + side_h(forge, n(false), max, lh16));
    }
    BAR_TOP + 32.0 + BAR_GAP + sides_h + 26.0
}

/// Room between the seasonal side and the regular one below it.
const SIDE_GAP: f32 = 18.0;

/// How tall one side of a piece is in the bar: its caption, then its tokens and spare (a missing piece the forge makes),
/// "none", or a row per copy (at most `max`, then "+N more").
fn side_h(forge: bool, n: usize, max: usize, lh16: f32) -> f32 {
    let body = if forge {
        lh16 + 6.0 + ICON_B
    } else if n == 0 {
        lh16
    } else {
        n.min(max) as f32 * (ICON_B + BAR_ROW_GAP) + if n > max { lh16 } else { 0.0 }
    };
    lh16 + if forge { 4.0 } else { 12.0 } + body
}

/// An item's icon in `r` with its copy's enchantments (the rarity badge at its corner).
#[allow(clippy::too_many_arguments)]
fn item_icon(
    painter: &egui::Painter,
    icons: &mut crate::icons::Icons,
    ui: &Ui,
    app: &lg_app::App,
    rarity: &Rarity,
    item: i32,
    h: &lg_app::loadout::Held,
    r: Rect,
) {
    // the item a little inside its box, its rarity big at the corner
    let ir = r.shrink(4.0);
    if let Some(tx) = icons.object(ui.ctx(), app.world.data(), item, ir.width() as u32) {
        painter.image(tx.id(), ir, uv(), Color32::WHITE);
    }
    let ench = h.enchant_ids.as_ref().map_or(0, |e| e.iter().filter(|x| **x >= 0).count());
    enchant_badge(painter, rarity, pos2(r.right(), r.bottom()), ench, 7.5);
}

/// Where a copy is, as a look in `r` (the character, or the vault's chest); where it is in words when hovered.
#[allow(clippy::too_many_arguments)]
fn place_look(
    ui: &Ui,
    painter: &egui::Painter,
    icons: &mut crate::icons::Icons,
    app: &lg_app::App,
    h: &lg_app::loadout::Held,
    item: i32,
    r: Rect,
    id: impl std::hash::Hash + std::fmt::Debug,
) {
    let data = app.world.data();
    let look = if h.char_id > 0 {
        app.characters
            .get(h.char_id)
            .and_then(|c| icons.skin(ui.ctx(), data, if c.skin > 0 { c.skin } else { c.class_type }, c.tex1, c.tex2, r.width() as u32))
    } else {
        chest_look(icons, ui.ctx(), data, &h.slot, h.vault.as_deref() == Some(lg_app::holdings::SEASONAL_VAULT), r.width() as u32)
    };
    if let Some(tx) = look {
        painter.image(tx.id(), r, uv(), Color32::WHITE);
    }
    let name = data.items.item(item).map_or(format!("Item {item}"), |i| i.name.clone());
    widgets::tip(ui, r, id, &format!("{name}\n{}", place_of(app, h)));
}

/// How tall the bar's token flow is.
const FLOW_H: f32 = 72.0;

/// Where the bar under a set is and how it shows: the piece clicked (alone), the set's card (joined to it), the bar as
/// far as open, its full height, how far open, the forge mode.
struct BarLook {
    piece: Option<i32>,
    card: Rect,
    /// Where the joint to the card is (its left; sliding to the card when another of the row is clicked).
    joint_x: f32,
    bar: Rect,
    full_h: f32,
    t: f32,
    mode: lg_app::st_forge::Mode,
    /// The card's own colour, fading down into the bar.
    card_fill: Color32,
}

/// Collector Mode (the ST Sets' forge suggestions; the optimization's too), its right end at `right_top`, `h` tall: a
/// pill set into the page (darker, a shadow along its top, a light edge along its bottom) saying "COLLECTOR MODE",
/// with a switch whose knob stands left (off), in the middle (collector, gold) or right (with a backup, green). Only the
/// switch is clicked; the whole pill, hovered, explains the three at once. Gives the next mode when clicked.
fn forge_switch(
    ui: &Ui,
    p: &Palette,
    rarity: &Rarity,
    right_top: egui::Pos2,
    h: f32,
    mode: lg_app::st_forge::Mode,
    allowed: [bool; 5],
    room: f32,
) -> SwitchOut {
    use lg_app::st_forge::Mode;
    let painter = ui.painter().clone();
    let font = small();
    let (track_w, pad, pick_w) = (56.0, 16.0, 40.0);
    // its label only where there is room for it
    let full = pad + jtext::width(&painter, "COLLECTOR MODE", &font) + 14.0 + track_w + 12.0 + pick_w + pad - 6.0;
    let label = if full <= room { "COLLECTOR MODE" } else { "" };
    let tw = if label.is_empty() { -14.0 } else { jtext::width(&painter, label, &font) };
    let w = pad + tw + 14.0 + track_w + 12.0 + pick_w + pad - 6.0;
    let r = Rect::from_min_size(pos2(right_top.x - w, right_top.y), vec2(w, h));
    // the pill, sunk into the page
    let well = mix(p.header, Color32::BLACK, 0.45);
    painter.rect_filled(r, h / 2.0, well);
    painter.rect_filled(Rect::from_min_size(r.min + vec2(h / 2.0, 1.0), vec2(w - h, 2.0)), 1.0, Color32::from_black_alpha(120));
    painter.rect_stroke(r, h / 2.0, Stroke::new(1.0, Color32::from_black_alpha(160)), StrokeKind::Inside);
    painter.line_segment(
        [pos2(r.left() + h / 2.0, r.bottom() - 0.5), pos2(r.right() - h / 2.0, r.bottom() - 0.5)],
        Stroke::new(1.0, Color32::from_rgba_unmultiplied(0xE8, 0xD8, 0xB0, 40)),
    );
    let hover = ui.interact(r, ui.scope_id().with("collector-mode"), Sense::hover());
    let hover = if label.is_empty() { hover.on_hover_text("Collector Mode") } else { hover };
    let lit = hover.contains_pointer();
    jtext::draw(&painter, pos2(r.left() + pad, r.center().y), Align2::LEFT_CENTER, label, &font, if lit { p.ink } else { p.ink_soft });
    // the switch: a dark track (tinted by the mode) and its silver knob sliding to its place; it alone is clicked
    let pick = Rect::from_min_size(pos2(r.right() - pad + 6.0 - pick_w, r.center().y - 15.0), vec2(pick_w, 30.0));
    let track = Rect::from_min_size(pos2(pick.left() - 12.0 - track_w, r.center().y - 9.0), vec2(track_w, 18.0));
    crate::guide::mark(ui, "inventory.collector", r);
    crate::guide::mark(ui, "inventory.rarity", pick);
    let (place, tint) = match mode {
        Mode::Off => (0.0, None),
        Mode::Collector => (0.5, Some(p.gold)),
        Mode::Backup => (1.0, Some(fixed::GREEN)),
    };
    let at = ui.ctx().animate_value_with_time(ui.scope_id().with("forge-switch-knob"), place, 0.18);
    let dark = Color32::from_rgb(0x2B, 0x26, 0x18);
    let fill = tint.map_or(dark, |c| mix(c, dark, 0.35));
    painter.rect_filled(track, 9.0, Color32::from_rgb(0x14, 0x11, 0x0A));
    painter.rect_filled(track.shrink(1.5), 7.5, fill);
    painter.rect_filled(Rect::from_min_size(track.min + vec2(4.0, 2.0), vec2(track_w - 8.0, 3.0)), 1.5, Color32::from_black_alpha(70));
    painter.rect_stroke(track, 9.0, Stroke::new(1.0, Color32::from_rgba_unmultiplied(0xC8, 0xB0, 0x70, 90)), StrokeKind::Inside);
    let kc = pos2(track.left() + 9.0 + at * (track_w - 18.0), track.center().y);
    painter.circle_filled(kc + vec2(0.0, 1.5), 10.5, Color32::from_black_alpha(110));
    painter.circle_filled(kc, 10.5, Color32::from_rgb(0x6E, 0x6E, 0x6E));
    painter.circle_filled(kc, 9.5, Color32::from_rgb(0xC9, 0xC9, 0xC9));
    painter.circle_filled(kc, 7.5, Color32::from_rgb(0xB4, 0xB4, 0xB4));
    painter.circle_stroke(kc, 7.5, Stroke::new(1.0, Color32::from_rgb(0xE6, 0xE6, 0xE6)));
    painter.circle_filled(kc, 5.5, Color32::from_rgb(0xD8, 0xD8, 0xD8));
    painter.circle_filled(kc + vec2(-2.5, -2.5), 2.2, Color32::from_white_alpha(200));
    let click = ui
        .interact(track.expand(3.0), ui.scope_id().with("collector-mode-switch"), Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    // hovered anywhere on the pill (but its rarity button): the three explained, at once
    if lit && !ui.rect_contains_pointer(pick) {
        hover.show_tooltip_ui(|ui| {
            ui.set_max_width(380.0);
            let rows: [(Mode, &str); 3] = [
                (Mode::Off, "Off: every item can be dispersed."),
                (Mode::Collector, "Collector: the best copy is kept."),
                (Mode::Backup, "Collector + Backup: the best copy is kept; the second best copy is marked green but can be used."),
            ];
            for (m, t) in rows {
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::hover());
                    match m {
                        Mode::Off => {
                            ui.painter().circle(r.center(), 6.0, p.paper, Stroke::new(1.0, p.ink_muted));
                        }
                        Mode::Collector => lock(ui.painter(), r.center(), 14.0, p.gold, false),
                        Mode::Backup => lock(ui.painter(), r.center(), 14.0, fixed::GREEN, false),
                    }
                    let color = if m == mode { p.ink } else { p.ink_soft };
                    ui.add(egui::Label::new(egui::RichText::new(t).color(color).strong()).wrap());
                });
            }
        });
    }
    // the rarities considered: a button with the rarity marks; a click opens the five to tick
    let pid = ui.scope_id().with("rarity-pick-open");
    let open = ui.data(|d| d.get_temp::<bool>(pid)).unwrap_or(false);
    let presp = ui.interact(pick, ui.scope_id().with("rarity-pick"), Sense::hover());
    let some_off = allowed.iter().any(|a| !a);
    painter.rect(
        pick,
        5.0,
        if open || presp.hovered() { mix(p.header, p.gold, 0.25) } else { p.header },
        Stroke::new(1.0, if some_off { p.gold } else { Color32::from_black_alpha(160) }),
        StrokeKind::Inside,
    );
    // the rarities as a little stack, bright when considered: legendary and divine at the bottom (each turned 45°
    // counter-clockwise, standing on one line, touching), rare on top where they meet, uncommon on rare's left stone and
    // a box for common on its right one
    let size = |k: usize| rarity[k].as_ref().map_or(vec2(8.0, 8.0), |t| vec2(t.size()[0] as f32, t.size()[1] as f32));
    // a mark's stones fill a diamond in its picture: turned, they stand in a square of its width over the root of two
    let turned = |v: egui::Vec2| v.x.max(v.y) / std::f32::consts::SQRT_2;
    let (l, d, ra, un) = (size(3), size(4), size(2), size(1));
    // heights at scale 1: the bottom row, rare, uncommon; then scaled to fit the button
    let bottom_h = turned(l).max(turned(d));
    let full = bottom_h + ra.y + un.y;
    let k = (pick.height() - 6.0) / full;
    let base = pick.bottom() - 3.0;
    let meet = pick.center().x + (turned(l) - turned(d)) * k / 2.0;
    let mark = |r: usize, center: egui::Pos2, turn: bool| {
        let Some(t) = &rarity[r] else { return };
        let (hw, hh) = (t.size()[0] as f32 * k / 2.0, t.size()[1] as f32 * k / 2.0);
        let tint = if allowed[r] { Color32::WHITE } else { Color32::from_white_alpha(55) };
        let a = if turn { -std::f32::consts::FRAC_PI_4 } else { 0.0 };
        let (sn, cs) = a.sin_cos();
        let rot = |x: f32, y: f32| center + vec2(x * cs - y * sn, x * sn + y * cs);
        let mut mesh = egui::Mesh::with_texture(t.id());
        for (x, y, u, v) in [(-hw, -hh, 0.0, 0.0), (hw, -hh, 1.0, 0.0), (hw, hh, 1.0, 1.0), (-hw, hh, 0.0, 1.0)] {
            mesh.vertices.push(egui::epaint::Vertex { pos: rot(x, y), uv: pos2(u, v), color: tint });
        }
        mesh.add_triangle(0, 1, 2);
        mesh.add_triangle(0, 2, 3);
        painter.add(egui::Shape::mesh(mesh));
    };
    let (lw, dw) = (turned(l) * k, turned(d) * k);
    // their pictures have an empty edge: a little closer, so the stones meet
    // legendary's stones sit high in its picture: a little lower, on the same line as divine's
    mark(3, pos2(meet - lw / 2.0, base - lw / 2.0 + 2.5 * k - 1.0), true);
    mark(4, pos2(meet + dw / 2.0, base - dw / 2.0), true);
    let row_top = base - bottom_h * k;
    let rare_c = pos2(meet, row_top - ra.y * k / 2.0);
    mark(2, rare_c, false);
    let top_y = row_top - ra.y * k - un.y * k / 2.0;
    mark(1, pos2(rare_c.x - ra.x * k / 4.0, top_y), false);
    // common: a stone turned as uncommon's, a faint white
    let cc = pos2(rare_c.x + ra.x * k / 4.0, top_y);
    let (rx, ry) = (un.x * k * 0.27, un.y * k * 0.27);
    let a = if allowed[0] { 1.0 } else { 0.35 };
    painter.add(egui::Shape::convex_polygon(
        vec![cc + vec2(0.0, -ry), cc + vec2(rx, 0.0), cc + vec2(0.0, ry), cc + vec2(-rx, 0.0)],
        Color32::from_white_alpha((70.0 * a) as u8),
        Stroke::new(1.0, Color32::from_white_alpha((150.0 * a) as u8)),
    ));
    // open while the mouse is on the button or on the panel (and the way between them)
    let aid = ui.scope_id().with("rarity-pick-panel");
    let panel_was: Option<Rect> = ui.data(|d| d.get_temp(aid));
    let pointer = ui.input(|i| i.pointer.hover_pos());
    // the button, the panel and the strip between them (not the rest of the box round them: the switch is beside it)
    let near = |q: egui::Pos2, panel: Rect| {
        let bridge = Rect::from_min_max(pos2(pick.left(), pick.bottom() - 2.0), pos2(pick.right(), panel.top() + 2.0));
        pick.expand(2.0).contains(q) || panel.expand(4.0).contains(q) || bridge.contains(q)
    };
    let on_panel = pointer.is_some_and(|q| panel_was.is_some_and(|r| near(q, r)));
    let mut open = presp.hovered() || (open && on_panel);
    let mut next_rarities = None;
    if open {
        let area = egui::Area::new(ui.scope_id().with("rarity-pick-area"))
            .order(egui::Order::Foreground)
            .fixed_pos(pos2(pick.right() - 220.0, pick.bottom() + 6.0))
            .show(ui.ctx(), |ui| {
                egui::Frame::new().fill(p.paper).stroke(Stroke::new(2.0, p.gold)).corner_radius(6.0).inner_margin(10.0).show(ui, |ui| {
                    ui.set_width(200.0);
                    let names = ["Common", "Uncommon", "Rare", "Legendary", "Divine"];
                    let f = text(16.0);
                    jtext::draw(ui.painter(), ui.cursor().min, Align2::LEFT_TOP, "CONSIDERED FOR CRAFTING", &small(), fixed::MUTED);
                    ui.add_space(jtext::height(ui.painter(), &small()) + 6.0);
                    for (k, name) in names.iter().enumerate() {
                        let (row, resp) = ui.allocate_exact_size(vec2(200.0, 26.0), Sense::click());
                        let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
                        let painter = ui.painter();
                        if resp.hovered() {
                            painter.rect_filled(row, 3.0, mix(p.header, p.paper, 0.5));
                        }
                        // the tick box, the rarity's mark, its name
                        let bx = Rect::from_center_size(pos2(row.left() + 10.0, row.center().y), vec2(14.0, 14.0));
                        painter.rect(bx, 2.0, p.header, Stroke::new(1.0, p.line), StrokeKind::Inside);
                        if allowed[k] {
                            painter.line_segment(
                                [bx.left_center() + vec2(3.0, 0.0), bx.center_bottom() + vec2(0.0, -3.0)],
                                Stroke::new(2.0, p.gold),
                            );
                            painter.line_segment(
                                [bx.center_bottom() + vec2(0.0, -3.0), bx.right_top() + vec2(-3.0, 3.0)],
                                Stroke::new(2.0, p.gold),
                            );
                        }
                        // the rarity's mark, all centred in one column; common a faint white stone
                        let mc = pos2(row.left() + 38.0, row.center().y);
                        match &rarity[k] {
                            Some(t) if k > 0 => {
                                let sz = vec2(t.size()[0] as f32, t.size()[1] as f32);
                                painter.image(t.id(), Rect::from_center_size(mc, sz), uv(), Color32::WHITE);
                            }
                            _ => {
                                painter.add(egui::Shape::convex_polygon(
                                    vec![mc + vec2(0.0, -3.5), mc + vec2(3.5, 0.0), mc + vec2(0.0, 3.5), mc + vec2(-3.5, 0.0)],
                                    Color32::from_white_alpha(70),
                                    Stroke::new(1.0, Color32::from_white_alpha(150)),
                                ));
                            }
                        }
                        jtext::draw(
                            painter,
                            pos2(row.left() + 54.0, row.center().y),
                            Align2::LEFT_CENTER,
                            name,
                            &f,
                            if allowed[k] { p.ink } else { p.ink_muted },
                        );
                        if resp.clicked() {
                            let mut a = allowed;
                            a[k] = !a[k];
                            next_rarities = Some(a);
                        }
                    }
                });
            });
        ui.data_mut(|d| d.insert_temp(aid, area.response.rect));
        if !presp.hovered() && !pointer.is_some_and(|q| near(q, area.response.rect)) {
            open = false;
        }
    }
    ui.data_mut(|d| d.insert_temp(pid, open));
    SwitchOut { mode: click.clicked().then(|| mode.next()), rarities: next_rarities }
}

/// What the Collector Mode pill gives back: the next mode (its switch clicked), the rarities ticked (one changed).
struct SwitchOut {
    mode: Option<lg_app::st_forge::Mode>,
    rarities: Option<[bool; 5]>,
}

/// A padlock centred on `c`, `s` tall (open: its shackle lifted on one side).
fn lock(painter: &egui::Painter, c: egui::Pos2, s: f32, color: Color32, open: bool) {
    let body = Rect::from_center_size(c + vec2(0.0, s * 0.18), vec2(s * 0.8, s * 0.6));
    let w = (s * 0.13).max(1.2);
    let (l, r) = (body.left() + s * 0.16, body.right() - s * 0.16);
    let top = body.top() - s * 0.38;
    let lift = if open { s * 0.18 } else { 0.0 };
    painter.line_segment([pos2(l, body.top()), pos2(l, top + s * 0.12 - lift)], Stroke::new(w, color));
    painter.line_segment([pos2(r, body.top() - if open { s * 0.12 } else { 0.0 }), pos2(r, top + s * 0.12 - lift)], Stroke::new(w, color));
    painter.line_segment([pos2(l, top + s * 0.12 - lift), pos2(l + s * 0.1, top - lift)], Stroke::new(w, color));
    painter.line_segment([pos2(r, top + s * 0.12 - lift), pos2(r - s * 0.1, top - lift)], Stroke::new(w, color));
    painter.line_segment([pos2(l + s * 0.1, top - lift), pos2(r - s * 0.1, top - lift)], Stroke::new(w, color));
    painter.rect_filled(body, 1.0, color);
    painter.rect_stroke(body, 1.0, Stroke::new(1.0, Color32::BLACK), StrokeKind::Inside);
}

/// One copy as a chip: its place's look (the character, or the vault's chest) and its enchantments; where it is when
/// hovered.
#[allow(clippy::too_many_arguments)]
fn copy_chip(
    ui: &Ui,
    painter: &egui::Painter,
    p: &Palette,
    icons: &mut crate::icons::Icons,
    app: &lg_app::App,
    rarity: &Rarity,
    h: &lg_app::loadout::Held,
    item: i32,
    cr: Rect,
    id: impl std::hash::Hash + std::fmt::Debug,
) {
    let data = app.world.data();
    let _ = p;
    let look = if h.char_id > 0 {
        app.characters.get(h.char_id).and_then(|c| {
            icons.skin(ui.ctx(), data, if c.skin > 0 { c.skin } else { c.class_type }, c.tex1, c.tex2, (cr.width() - 2.0) as u32)
        })
    } else {
        chest_look(icons, ui.ctx(), data, &h.slot, h.vault.as_deref() == Some(lg_app::holdings::SEASONAL_VAULT), (cr.width() - 2.0) as u32)
    };
    if let Some(tx) = look {
        painter.image(tx.id(), cr.shrink(1.0), uv(), Color32::WHITE);
    }
    let ench = h.enchant_ids.as_ref().map_or(0, |e| e.iter().filter(|x| **x >= 0).count());
    enchant_badge(painter, rarity, pos2(cr.right() - 1.0, cr.bottom() - 1.0), ench, 4.0);
    let name = data.items.item(item).map_or(format!("Item {item}"), |i| i.name.clone());
    let tip = format!(
        "{name}\n{}{}",
        place_of(app, h),
        if ench > 0 { format!("  ·  {ench} enchantment{}", if ench == 1 { "" } else { "s" }) } else { String::new() }
    );
    widgets::tip(ui, cr, id, &tip);
}

/// The bar under a set clicked (ST Sets): per piece where its copies are, the seasonal side left and the regular right,
/// each copy as its place (the character's look, or the vault's chest) with its enchantments; with one piece clicked
/// that piece alone, each copy with where it is in words. With the forge suggestions on, the copy kept has a gold
/// lock, a backup kept a green one (a click lets it go or keeps it again) and the copy free to dismantle a gold box;
/// a missing piece the forge makes shows per side its tokens and the spare ST item it would take (beside that item's
/// copy kept). Below, the token flow: where the set's tokens are and how many pieces each place's make. `look.card`
/// is the set's card, joined to the bar by gold lines. Gives the item whose backup lock was clicked.
#[allow(clippy::too_many_arguments)]
fn st_bar(
    ui: &Ui,
    p: &Palette,
    icons: &mut crate::icons::Icons,
    app: &lg_app::App,
    rarity: &Rarity,
    set: &lg_gamedata::st_sets::StSet,
    stock: &lg_app::st_forge::Stock,
    has: &dyn Fn(i32) -> bool,
    look: BarLook,
    show: Option<(&(i32, usize, lg_app::loadout::Held), f32, egui::Vec2)>,
    planned: &[Planned],
) -> BarOut {
    use lg_app::st_forge::Mode;
    let BarLook { piece, card, joint_x, bar, full_h, t, mode, card_fill } = look;
    // the card gets its gold frame; the joint (and the fade) stand where the joint is
    let real_card = card;
    if t >= 1.0 {
        crate::guide::mark(ui, "inventory.st.bar", real_card.union(bar));
    }
    let card = card.translate(vec2(joint_x - card.left(), 0.0));
    let full = ui.painter().clone();
    let gold = p.gold.gamma_multiply(t);
    // the card and the bar as one: the bar, a joint as wide as the card down to it, gold around both
    full.rect_filled(bar, 0.0, p.paper);
    // the bar framed as the card is (3 px), so the lines meet without a step
    full.rect_stroke(bar, 0.0, Stroke::new(3.0, gold), StrokeKind::Inside);
    // the card's frame as wide as the selected one's (3 px), so the joint's sides run on from it
    full.rect_stroke(real_card, 0.0, Stroke::new(3.0, gold), StrokeKind::Inside);
    // the card's colour running down through the joint and fading out in the bar
    // a set complete fades its gold into the bar; one not complete its card's brown
    let glow = card_fill;
    crate::overlays::gradient_round_rect(
        &full,
        // over the card's bottom border too (the selected one's is 3 px)
        Rect::from_min_max(pos2(card.left() + 3.0, card.bottom() - 4.0), pos2(card.right() - 3.0, bar.top() + 3.0)),
        0.0,
        card_fill,
        glow,
    );
    let fade_end = (bar.top() + 110.0).min(bar.bottom() - 2.0);
    if fade_end > bar.top() + 3.0 {
        // softly out, rounded: each point of a fine grid a mix of the two colours by its distance from the card
        // (an ellipse: as far down as `fade_end`, `spread` out to either side)
        let spread = 110.0;
        let (l, r) = (card.left() + 3.0, card.right() - 3.0);
        let (x0, x1) = ((l - spread).max(bar.left() + 3.0), (r + spread).min(bar.right() - 3.0));
        let depth = fade_end - bar.top() - 2.0;
        let weight = |x: f32, y: f32| {
            let dx = if x < l {
                (l - x) / spread
            } else if x > r {
                (x - r) / spread
            } else {
                0.0
            };
            let dy = (y - bar.top() - 2.0) / depth;
            let d = (dx * dx + dy * dy).sqrt().min(1.0);
            // eased twice: a long, soft tail
            let e = 1.0 - d;
            e * e * (3.0 - 2.0 * e) * e
        };
        let (cols_n, rows_n) = (28usize, 14usize);
        let mut mesh = egui::Mesh::default();
        for k in 0..=rows_n {
            let y = bar.top() + 3.0 + depth * k as f32 / rows_n as f32;
            for i in 0..=cols_n {
                let x = x0 + (x1 - x0) * i as f32 / cols_n as f32;
                mesh.colored_vertex(pos2(x, y), mix(glow, p.paper, f64::from(weight(x, y))));
            }
        }
        let n = cols_n as u32 + 1;
        for k in 0..rows_n as u32 {
            for i in 0..n - 1 {
                let (a, b) = (k * n + i, k * n + i + 1);
                mesh.add_triangle(a, b, a + n);
                mesh.add_triangle(b, b + n, a + n);
            }
        }
        full.add(egui::Shape::mesh(mesh));
    }
    for x in [card.left() + 1.5, card.right() - 1.5] {
        full.line_segment([pos2(x, card.bottom() - 4.0), pos2(x, bar.top() + 3.0)], Stroke::new(3.0, gold));
    }
    let painter = full.with_clip_rect(bar.shrink(2.0).intersect(full.clip_rect()));
    let data = app.world.data();
    let living: HashMap<i32, bool> = app.characters.living().iter().map(|l| (l.char_id, l.seasonal)).collect();
    let copies = app.holdings.copies_of(&living, |id| set.pieces.contains(&id));
    let plan = if mode == Mode::Off { Vec::new() } else { stock.plan(data, set, &|i| has(i)) };
    // a piece clicked: its column shows every copy with where it is in words (scrolled when they do not fit), the
    // others darkened
    let one = piece.filter(|id| set.pieces.contains(id));
    let pieces: Vec<i32> = set.pieces.to_vec();
    let (f18, f16, fs) = (text(18.0), text(16.0), text(14.0));
    let lh16 = jtext::height(&painter, &f16);
    let colw = (bar.width() - 16.0) / pieces.len() as f32;
    let mut toggled = None;
    let mut shown: Option<(i32, usize, lg_app::loadout::Held)> = None;
    let mut plan_click: Option<Result<Planned, (i32, bool)>> = None;
    let same = |a: &lg_app::loadout::Held, b: &lg_app::loadout::Held| {
        a.char_id == b.char_id && a.vault == b.vault && a.slot == b.slot && a.enchant_ids == b.enchant_ids
    };
    for (k, id) in pieces.iter().enumerate() {
        let x0 = bar.left() + 8.0 + k as f32 * colw;
        if k > 0 {
            // down to the token flow's line, when there is one
            let flow_top = if plan.is_empty() { bar.top() + full_h - 8.0 } else { bar.top() + full_h - FLOW_H - 6.0 };
            widgets::pack_divider(&painter, x0 - 4.0, bar.top() + 22.0, flow_top - 14.0, p.gold);
        }
        // where the next divider stands (the bar's edge after the last column): labels stop short of it
        let col_right = x0 + colw - 6.0;
        // the piece, a little away from the divider
        let x0 = x0 + if k > 0 { 12.0 } else { 4.0 };
        let colw = colw - 16.0;
        let y0 = bar.top() + BAR_TOP;
        if let Some(tx) = icons.object(ui.ctx(), data, *id, 32) {
            painter.image(tx.id(), Rect::from_min_size(pos2(x0, y0), vec2(32.0, 32.0)), uv(), Color32::WHITE);
        }
        let name = data.items.item(*id).map_or(format!("Item {id}"), |i| i.name.clone());
        let nt = widgets::fit(&painter, &name, &f18, colw - 42.0);
        jtext::draw(&painter, pos2(x0 + 40.0, y0 + 16.0), Align2::LEFT_CENTER, &nt, &f18, p.ink);
        let cy = y0 + 32.0 + BAR_GAP;
        let half = colw;
        let forge = plan.iter().find(|f| f.piece == *id).filter(|_| !has(*id));
        // the copies kept (by rank: the best, then a backup) and the next one, free to dismantle
        let ranked = stock.ranked(*id);
        let kept = stock.kept(*id);
        let mut used = vec![false; ranked.len()];
        // the seasonal side, then the regular one below it, each the column's width
        let focus = one == Some(*id);
        let max_rows = if focus { usize::MAX } else { BAR_ROWS };
        let n_side = |seasonal: bool| copies.get(id).map_or(0, |c| c.iter().filter(|h| (h.seasonal == Some(true)) == seasonal).count());
        let n_seasonal = n_side(true);
        let seasonal_h = side_h(forge.is_some(), n_seasonal, max_rows, lh16);
        // the column clicked: as tall as it needs, scrolled within the bar by the wheel (no scrollbar)
        let bottom = if plan.is_empty() { bar.top() + full_h - 8.0 } else { bar.top() + full_h - FLOW_H - 6.0 };
        let view = Rect::from_min_max(pos2(x0 - 6.0, cy - 4.0), pos2(x0 + colw + 6.0, bottom));
        let mut off = 0.0f32;
        if focus {
            let content = seasonal_h + SIDE_GAP + side_h(forge.is_some(), n_side(false), max_rows, lh16);
            let most = (content - view.height() + 8.0).max(0.0);
            let key = ui.scope_id().with(("st-col-scroll", &set.name, *id));
            off = ui.data(|d| d.get_temp::<f32>(key)).unwrap_or(0.0);
            if ui.rect_contains_pointer(view) {
                let d = ui.input(|i| i.smooth_scroll_delta.y);
                if d != 0.0 {
                    off -= d;
                    // the wheel is this column's: the page does not scroll with it
                    ui.ctx().input_mut(|i| i.smooth_scroll_delta.y = 0.0);
                }
            }
            off = off.clamp(0.0, most);
            ui.data_mut(|d| d.insert_temp(key, off));
        }
        let painter = if focus { painter.with_clip_rect(view.intersect(painter.clip_rect())) } else { painter.clone() };
        let cy = cy - off;
        for seasonal in [true, false] {
            let sx = x0;
            let top_s = if seasonal { cy } else { cy + seasonal_h + SIDE_GAP };
            jtext::draw(&painter, pos2(sx, top_s), Align2::LEFT_TOP, if seasonal { "SEASONAL" } else { "REGULAR" }, &f16, fixed::MUTED);
            // the token count close under its caption; copies a little further down
            let mut y = top_s + lh16 + if forge.is_some() { 4.0 } else { 12.0 };
            let on_side = |h: &lg_app::loadout::Held| h.seasonal.is_none() || h.seasonal == Some(seasonal);
            if let Some(f) = forge {
                // planned on this side: what it takes, set aside, and "PLANNED" (a click: no longer)
                if let Some(pl) = planned.iter().find(|pl| pl.piece == *id && pl.seasonal == seasonal) {
                    let tt = format!("{} token{} set aside", f.amount, if f.amount == 1 { "" } else { "s" });
                    jtext::draw(&painter, pos2(sx, y), Align2::LEFT_TOP, &tt, &f16, fixed::GREEN);
                    y += lh16 + 6.0;
                    let mut x = sx;
                    if let Some((item, held)) = pl.taken.iter().find(|(i, _)| *i != f.token) {
                        let ir = Rect::from_min_size(pos2(x, y), vec2(ICON_B, ICON_B));
                        item_icon(&painter, icons, ui, app, rarity, *item, held, ir);
                        let pr = Rect::from_min_size(pos2(ir.right() + 6.0, y), vec2(ICON_B, ICON_B));
                        place_look(ui, &painter, icons, app, held, *item, pr, ("st-planned", *id, seasonal));
                        x = pr.right() + 8.0;
                    }
                    if plan_label(ui, &painter, p, pos2(x, y + ICON_B / 2.0), true, col_right, ("st-plan", *id, seasonal)) {
                        plan_click = Some(Err((*id, seasonal)));
                    }
                    continue;
                }
                // missing, the forge makes it: the tokens on this side, and the spare ST item it takes with where it is
                let n = f.tokens.iter().filter(|c| on_side(&c.held)).count() as i32;
                let enough = n >= f.amount;
                // needed / held
                let tt = format!("{} / {n} token{}", f.amount, if n == 1 { "" } else { "s" });
                jtext::draw(&painter, pos2(sx, y), Align2::LEFT_TOP, &tt, &f16, if enough { fixed::GREEN } else { fixed::MUTED });
                y += lh16 + 6.0;
                // what planning it would set aside: the tokens it takes from this side (and the spare)
                let tokens: Vec<(i32, lg_app::loadout::Held)> =
                    f.tokens.iter().filter(|c| on_side(&c.held)).take(f.amount.max(0) as usize).map(|c| (c.item, c.held.clone())).collect();
                match &f.st_slot {
                    None => {
                        jtext::draw(&painter, pos2(sx, y), Align2::LEFT_TOP, "no ST item needed", &fs, fixed::MUTED);
                        if enough && plan_label(ui, &painter, p, pos2(sx, y + lh16 + 14.0), false, col_right, ("st-plan", *id, seasonal)) {
                            plan_click = Some(Ok(Planned { piece: *id, seasonal, taken: tokens }));
                        }
                    }
                    Some(slot) => match f.spares.iter().find(|c| on_side(&c.held)) {
                        None => {
                            // none, or only of rarities left out (the rarity button)
                            let t = if stock.spare_left_out(data, slot, Some(seasonal)) {
                                format!("spare ST {} left out by rarity", slot.to_lowercase())
                            } else {
                                format!("no spare ST {}", slot.to_lowercase())
                            };
                            let t = widgets::fit(&painter, &t, &fs, half - 6.0);
                            jtext::draw(&painter, pos2(sx, y), Align2::LEFT_TOP, &t, &fs, fixed::MUTED);
                        }
                        Some(spare) => {
                            let ir = Rect::from_min_size(pos2(sx, y), vec2(ICON_B, ICON_B));
                            item_icon(&painter, icons, ui, app, rarity, spare.item, &spare.held, ir);
                            // a click: this item shown beside its column
                            if ui
                                .interact(ir, ui.scope_id().with(("st-spare-item", *id, seasonal)), Sense::click())
                                .on_hover_cursor(egui::CursorIcon::PointingHand)
                                .clicked()
                            {
                                shown = Some((spare.item, k, spare.held.clone()));
                            }
                            let pr = Rect::from_min_size(pos2(ir.right() + 6.0, y), vec2(ICON_B, ICON_B));
                            place_look(ui, &painter, icons, app, &spare.held, spare.item, pr, ("st-spare", *id, seasonal));
                            // enough tokens on this side too: it can be made here; a click plans it
                            if enough
                                && plan_label(
                                    ui,
                                    &painter,
                                    p,
                                    pos2(pr.right() + 8.0, pr.center().y),
                                    false,
                                    col_right,
                                    ("st-plan", *id, seasonal),
                                )
                            {
                                let mut taken = tokens;
                                taken.push((spare.item, spare.held.clone()));
                                plan_click = Some(Ok(Planned { piece: *id, seasonal, taken }));
                            }
                        }
                    },
                }
                continue;
            }
            let list: Vec<&lg_app::loadout::Held> =
                copies.get(id).into_iter().flatten().filter(|h| (h.seasonal == Some(true)) == seasonal).collect();
            if list.is_empty() {
                jtext::draw(&painter, pos2(sx, y), Align2::LEFT_TOP, "none", &f16, fixed::MUTED);
                continue;
            }
            // a row per copy: the item with its enchantments, beside it where it is (one piece: also in words)
            let max = max_rows;
            for (n, h) in list.iter().enumerate() {
                if n == max {
                    let more = format!("+{} more", list.len() - max);
                    jtext::draw(&painter, pos2(sx, y), Align2::LEFT_TOP, &more, &fs, fixed::MUTED);
                    break;
                }
                let ir = Rect::from_min_size(pos2(sx, y), vec2(ICON_B, ICON_B));
                item_icon(&painter, icons, ui, app, rarity, *id, h, ir);
                let pr = Rect::from_min_size(pos2(ir.right() + 6.0, y), vec2(ICON_B, ICON_B));
                place_look(ui, &painter, icons, app, h, *id, pr, ("st-copy", *id, h.char_id, &h.slot, n));
                if focus {
                    let tt = widgets::fit(&painter, &place_of(app, h), &fs, half - 2.0 * ICON_B - 18.0);
                    jtext::draw(&painter, pos2(pr.right() + 6.0, pr.center().y), Align2::LEFT_CENTER, &tt, &fs, p.ink_soft);
                }
                y += ICON_B + BAR_ROW_GAP;
                if mode == Mode::Off {
                    continue;
                }
                // its rank among your copies: kept (locks) or free (a gold box)
                let Some(rank) = ranked.iter().enumerate().position(|(i, c)| !used[i] && same(&c.held, h)) else { continue };
                used[rank] = true;
                let lock_at = ir.left_top() + vec2(6.0, 7.0);
                if rank == 0 {
                    lock(&painter, lock_at, 12.0, p.gold, false);
                } else if rank == 1 && mode == Mode::Backup {
                    // the backup: a green lock, open when let go; a click lets it go or keeps it again
                    let held = kept >= 2;
                    lock(&painter, lock_at, 12.0, if held { fixed::GREEN } else { fixed::MUTED }, !held);
                    let lr = Rect::from_center_size(lock_at, vec2(14.0, 16.0));
                    let resp = ui
                        .interact(lr, ui.scope_id().with(("st-backup", *id, n)), Sense::click())
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .on_hover_text(if held {
                            "Backup kept: click to let it go (it can be dismantled)"
                        } else {
                            "Backup let go: click to keep it"
                        });
                    if resp.clicked() {
                        toggled = Some(*id);
                    }
                }
            }
        }
        // a piece clicked: the other columns a little darker; an item shown: all but its own column
        let dim = match show {
            Some(((_, col, _), st, _)) if st > 0.0 => k != *col,
            _ => one.is_some() && !focus,
        };
        if dim {
            full.rect_filled(
                Rect::from_min_max(pos2(x0 - 8.0, bar.top() + 3.0), pos2(x0 + colw + 8.0, bottom)),
                0.0,
                Color32::from_black_alpha(90),
            );
        }
    }
    // the token flow: each place holding the set's tokens, its side, how many and how many pieces they make
    if let Some(f) = plan.first() {
        let fy = bar.top() + full_h - FLOW_H;
        // as the dividers: a gold line with short caps
        let g = p.gold;
        let line = Stroke::new(2.0, Color32::from_rgba_unmultiplied(g.r(), g.g(), g.b(), 170));
        painter.line_segment([pos2(bar.left() + 8.0, fy), pos2(bar.right() - 8.0, fy)], line);
        for x in [bar.left() + 8.0, bar.right() - 8.0] {
            painter.line_segment([pos2(x, fy - 3.0), pos2(x, fy + 3.0)], line);
        }
        let cy = fy + FLOW_H / 2.0;
        let mut x = bar.left() + 10.0;
        if let Some(tx) = icons.object(ui.ctx(), data, f.token, 40) {
            painter.image(tx.id(), Rect::from_center_size(pos2(x + 20.0, cy), vec2(40.0, 40.0)), uv(), Color32::WHITE);
        }
        x += 50.0;
        let amount = plan.iter().map(|f| f.amount).min().unwrap_or(1).max(1);
        // grouped by place, in the order held
        let mut places: Vec<(&lg_app::loadout::Held, i32)> = Vec::new();
        for c in &f.tokens {
            match places.iter_mut().find(|(h, _)| h.char_id == c.held.char_id && h.vault == c.held.vault) {
                Some(e) => e.1 += 1,
                None => places.push((&c.held, 1)),
            }
        }
        if places.is_empty() {
            jtext::draw(&painter, pos2(x, cy), Align2::LEFT_CENTER, "No tokens for this set held", &fs, fixed::MUTED);
        }
        for (k, (h, n)) in places.iter().enumerate() {
            const FLOW_CHIP: f32 = 44.0;
            let cr = Rect::from_center_size(pos2(x + FLOW_CHIP / 2.0, cy), vec2(FLOW_CHIP, FLOW_CHIP));
            copy_chip(ui, &painter, p, icons, app, rarity, h, f.token, cr, ("st-flow", k));
            x += FLOW_CHIP + 4.0;
            let side = if h.seasonal == Some(true) { "S" } else { "R" };
            let make = n / amount;
            let tt = format!("{side}  ×{n}  makes {make} piece{}", if make == 1 { "" } else { "s" });
            let r = jtext::draw(&painter, pos2(x, cy), Align2::LEFT_CENTER, &tt, &fs, if make > 0 { p.ink } else { fixed::MUTED });
            x = r.right() + 16.0;
            if x > bar.right() - 60.0 {
                break;
            }
        }
    }
    // the item clicked shown in the column beside its own (right, or left for the last one), over it
    let mut card_r = None;
    let mut drag = vec2(0.0, 0.0);
    if let Some(((item, col, held), st, off)) = show
        && st > 0.0
    {
        let sc = if *col + 1 < pieces.len() { col + 1 } else { col.saturating_sub(1) };
        let bottom = if plan.is_empty() { bar.top() + full_h - 8.0 } else { bar.top() + full_h - FLOW_H - 6.0 };
        let x = bar.left() + 8.0 + sc as f32 * colw;
        // at least wide enough for its rows; kept inside the bar
        let w = (colw - 8.0).max(320.0);
        let left = (x + 2.0).min(bar.right() - 6.0 - w).max(bar.left() + 6.0);
        let r = Rect::from_min_max(pos2(left, bar.top() + 10.0), pos2(left + w, bottom)).translate(off);
        // dragged anywhere by the mouse
        let resp = ui.interact(r, ui.scope_id().with("st-showcase"), Sense::drag()).on_hover_cursor(egui::CursorIcon::Grab);
        if resp.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
            drag = resp.drag_delta();
        }
        showcase(ui, p, icons, app, rarity, *item, held, r, st, stock, mode);
        card_r = Some(r);
    }
    BarOut { toggled, plan: plan_click, show: shown, card: card_r, drag }
}

/// The label of a craft that can be made: "CRAFTABLE" in gold, or once planned "PLANNED" in green in a grey box,
/// its left middle at `at`. True when clicked.
fn plan_label(
    ui: &Ui,
    painter: &egui::Painter,
    p: &Palette,
    at: egui::Pos2,
    planned: bool,
    right: f32,
    id: impl std::hash::Hash + std::fmt::Debug,
) -> bool {
    let label = if planned { "PLANNED" } else { "CRAFTABLE" };
    // as large as fits before `right` (the column's divider): the box's margin first, then the text smaller
    let (font, pad) = [(16.0, 14.0), (16.0, 8.0), (14.0, 8.0), (14.0, 4.0), (12.0, 4.0)]
        .into_iter()
        .map(|(s, pad)| (text(s), pad))
        .find(|(f, pad)| at.x + jtext::width(painter, label, f) + pad <= right)
        .unwrap_or_else(|| (text(12.0), 6.0));
    let font = &font;
    let w = jtext::width(painter, label, font);
    let h = jtext::height(painter, font);
    let r = Rect::from_min_size(pos2(at.x, at.y - h / 2.0 - 4.0), vec2(w + pad, h + 8.0));
    crate::guide::mark_first(ui, if planned { "inventory.planned" } else { "inventory.craftable" }, r);
    let resp =
        ui.interact(r, ui.scope_id().with(id), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(if planned {
            "Planned: what it takes is set aside. Click: not planned any more"
        } else {
            "Click to plan this craft: its tokens and the item it takes are set aside for it"
        });
    if planned {
        painter.rect(r, 3.0, mix(p.paper, p.line, 0.4), Stroke::new(1.0, p.ink_muted), StrokeKind::Inside);
        jtext::draw(painter, r.center(), Align2::CENTER_CENTER, label, font, fixed::GREEN);
    } else {
        if resp.hovered() {
            painter.rect_stroke(r, 3.0, Stroke::new(1.0, p.gold.gamma_multiply(0.6)), StrokeKind::Inside);
        }
        jtext::draw(painter, r.center(), Align2::CENTER_CENTER, label, font, p.gold);
    }
    resp.clicked()
}

/// A craft planned in a set's bar: the piece, on which side, the spare ST item and the tokens it takes (set aside).
#[derive(Clone, Debug)]
struct Planned {
    piece: i32,
    seasonal: bool,
    taken: Vec<(i32, lg_app::loadout::Held)>,
}

/// What a set's bar gives back: the item whose backup lock was clicked, an item clicked that is not of the set (it,
/// its column, its copy), where the item shown is.
struct BarOut {
    toggled: Option<i32>,
    /// A craft planned (Some) or no longer (None: the piece and side), when its label was clicked.
    plan: Option<Result<Planned, (i32, bool)>>,
    show: Option<(i32, usize, lg_app::loadout::Held)>,
    card: Option<Rect>,
    /// How far the item shown was dragged this frame.
    drag: egui::Vec2,
}

/// An item shown in a set's bar (`t` how far faded in), as the bar shows its pieces: its icon and name, then every
/// copy of it (the item beside where it is, in words too; the copy kept with a gold lock, a backup with a green one, the
/// copy clicked framed), the clicked copy's enchantments and what the item is worth; on a card of its own above the
/// bar (it can be dragged; scrolled by the wheel when it does not fit).
#[allow(clippy::too_many_arguments)]
fn showcase(
    ui: &Ui,
    p: &Palette,
    icons: &mut crate::icons::Icons,
    app: &lg_app::App,
    rarity: &Rarity,
    item: i32,
    held: &lg_app::loadout::Held,
    r: Rect,
    t: f32,
    stock: &lg_app::st_forge::Stock,
    mode: lg_app::st_forge::Mode,
) {
    use lg_app::st_forge::Mode;
    let data = app.world.data();
    // above all of the bar, its own layer
    let layer = egui::LayerId::new(egui::Order::Foreground, ui.scope_id().with("st-showcase-layer"));
    let mut shadow = ui.ctx().layer_painter(layer);
    shadow.set_opacity(t);
    shadow.rect_filled(r.translate(vec2(3.0, 4.0)), 6.0, Color32::from_black_alpha(90));
    shadow.rect(r, 6.0, p.paper, Stroke::new(2.0, p.gold), StrokeKind::Inside);
    let (f18, f16, fs) = (text(18.0), text(16.0), text(14.0));
    let (lh16, lhs) = (jtext::height(&shadow, &f16), jtext::height(&shadow, &fs));
    let (x, w) = (r.left() + 14.0, r.width() - 28.0);
    // what follows the name scrolls in the card
    let living: HashMap<i32, bool> = app.characters.living().iter().map(|l| (l.char_id, l.seasonal)).collect();
    let mut all = app.holdings.every_copy(&living, item);
    // the rarest first
    all.sort_by_key(|h| std::cmp::Reverse(h.enchant_ids.as_ref().map_or(0, |e| e.iter().filter(|x| **x >= 0).count())));
    let ench: Vec<String> = held
        .enchant_ids
        .iter()
        .flatten()
        .filter(|e| **e >= 0)
        .map(|e| data.enchant_looks.get(e).map_or(format!("Enchantment {e}"), |l| l.name.clone()))
        .collect();
    let head_h = BAR_TOP - 6.0 + 32.0 + 14.0;
    let view = Rect::from_min_max(pos2(r.left() + 2.0, r.top() + head_h), pos2(r.right() - 2.0, r.bottom() - 6.0));
    let content = (lh16 + 8.0)
        + all.len() as f32 * (ICON_B + BAR_ROW_GAP)
        + 10.0
        + (lh16 + 8.0)
        + ench.len().max(1) as f32 * (lhs + 3.0)
        + 10.0
        + (lh16 + 8.0)
        + lhs;
    let key = ui.scope_id().with(("st-showcase-scroll", item));
    let mut off = ui.data(|d| d.get_temp::<f32>(key)).unwrap_or(0.0);
    if ui.rect_contains_pointer(view) {
        let d = ui.input(|i| i.smooth_scroll_delta.y);
        if d != 0.0 {
            off -= d;
            ui.ctx().input_mut(|i| i.smooth_scroll_delta.y = 0.0);
        }
    }
    off = off.clamp(0.0, (content - view.height()).max(0.0));
    ui.data_mut(|d| d.insert_temp(key, off));
    // the item, as a piece of the set is shown
    let mut top = ui.ctx().layer_painter(layer).with_clip_rect(r);
    top.set_opacity(t);
    let y0 = r.top() + BAR_TOP - 6.0;
    if let Some(tx) = icons.object(ui.ctx(), data, item, 32) {
        top.image(tx.id(), Rect::from_min_size(pos2(x, y0), vec2(32.0, 32.0)), uv(), Color32::WHITE);
    }
    let it = data.items.item(item);
    let name = it.map_or(format!("Item {item}"), |i| i.name.clone());
    jtext::draw(&top, pos2(x + 40.0, y0 + 16.0), Align2::LEFT_CENTER, &widgets::fit(&top, &name, &f18, w - 42.0), &f18, p.ink);
    let mut painter = ui.ctx().layer_painter(layer).with_clip_rect(view);
    painter.set_opacity(t);
    let mut y = view.top() - off;
    let caption = |y: &mut f32, t: &str| {
        jtext::draw(&painter, pos2(x, *y), Align2::LEFT_TOP, t, &f16, fixed::MUTED);
        *y += lh16 + 8.0;
    };
    // every copy: the item (its rarity), where it is, a lock on those kept; the copy clicked framed
    caption(&mut y, &format!("ALL COPIES ({})", all.len()));
    let ranked = stock.ranked(item);
    let kept = stock.kept(item);
    let same = |a: &lg_app::loadout::Held, b: &lg_app::loadout::Held| {
        a.char_id == b.char_id && a.vault == b.vault && a.slot == b.slot && a.enchant_ids == b.enchant_ids
    };
    let mut used = vec![false; ranked.len()];
    let mut framed = false;
    for (n, h) in all.iter().enumerate() {
        let ir = Rect::from_min_size(pos2(x, y), vec2(ICON_B, ICON_B));
        item_icon(&painter, icons, ui, app, rarity, item, h, ir);
        let pr = Rect::from_min_size(pos2(ir.right() + 6.0, y), vec2(ICON_B, ICON_B));
        if view.contains(pr.center()) {
            place_look(ui, &painter, icons, app, h, item, pr, ("showcase-copy", item, n));
        }
        // where it is, and below it which side it is on
        let place = widgets::fit(&painter, &place_of(app, h).replace(" (seasonal)", ""), &fs, w - 2.0 * ICON_B - 18.0);
        jtext::draw(&painter, pos2(pr.right() + 8.0, pr.center().y - 1.0), Align2::LEFT_BOTTOM, &place, &fs, p.ink_soft);
        let seasonal = h.seasonal == Some(true);
        jtext::draw(
            &painter,
            pos2(pr.right() + 8.0, pr.center().y + 1.0),
            Align2::LEFT_TOP,
            if seasonal { "Seasonal" } else { "Regular" },
            &fs,
            if seasonal { p.gold } else { fixed::MUTED },
        );
        // the copy clicked (the first of equal ones)
        if !framed && same(h, held) {
            framed = true;
            painter.rect_stroke(
                Rect::from_min_max(ir.min - vec2(4.0, 3.0), pos2(r.right() - 10.0, ir.bottom() + 3.0)),
                4.0,
                Stroke::new(1.5, p.gold),
                StrokeKind::Inside,
            );
        }
        if mode != Mode::Off
            && let Some(rank) = ranked.iter().enumerate().position(|(i, c)| !used[i] && same(&c.held, h))
        {
            used[rank] = true;
            let at = ir.left_top() + vec2(6.0, 7.0);
            if rank == 0 {
                lock(&painter, at, 12.0, p.gold, false);
            } else if rank < kept {
                lock(&painter, at, 12.0, fixed::GREEN, false);
            }
        }
        y += ICON_B + BAR_ROW_GAP;
    }
    y += 10.0;
    // the clicked copy's enchantments by name
    caption(&mut y, "ENCHANTMENTS");
    if ench.is_empty() {
        jtext::draw(&painter, pos2(x, y), Align2::LEFT_TOP, "none", &fs, fixed::MUTED);
        y += lhs + 3.0;
    }
    for e in &ench {
        jtext::draw(&painter, pos2(x, y), Align2::LEFT_TOP, &widgets::fit(&painter, e, &fs, w), &fs, p.ink_soft);
        y += lhs + 3.0;
    }
    y += 10.0;
    // what it is worth
    caption(&mut y, "WORTH");
    let fp = it.map_or(0, |i| i.feed_power.max(0));
    let worth = format!("{fp} feed power  ·  {} essence", essence(data, item));
    jtext::draw(&painter, pos2(x, y), Align2::LEFT_TOP, &widgets::fit(&painter, &worth, &fs, w), &fs, p.ink_soft);
}

/// A vault place's look by its chest (`section`) and side: the vault chest (seasonal: its green texture), the gift
/// chest, the seasonal spoils, the material (forge) chest, the potion storage (the game's `containers.xml`).
fn chest_look(
    icons: &mut crate::icons::Icons,
    ctx: &egui::Context,
    data: &lg_gamedata::GameData,
    section: &str,
    seasonal: bool,
    size: u32,
) -> Option<egui::TextureHandle> {
    use lg_app::holdings::{GIFT_CHEST, MATERIALS, POTIONS, SPOILS};
    match section {
        GIFT_CHEST => icons.object(ctx, data, 0x0744, size),
        SPOILS => icons.object(ctx, data, 0x2944, size),
        MATERIALS => icons.object(ctx, data, 0x046F, size),
        POTIONS => icons.object(ctx, data, 0x0743, size),
        _ if seasonal => icons.sprite(ctx, "lofiObj", 0x21F, size),
        _ => icons.object(ctx, data, 0x0504, size),
    }
}

/// Where a copy is, in words: "Druid · Slot 3" (seasonal marked), "Vault · Vault chest", "Seasonal vault · Gift chest".
fn place_of(app: &lg_app::App, h: &lg_app::loadout::Held) -> String {
    if h.char_id > 0 {
        let class = app.characters.get(h.char_id).map_or("Character", |c| c.class_name.as_str());
        let seasonal = if h.seasonal == Some(true) { " (seasonal)" } else { "" };
        format!("{class}{seasonal} · {}", h.slot)
    } else if h.vault.as_deref() == Some(lg_app::holdings::SEASONAL_VAULT) {
        format!("Seasonal vault · {}", h.slot)
    } else {
        format!("Vault · {}", h.slot)
    }
}

/// The ST Sets side's Forge section for the set clicked: each missing piece the forge makes from its token, whether
/// you can forge it now, where your tokens are and the spare ST items it could take; drawn from `at` when `paint`,
/// else only measured. Gives the height.
#[allow(clippy::too_many_arguments)]
fn forge_rows(
    ui: &Ui,
    p: &Palette,
    icons: &mut crate::icons::Icons,
    app: &lg_app::App,
    rarity: &Rarity,
    set: &lg_gamedata::st_sets::StSet,
    plan: &[lg_app::st_forge::Piece],
    owned: usize,
    at: egui::Pos2,
    w: f32,
    paint: bool,
) -> f32 {
    let painter = ui.painter().clone();
    let data = app.world.data();
    let (f18, f16, fs) = (text(18.0), text(16.0), small());
    let (lh, lh16, lhs) = (jtext::height(&painter, &f18), jtext::height(&painter, &f16), jtext::height(&painter, &fs));
    let (x, mut y) = (at.x, at.y);
    let line = |y: &mut f32, s: &str, f: &egui::FontId, color: Color32, indent: f32, h: f32| {
        if paint {
            let t = widgets::fit(&painter, s, f, w - indent);
            jtext::draw(&painter, pos2(x + indent, *y), Align2::LEFT_TOP, &t, f, color);
        }
        *y += h;
    };
    line(&mut y, &format!("{}  ·  {}", set.title, set.class_name), &f18, p.gold, 0.0, lh + 4.0);
    if owned == 4 {
        line(&mut y, "All four pieces are yours.", &f16, fixed::MUTED, 0.0, lh16);
        return y - at.y;
    }
    if plan.is_empty() {
        line(&mut y, "Its missing pieces are not made from a set token.", &f16, fixed::MUTED, 0.0, lh16);
        return y - at.y;
    }
    // each missing piece: whether you can forge it now, else what it lacks
    for piece in plan {
        let row = 30f32.max(lh + lh16);
        if paint {
            let ir = Rect::from_min_size(pos2(x, y + (row - 28.0) / 2.0), vec2(28.0, 28.0));
            painter.rect_filled(ir, 0.0, if piece.craftable { mix(p.gold, p.paper, 0.35) } else { Color32::from_rgb(5, 5, 5) });
            painter.rect_stroke(ir, 0.0, Stroke::new(1.0, if piece.craftable { p.gold } else { Color32::BLACK }), StrokeKind::Inside);
            if let Some(t) = icons.object(ui.ctx(), data, piece.piece, 22) {
                painter.image(t.id(), Rect::from_center_size(ir.center(), vec2(22.0, 22.0)), uv(), Color32::WHITE);
            }
            let name = data.items.item(piece.piece).map_or(format!("Item {}", piece.piece), |i| i.name.clone());
            let nx = x + 28.0 + 8.0;
            jtext::draw(&painter, pos2(nx, y), Align2::LEFT_TOP, &widgets::fit(&painter, &name, &f18, w - 36.0), &f18, p.ink);
            let mut needs = Vec::new();
            let short = piece.amount - piece.tokens.len() as i32;
            if short > 0 {
                needs.push(format!("{short} more token{}", if short == 1 { "" } else { "s" }));
            }
            if let Some(slot) = &piece.st_slot
                && piece.spares.is_empty()
            {
                needs.push(format!("a spare ST {}", slot.to_lowercase()));
            }
            let (status, color) = if piece.craftable {
                ("You can forge it now".to_string(), fixed::GREEN)
            } else if needs.is_empty() {
                ("Tokens and spare are on different sides".to_string(), fixed::MUTED)
            } else {
                (format!("Needs {}", needs.join(" and ")), fixed::MUTED)
            };
            jtext::draw(&painter, pos2(nx, y + lh), Align2::LEFT_TOP, &widgets::fit(&painter, &status, &f16, w - 36.0), &f16, color);
        }
        y += row + 6.0;
    }
    // where your tokens sit, per side, each place with how many
    let Some(first) = plan.first() else { return y - at.y };
    y += 6.0;
    let token = data.items.item(first.token).map_or("Tokens".to_string(), |i| i.name.clone());
    line(&mut y, &format!("WHERE YOUR {} ARE", token.to_uppercase()), &fs, p.ink_muted, 0.0, lhs + 4.0);
    if first.tokens.is_empty() {
        line(&mut y, "None held", &f16, fixed::MUTED, 10.0, lh16);
    }
    // per side, each place as "2×" and its look (the character, or the vault's chest); where it is when hovered
    const CHIP: f32 = 40.0;
    for (label, seasonal) in [("Seasonal", true), ("Regular", false)] {
        let mut places: Vec<(&lg_app::loadout::Held, i32)> = Vec::new();
        for c in first.tokens.iter().filter(|c| (c.held.seasonal == Some(true)) == seasonal) {
            match places.iter_mut().find(|(h, _)| h.char_id == c.held.char_id && h.vault == c.held.vault) {
                Some(e) => e.1 += 1,
                None => places.push((&c.held, 1)),
            }
        }
        if places.is_empty() {
            continue;
        }
        line(&mut y, label, &f16, p.gold, 4.0, lh16 + 4.0);
        let mut cx = x + 14.0;
        for (k, (h, n)) in places.iter().enumerate() {
            let count = format!("{n}×");
            let tw = jtext::width(&painter, &count, &f16);
            if cx + tw + 4.0 + CHIP > x + w {
                cx = x + 14.0;
                y += CHIP + 4.0;
            }
            if paint {
                jtext::draw(&painter, pos2(cx, y + CHIP / 2.0), Align2::LEFT_CENTER, &count, &f16, p.ink);
                let cr = Rect::from_min_size(pos2(cx + tw + 4.0, y), vec2(CHIP, CHIP));
                copy_chip(ui, &painter, p, icons, app, rarity, h, first.token, cr, ("forge-token", seasonal, k));
            }
            cx += tw + 4.0 + CHIP + 14.0;
        }
        y += CHIP + 8.0;
    }
    y - at.y
}
