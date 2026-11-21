//! An item's history beside the Inventory grid, in place of the Recent list (Java `CollectionPanel.renderHistory`):
//! the item, where your copies are now, the drops of it you found and the copies used up.

use crate::detail::{RARITY_COLORS, Rarity, enchant_badge, uv};
use crate::icons::Icons;
use crate::jtext;
use crate::theme::{Palette, fixed};
use crate::widgets::{self, small, text};
use egui::{Align2, Color32, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};
use lg_app::App;
use lg_app::collection::{self, Find, Seen};

const UT_COLOR: Color32 = Color32::from_rgb(0x80, 0x49, 0xA8);
const ST_COLOR: Color32 = Color32::from_rgb(0xE0, 0x7B, 0x2A);
const TIER_COLOR: Color32 = Color32::from_rgb(0xB0, 0x7D, 0x0A);
const COUNT_GOLD: Color32 = Color32::from_rgb(0xF5, 0xC5, 0x42);
const SEEN_TILE: f32 = 52.0;
const SEEN_SPRITE: u32 = 44;
const VAULT_CHEST: i32 = 0x0504;

/// Days since 1970 of "YYYY-MM-DD".
fn day_number(d: &str) -> Option<i64> {
    let y: i64 = d.get(0..4)?.parse().ok()?;
    let m: i64 = d.get(5..7)?.parse().ok()?;
    let dd: i64 = d.get(8..10)?.parse().ok()?;
    let (y, m) = if m <= 2 { (y - 1, m + 9) } else { (y, m - 3) };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * m + 2) / 5 + dd - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era * 146_097 + doe - 719_468)
}

/// Java `daysAgo`: "today", "1 day", "63 days", counted in calendar days.
fn days_ago(time: &str, today: &str) -> String {
    match (day_number(time), day_number(today)) {
        (Some(a), Some(b)) => match b - a {
            d if d <= 0 => "today".into(),
            1 => "1 day".into(),
            d => format!("{d} days"),
        },
        _ => String::new(),
    }
}

/// Java `how`: where a find came from.
fn how(f: &Find) -> String {
    let or = |s: &str, d: &str| if s.is_empty() { d.to_string() } else { s.to_string() };
    match f.source.as_str() {
        collection::VAULT => or(&f.bag, "Vault"),
        collection::INVENTORY => or(&f.bag, "Inventory"),
        collection::FORGED => "Forged (used in the forge)".into(),
        collection::FED => "Fed to a pet".into(),
        collection::DROPPED => "Dropped on the ground".into(),
        collection::CONSUMED => "Used up (forge or pet)".into(),
        _ => {
            let bag = if f.bag.is_empty() { "Drop".to_string() } else { format!("{} bag", f.bag) };
            if f.r#where.is_empty() { bag } else { format!("{bag}  ·  {}", f.r#where) }
        }
    }
}

/// Java `usedCopies`: each removal with the find of the copy it used up, if it can be told.
fn used_copies(history: &[Find]) -> Vec<(usize, Option<usize>)> {
    let mut out = Vec::new();
    let mut taken = vec![false; history.len()];
    for (i, r) in history.iter().enumerate() {
        if !r.is_removal() {
            continue;
        }
        let mut found = None;
        for j in (0..i).rev() {
            let f = &history[j];
            if f.is_removal() || taken[j] {
                continue;
            }
            let same = match &r.enchant_ids {
                Some(e) => Some(e) == f.enchant_ids.as_ref() && (r.seasonal.is_none() || f.seasonal.is_none() || r.seasonal == f.seasonal),
                None => f.source == collection::INVENTORY,
            };
            if same {
                found = Some(j);
                break;
            }
        }
        if let Some(j) = found {
            taken[j] = true;
        }
        out.push((i, found));
    }
    out
}

/// Java `asUsed`: a removal shown with what it knew of the copy used up.
fn as_used(removal: &Find, was: Option<&Find>) -> Find {
    let Some(was) = was else { return removal.clone() };
    let mut f = removal.clone();
    if removal.enchant_ids.is_none() {
        f.enchant_ids = was.enchant_ids.clone();
        f.enchants = was.enchants;
    }
    f.seasonal = removal.seasonal.or(was.seasonal);
    f.seen = removal.seen.clone().or_else(|| was.seen.clone());
    f
}

struct Rows<'a> {
    ui: &'a mut Ui,
    p: &'a Palette,
    app: &'a App,
    icons: &'a mut Icons,
    rarity: &'a Rarity,
    x: f32,
    w: f32,
    y: f32,
    today: String,
}

impl Rows<'_> {
    fn muted(&mut self, t: &str) {
        let painter = self.ui.painter().clone();
        let g = painter.layout(t.to_string(), text(18.0), fixed::MUTED, self.w);
        let h = g.size().y;
        painter.galley(pos2(self.x, self.y), g, fixed::MUTED);
        self.y += h + 4.0;
    }

    /// Java `heading`: a line, then a small grey title.
    fn heading(&mut self, t: &str) {
        let painter = self.ui.painter().clone();
        self.y += 6.0;
        painter.rect_filled(Rect::from_min_size(pos2(self.x, self.y), vec2(self.w, 1.0)), 0.0, self.p.line);
        self.y += 2.0 + 8.0;
        let r = jtext::draw(&painter, pos2(self.x, self.y), Align2::LEFT_TOP, t, &small(), fixed::MUTED);
        self.y = r.bottom() + 6.0;
    }

    /// Java `findRow`: icon with its enchantment diamonds; where it came from (seasonal or not) and its
    /// enchantments; the character or vault chest it was seen in last; how long ago.
    fn find(&mut self, item: i32, f: &Find, k: usize) {
        let painter = self.ui.painter().clone();
        let ctx = self.ui.ctx().clone();
        let data = self.app.world.data();
        let f18 = text(18.0);
        let lh = jtext::height(&painter, &f18);
        let count = f.enchant_ids.as_ref().map_or(f.enchants.max(0) as usize, Vec::len);
        let has_icons = f.enchant_ids.as_ref().is_some_and(|e| e.iter().any(|i| *i >= 0));
        let unknown = f.enchant_ids.is_none() && f.enchants < 0 && !f.is_removal();
        let text_h = lh + if has_icons { 4.0 + 22.0 } else { 0.0 } + if unknown { jtext::height(&painter, &small()) } else { 0.0 };
        let h = text_h.max(SEEN_TILE) + 8.0;
        let cy = self.y + h / 2.0;
        // the icon
        let ir = Rect::from_min_size(pos2(self.x, cy - 16.0), vec2(32.0, 32.0));
        if let Some(t) = self.icons.object(&ctx, data, item, 32) {
            painter.image(t.id(), ir, uv(), Color32::WHITE);
        }
        enchant_badge(&painter, self.rarity, ir.right_bottom() - vec2(1.0, 1.0), count, 5.0);
        if count > 0 {
            let names = ["", "Uncommon", "Rare", "Legendary", "Divine"];
            let t = format!("{count} {} ({})", if count == 1 { "enchantment" } else { "enchantments" }, names[count.min(4)]);
            widgets::tip(self.ui, ir, ("hist-ench", k), &t);
        }
        // at the right: how long ago, and left of it where it was seen last
        let ago_f = crate::theme::head(17.0);
        let ago_w = jtext::width(&painter, "999 days", &ago_f) + 4.0;
        let ago = days_ago(&f.time, &self.today);
        let right = self.x + self.w;
        let ar = jtext::draw(&painter, pos2(right, cy), Align2::RIGHT_CENTER, &ago, &ago_f, COUNT_GOLD);
        if !f.time.is_empty() {
            widgets::tip(self.ui, ar, ("hist-ago", k), &f.time.replace('T', " "));
        }
        let tile = Rect::from_min_size(pos2(right - ago_w - 16.0 - SEEN_TILE, cy - SEEN_TILE / 2.0), vec2(SEEN_TILE, SEEN_TILE));
        self.seen_tile(tile, f.seen.as_ref(), k);
        // the middle: what, seasonal or not, the enchantments
        let tx = ir.right() + 8.0;
        let tw = (tile.left() - 8.0 - tx).max(20.0);
        let mut ty = cy - text_h / 2.0;
        let what = how(f);
        let wr = jtext::draw(&painter, pos2(tx, ty), Align2::LEFT_TOP, &widgets::fit(&painter, &what, &f18, tw), &f18, self.p.ink);
        if let Some(s) = f.seasonal
            && wr.right() + 10.0 < tile.left()
        {
            let (t, c) = if s {
                ("(seasonal)", Color32::from_rgb(0x2B, 0xD8, 0xB0))
            } else {
                ("(non-seasonal)", Color32::from_rgb(0xE5, 0x63, 0x6E))
            };
            let sr = jtext::draw(&painter, pos2(wr.right() + 10.0, ty), Align2::LEFT_TOP, t, &f18, c);
            let tip = if s { "Found on a seasonal character" } else { "Found on a non-seasonal character" };
            widgets::tip(self.ui, sr, ("hist-season", k), tip);
        }
        ty += lh;
        if has_icons && let Some(ids) = &f.enchant_ids {
            ty += 4.0;
            let frame = RARITY_COLORS[ids.len().min(4)];
            let mut ex = tx;
            for (j, e) in ids.iter().enumerate() {
                if *e < 0 {
                    continue;
                }
                let er = Rect::from_min_size(pos2(ex, ty), vec2(22.0, 22.0));
                if let Some((sheet, idx)) = data.enchant_looks.get(e).and_then(|l| l.texture.clone())
                    && let Some(t) = self.icons.sheet(&ctx, &sheet, idx, 20)
                {
                    painter.image(t.id(), er.shrink(1.0), uv(), Color32::WHITE);
                }
                painter.rect_stroke(er, 0.0, Stroke::new(1.0, frame), StrokeKind::Inside);
                let name = data.enchant_looks.get(e).map_or(format!("Enchantment #{e}"), |l| l.name.clone());
                widgets::tip(self.ui, er, ("hist-e", k, j), &name);
                ex += 22.0 + 4.0;
            }
            ty += 22.0;
        }
        if unknown {
            let r = jtext::draw(&painter, pos2(tx, ty), Align2::LEFT_TOP, "enchantments unknown", &small(), fixed::MUTED);
            widgets::tip(
                self.ui,
                r,
                ("hist-unknown", k),
                "Filled in once the game sends this item's enchantments (character list at login, or Refresh in Characters)",
            );
        }
        self.y += h + 6.0;
    }

    /// Java `seenTile`: the character as it looked, or a vault chest; empty (same size) until seen.
    fn seen_tile(&mut self, r: Rect, seen: Option<&Seen>, k: usize) {
        let ctx = self.ui.ctx().clone();
        let data = self.app.world.data();
        let Some(s) = seen else { return };
        let (tex, tip) = if s.char_id > 0 {
            let sprite = if s.skin > 0 && !data.items.transform_skins.contains(&s.skin) { s.skin } else { s.class_type };
            let place = if s.place.is_empty() { "character" } else { s.place.as_str() };
            (
                self.icons.skin(&ctx, data, sprite, s.tex1, s.tex2, SEEN_SPRITE),
                format!("Last seen on your {place}{} #{}", if s.seasonal { " (seasonal)" } else { "" }, s.char_id),
            )
        } else {
            (self.icons.object(&ctx, data, VAULT_CHEST, SEEN_SPRITE), format!("Last seen in your {}", s.place.to_lowercase()))
        };
        if let Some(t) = tex {
            let sz = SEEN_SPRITE as f32;
            self.ui.painter().image(t.id(), Rect::from_center_size(r.center(), vec2(sz, sz)), uv(), Color32::WHITE);
            widgets::tip(self.ui, r, ("hist-seen", k), &tip);
        }
    }
}

/// Java `asFind`: a copy you hold now, as a row of the history.
fn as_find(app: &App, h: &lg_app::loadout::Held, seen_at: i64) -> Find {
    let source = if h.char_id > 0 { collection::INVENTORY } else { collection::VAULT };
    let time = if seen_at > 0 { app.zone.local_date_time(seen_at) } else { String::new() };
    let mut f = Find::of(source, h.enchant_ids.as_ref().map_or(-1, |e| e.len() as i32), "", &h.slot, seen_at, &time);
    f.enchant_ids = h.enchant_ids.clone();
    f.seasonal = h.seasonal;
    f.seen = if h.char_id > 0 {
        app.characters.all().find(|c| c.char_id == h.char_id).map(Seen::character)
    } else {
        let seasonal = if h.vault.as_deref() == Some(lg_app::holdings::SEASONAL_VAULT) { "seasonal " } else { "" };
        Some(Seen::vault(&format!("{seasonal}{}", h.slot)))
    };
    f
}

/// The history of `item` in `r` (the side section's body); `held`: how many you have now.
pub fn show(ui: &mut Ui, p: &Palette, app: &App, icons: &mut Icons, rarity: &Rarity, item: i32, held: i32, now: i64, r: Rect) {
    let data = app.world.data();
    let Some(it) = data.items.item(item) else { return };
    let entry = app.collection.entry(item);
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(r));
    egui::ScrollArea::vertical().id_salt(("item-history", item)).show(&mut child, |ui| {
        let top = ui.cursor().min;
        let x = top.x + 10.0;
        let w = r.width() - 20.0 - 12.0;
        let painter = ui.painter().clone();
        let ctx = ui.ctx().clone();
        let mut y = top.y + 8.0 + 4.0;
        // the item: icon, name, tags and slot, count, where it drops
        if let Some(t) = icons.object(&ctx, data, item, 72) {
            painter.image(t.id(), Rect::from_min_size(pos2(x, y), vec2(72.0, 72.0)), uv(), Color32::WHITE);
        }
        let tx = x + 72.0 + 12.0;
        let tw = (w - 84.0).max(40.0);
        let name = painter.layout(it.name.clone(), text(22.0), p.ink, tw);
        let mut ty = y;
        let nh = name.size().y;
        painter.galley(pos2(tx, ty), name, p.ink);
        ty += nh + 4.0;
        let mut bx = tx;
        let tier = if it.is_ut() {
            Some(("UT".to_string(), UT_COLOR))
        } else if it.is_st() {
            Some(("ST".to_string(), ST_COLOR))
        } else if it.tier.is_empty() {
            None
        } else {
            Some((format!("T{}", it.tier), TIER_COLOR))
        };
        let f18 = text(18.0);
        let lh = jtext::height(&painter, &f18);
        let badge = |t: &str, bg: Color32, bx: &mut f32| {
            let bw = jtext::width(&painter, t, &f18) + 12.0;
            let br = Rect::from_min_size(pos2(*bx, ty), vec2(bw, lh + 2.0));
            painter.rect_filled(br, 0.0, bg);
            jtext::draw(&painter, br.center(), Align2::CENTER_CENTER, t, &f18, Color32::WHITE);
            *bx += bw + 6.0;
        };
        if let Some((t, c)) = &tier {
            badge(t, *c, &mut bx);
        }
        if it.is_shiny() {
            badge("SHINY", Color32::from_rgb(0x3F, 0xA7, 0xC4), &mut bx);
        }
        jtext::draw(&painter, pos2(bx, ty + 1.0), Align2::LEFT_TOP, it.slot_name(), &f18, p.ink);
        ty += lh + 2.0 + 6.0;
        let cr = jtext::draw(
            &painter,
            pos2(tx, ty),
            Align2::LEFT_TOP,
            &format!("Count: {held}"),
            &text(20.0),
            if held == 0 { fixed::MUTED } else { COUNT_GOLD },
        );
        ty = cr.bottom();
        // where it drops: from the game data, else the dungeons your own drops of it came from
        let places = it.origin().map(str::to_string).unwrap_or_else(|| {
            let mut counted: Vec<(String, i32)> = Vec::new();
            for f in entry.and_then(|e| e.history.as_ref()).into_iter().flatten() {
                if f.source == collection::DROP && !f.r#where.trim().is_empty() {
                    match counted.iter_mut().find(|(w, _)| *w == f.r#where) {
                        Some(c) => c.1 += 1,
                        None => counted.push((f.r#where.clone(), 1)),
                    }
                }
            }
            counted.sort_by_key(|c| std::cmp::Reverse(c.1));
            counted.iter().map(|(w, n)| if *n > 1 { format!("{w} ×{n}") } else { w.clone() }).collect::<Vec<_>>().join(", ")
        });
        if places.is_empty() {
            let r = jtext::draw(&painter, pos2(tx, ty), Align2::LEFT_TOP, "Drop location not known yet", &f18, fixed::MUTED);
            ty = r.bottom();
        } else {
            let c = jtext::draw(&painter, pos2(tx, ty), Align2::LEFT_TOP, "Drops in: ", &f18, fixed::MUTED);
            let g = painter.layout(places, f18.clone(), p.ink, (tx + tw - c.right()).max(40.0));
            let gh = g.size().y;
            painter.galley(pos2(c.right(), ty), g, p.ink);
            ty += gh;
        }
        y = y.max(ty).max(y + 72.0) + 10.0;
        painter.rect_filled(Rect::from_min_size(pos2(x, y), vec2(w, 1.0)), 0.0, p.line);
        y += 1.0 + 12.0;

        let today = app.zone.local_date_time(app.world.now().max(now));
        let mut rows = Rows { ui, p, app, icons, rarity, x, w, y, today };
        // where your copies are now, each as the game last showed its place (nothing guessed)
        rows.heading("WHERE THEY ARE");
        if !app.holdings.known() {
            rows.muted("Shown once Loot Gobbler has seen your characters (log in) and your vault.");
        } else {
            let living: std::collections::HashMap<i32, bool> = app.characters.living().iter().map(|l| (l.char_id, l.seasonal)).collect();
            let copies = app.holdings.every_copy(&living, item);
            if copies.is_empty() {
                rows.muted("None on your characters or in your vaults.");
            }
            for (k, h) in copies.iter().enumerate() {
                let key = if h.char_id > 0 { format!("char:{}", h.char_id) } else { h.vault.clone().unwrap_or_default() };
                let seen_at = app.holdings.places.get(&key).map_or(0, |pl| pl.seen_at);
                let f = as_find(app, h, seen_at);
                rows.find(item, &f, 1000 + k);
            }
            // Java `gaps`: what was not seen yet
            let unseen = living.keys().filter(|id| !app.holdings.places.contains_key(&format!("char:{id}"))).count();
            let seasonal = living.values().any(|s| *s);
            let mut parts = Vec::new();
            if unseen > 0 {
                parts.push(format!("{unseen} {}", if unseen == 1 { "character" } else { "characters" }));
            }
            if !app.holdings.places.contains_key(lg_app::holdings::VAULT) {
                parts.push("your vault".into());
            }
            if seasonal && !app.holdings.places.contains_key(lg_app::holdings::SEASONAL_VAULT) {
                parts.push("your seasonal vault".into());
            }
            if !parts.is_empty() {
                rows.muted(&format!("Not seen yet: {}. Their copies are not counted until then.", parts.join(", ")));
            }
        }
        if let Some(history) = entry.and_then(|e| e.history.as_ref()) {
            // what happened: drops (and chests opened), latest first; below, the copies used up
            rows.heading("FOUND");
            let drops: Vec<&Find> = history.iter().rev().filter(|f| f.source == collection::DROP).collect();
            if drops.is_empty() {
                let first = entry.and_then(|e| e.first.as_deref()).map(|f| f[..f.len().min(10)].to_string());
                rows.muted(&format!(
                    "No drop seen{}.",
                    first.map_or(String::new(), |d| format!("; first seen in your inventory or vault on {d}"))
                ));
            }
            for (k, f) in drops.iter().enumerate() {
                rows.find(item, f, 2000 + k);
            }
            let used = used_copies(history);
            if !used.is_empty() {
                rows.heading("USED");
                for (k, (i, was)) in used.iter().rev().enumerate() {
                    let f = as_used(&history[*i], was.map(|j| &history[j]));
                    rows.find(item, &f, 3000 + k);
                }
            }
        }
        let end = rows.y;
        ui.allocate_exact_size(vec2(r.width() - 12.0, end - top.y + 10.0), Sense::hover());
    });
}
