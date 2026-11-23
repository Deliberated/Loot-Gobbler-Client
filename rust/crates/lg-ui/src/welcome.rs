//! What greets you on start (Java `WelcomeDialog`, `WhatsNewDialog`): the greeting on the very first start, and the
//! release notes once after an update (also Info > Release Notes, with every release).

use crate::jtext;
use crate::theme::{self, Palette, fixed};
use egui::{Align2, Color32, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};

/// A published version's notes (what the updater reads from GitHub).
#[derive(Debug, Clone, PartialEq)]
pub struct Note {
    pub version: String,
    /// GitHub's publish time, e.g. "2026-11-01T15:44:44Z".
    pub published: String,
    pub body: String,
}

/// The release notes window: after an update (`from`: the version updated from) or every release (None).
pub struct ReleaseNotes {
    pub from: Option<String>,
    /// None while loading; Err: they could not be loaded.
    pub notes: Option<Result<Vec<Note>, String>>,
}

/// What the window asks of the app from here.
pub enum Out {
    Keep,
    Close,
    /// Info > Release Notes from the after-update window's ALL RELEASE NOTES.
    ShowAll,
    Open(String),
}

fn button(ui: &mut Ui, p: &Palette, r: Rect, t: &str, primary: bool, id: &str) -> bool {
    let resp = ui.interact(r, ui.scope_id().with(("welcome-button", id)), Sense::click());
    let (fill, ink) = if primary {
        (if resp.hovered() { p.gold_hover } else { p.gold }, p.on_accent)
    } else {
        (if resp.hovered() { p.sand } else { p.header }, p.ink)
    };
    ui.painter().rect(r, 0.0, fill, Stroke::new(1.5, Color32::BLACK), StrokeKind::Inside);
    jtext::draw(ui.painter(), r.center(), Align2::CENTER_CENTER, t, &crate::widgets::small(), ink);
    resp.clicked()
}

fn button_w(ui: &Ui, t: &str) -> f32 {
    (jtext::width(ui.painter(), t, &crate::widgets::small()) + 32.0).max(108.0)
}

/// The app icon at 128 px (2x its 64 px pixel art, so every pixel stays square).
fn icon(ctx: &egui::Context) -> egui::TextureHandle {
    let id = egui::Id::unique("lg-welcome-icon");
    if let Some(t) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(id)) {
        return t;
    }
    let img = image::load_from_memory(include_bytes!("../../../assets/icon/lootgobbler-256.png")).map(|i| i.to_rgba8()).unwrap_or_default();
    let t = ctx.load_texture(
        "welcome-icon",
        egui::ColorImage::from_rgba_unmultiplied([img.width() as usize, img.height() as usize], img.as_raw()),
        egui::TextureOptions::NEAREST,
    );
    ctx.data_mut(|d| d.insert_temp(id, t.clone()));
    t
}

/// The first start's offer to take over the Java app's records: from which folder (its default place unless changed),
/// and whether they are wanted.
#[derive(Debug, Clone, PartialEq)]
pub struct JavaOffer {
    pub path: String,
    pub take: bool,
}

/// What the greeting asks: to close (LET'S GO), or to pick the Java app's folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Greeting {
    Keep,
    Close,
    Browse,
}

/// Java `WelcomeDialog`: the goblin, "Hello fellow realmer!", what Loot Gobbler is for and how to start; LET'S GO.
/// On the first start of this app also the offer to take over the Java app's records (`offer`).
pub fn welcome(ctx: &egui::Context, p: &Palette, offer: &mut Option<JavaOffer>) -> Greeting {
    let mut close = false;
    let mut browse = false;
    let icon = icon(ctx);
    let modal = egui::Modal::new(egui::Id::unique("welcome")).frame(egui::Frame::NONE).show(ctx, |ui| {
        let painter = ui.painter().clone();
        // as wide as the wider of the heading and the 330 px the words wrap at (Java's dialog fits its content)
        let col = jtext::width(&painter, "Hello fellow realmer!", &theme::head(46.0)).max(330.0);
        let w = 16.0 + 32.0 + 128.0 + 28.0 + col + 32.0 + 16.0;
        let body = theme::body(25.0);
        let g1 = painter.layout(
            "I am here to be by your side and help you better understand your game. If you have questions, contact me (Gunshit) on Discord"
                .into(),
            body.clone(),
            p.ink,
            330.0,
        );
        let g2 = painter.layout(
            "When you're ready to start, click \"Start Sniffer\" in the bottom right and start your game".into(),
            body,
            p.ink,
            330.0,
        );
        let head = theme::head(46.0);
        let text_h = 6.0 + jtext::height(&painter, &head) + 8.0 + g1.size().y + 18.0 + g2.size().y;
        // the offer: a line to tick, the folder with BROWSE, a note
        let small = theme::body(18.0);
        let note = painter.layout(
            "This is where the Java Loot Gobbler keeps its records by default. If you installed it at its default place, leave this path as it is."
                .into(),
            theme::body(16.0),
            fixed::MUTED,
            w - 32.0 - 64.0,
        );
        let offer_h = if offer.is_some() { 24.0 + 30.0 + 8.0 + 36.0 + 6.0 + note.size().y } else { 0.0 };
        let h = 16.0 + 28.0 + text_h.max(128.0) + offer_h + 18.0 + 36.0 + 24.0 + 16.0;
        let (outer, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
        let r = outer.shrink(16.0);
        painter.rect(r, 0.0, p.paper, Stroke::new(2.0, p.line), StrokeKind::Inside);
        let (x, y) = (r.left() + 32.0, r.top() + 28.0);
        painter.image(icon.id(), Rect::from_min_size(pos2(x, y), vec2(128.0, 128.0)), crate::detail::uv(), Color32::WHITE);
        let tx = x + 128.0 + 28.0;
        let mut ty = y + 6.0;
        jtext::draw(&painter, pos2(tx, ty), Align2::LEFT_TOP, "Hello fellow realmer!", &head, p.gold);
        ty += jtext::height(&painter, &head) + 8.0;
        let h1 = g1.size().y;
        painter.galley(pos2(tx, ty), g1, p.ink);
        ty += h1 + 18.0;
        painter.galley(pos2(tx, ty), g2, p.ink);
        if let Some(o) = offer.as_mut() {
            let (lx, rx) = (r.left() + 32.0, r.right() - 32.0);
            let mut oy = y + text_h.max(128.0) + 24.0;
            painter.rect_filled(Rect::from_min_size(pos2(lx, oy - 12.0), vec2(rx - lx, 1.0)), 0.0, p.line);
            // the box to tick, and its words (both click)
            let label = "Take over my records and settings from the Java Loot Gobbler";
            let line = Rect::from_min_size(pos2(lx, oy), vec2(22.0 + 10.0 + jtext::width(&painter, label, &small), 30.0));
            let resp = ui.interact(line, ui.scope_id().with("java-offer-take"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
            if resp.clicked() {
                o.take = !o.take;
            }
            let bx = Rect::from_min_size(pos2(lx, oy + 4.0), vec2(22.0, 22.0));
            painter.rect(bx, 3.0, if o.take { p.gold } else { p.header }, Stroke::new(1.5, Color32::BLACK), StrokeKind::Inside);
            if o.take {
                let st = Stroke::new(2.5, p.on_accent);
                painter.line_segment([pos2(bx.left() + 5.0, bx.center().y), pos2(bx.left() + 9.5, bx.bottom() - 6.0)], st);
                painter.line_segment([pos2(bx.left() + 9.5, bx.bottom() - 6.0), pos2(bx.right() - 5.0, bx.top() + 6.0)], st);
            }
            jtext::draw(&painter, pos2(bx.right() + 10.0, bx.center().y), Align2::LEFT_CENTER, label, &small, p.ink);
            oy += 30.0 + 8.0;
            // the folder: typed, or picked with BROWSE
            let bw = button_w(ui, "BROWSE");
            let field = Rect::from_min_size(pos2(lx, oy), vec2(rx - lx - bw - 8.0, 36.0));
            if o.take {
                crate::widgets::search_field(ui, p, field, &mut o.path, "The Java Loot Gobbler's folder", "java-offer-path");
            } else {
                // not wanted: the folder shown greyed out
                painter.rect(field, 0.0, p.header, Stroke::new(1.0, p.line), StrokeKind::Inside);
                let shown = crate::widgets::fit(&painter, &o.path, &theme::body(18.0), field.width() - 16.0);
                jtext::draw(&painter, pos2(field.left() + 8.0, field.center().y), Align2::LEFT_CENTER, &shown, &theme::body(18.0), fixed::MUTED);
            }
            if o.take && button(ui, p, Rect::from_min_size(pos2(field.right() + 8.0, oy), vec2(bw, 36.0)), "BROWSE", false, "java-browse") {
                browse = true;
            }
            oy += 36.0 + 6.0;
            painter.galley(pos2(lx, oy), note, fixed::MUTED);
        }
        let bw = button_w(ui, "LET'S GO");
        let br = Rect::from_min_size(pos2(r.right() - 32.0 - bw, r.bottom() - 24.0 - 36.0), vec2(bw, 36.0));
        if button(ui, p, br, "LET'S GO", true, "lets-go") || (offer.is_none() && ui.input(|i| i.key_pressed(egui::Key::Enter))) {
            close = true;
        }
    });
    if browse {
        Greeting::Browse
    } else if close || modal.should_close() {
        Greeting::Close
    } else {
        Greeting::Keep
    }
}

/// Days since 1970 of a date.
fn days(y: i64, m: i64, d: i64) -> i64 {
    let (y, m) = if m <= 2 { (y - 1, m + 9) } else { (y, m - 3) };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * m + 2) / 5 + d - 1;
    era * 146_097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719_468
}

/// Java `date`: GitHub's publish time ("2026-11-01T15:44:44Z") as "1 Nov 2026" in local time, or "".
fn date(zone: &lg_app::localtime::Zone, published: &str) -> String {
    let n = |a: usize, b: usize| published.get(a..b).and_then(|s| s.parse::<i64>().ok());
    let (Some(y), Some(mo), Some(d), Some(h), Some(mi), Some(s)) = (n(0, 4), n(5, 7), n(8, 10), n(11, 13), n(14, 16), n(17, 19)) else {
        return String::new();
    };
    let ms = ((days(y, mo, d) * 24 + h) * 60 + mi) * 60_000 + s * 1000;
    let local = zone.local_date_time(ms);
    const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let m = local.get(5..7).and_then(|s| s.parse::<usize>().ok()).unwrap_or(1);
    let dd = local.get(8..10).and_then(|s| s.parse::<u32>().ok()).unwrap_or(1);
    format!("{dd} {} {}", MONTHS[m.clamp(1, 12) - 1], &local[..4])
}

/// A line of the notes with Java `inline`'s bits of Markdown: **bold**, `code` and [links](url).
fn inline(ui: &mut Ui, p: &Palette, line: &str, size: f32, bullet: bool, out: &mut Out) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(0.0, 2.0);
        let f = theme::body(size);
        if bullet {
            ui.label(egui::RichText::new("  •  ").font(f.clone()).color(p.ink));
        }
        let mut rest = line;
        while !rest.is_empty() {
            let next = [rest.find("**"), rest.find('`'), rest.find('[')].into_iter().flatten().min();
            let Some(at) = next else {
                ui.label(egui::RichText::new(rest).font(f.clone()).color(p.ink));
                break;
            };
            if at > 0 {
                ui.label(egui::RichText::new(&rest[..at]).font(f.clone()).color(p.ink));
                rest = &rest[at..];
                continue;
            }
            if let Some(body) = rest.strip_prefix("**")
                && let Some(end) = body.find("**")
            {
                ui.label(egui::RichText::new(&body[..end]).font(f.clone()).color(p.gold).strong());
                rest = &body[end + 2..];
            } else if let Some(body) = rest.strip_prefix('`')
                && let Some(end) = body.find('`')
            {
                ui.label(egui::RichText::new(&body[..end]).font(egui::FontId::monospace(size * 0.8)).color(p.ink));
                rest = &body[end + 1..];
            } else if let Some(body) = rest.strip_prefix('[')
                && let Some(end) = body.find("](")
                && let Some(close) = body[end + 2..].find(')')
                && body[end + 2..].starts_with("http")
            {
                let (t, url) = (&body[..end], &body[end + 2..end + 2 + close]);
                if ui.link(egui::RichText::new(t).font(f.clone())).clicked() {
                    *out = Out::Open(url.to_string());
                }
                rest = &body[end + 2 + close + 1..];
            } else {
                // a lone marker: as it is
                let c = rest.chars().next().map_or(1, char::len_utf8);
                ui.label(egui::RichText::new(&rest[..c]).font(f.clone()).color(p.ink));
                rest = &rest[c..];
            }
        }
    });
}

/// A right-pointing arrow in blocky pixel style (Java `Arrow`): a shaft and a stepped head.
fn arrow(painter: &egui::Painter, at: egui::Pos2, size: f32, color: Color32) {
    let px = (size / 8.0).floor().max(2.0);
    let mid = at.y + size / 2.0 - px / 2.0;
    painter.rect_filled(Rect::from_min_size(pos2(at.x, mid), vec2(size - 2.0 * px, px)), 0.0, color);
    for i in 0..4 {
        let hx = at.x + size - (4 - i) as f32 * px;
        let half = (3 - i) as f32;
        painter.rect_filled(Rect::from_min_size(pos2(hx, mid - half * px), vec2(px, (2.0 * half + 1.0) * px)), 0.0, color);
    }
}

impl ReleaseNotes {
    /// Java `WhatsNewDialog`: the version change (or "every release") above the notes, one version after the other
    /// with its date, its "- " lines as a list.
    pub fn show(&self, ctx: &egui::Context, p: &Palette, zone: &lg_app::localtime::Zone, version: &str) -> Out {
        let mut out = Out::Keep;
        let modal = egui::Modal::new(egui::Id::unique("release-notes")).frame(egui::Frame::NONE).show(ctx, |ui| {
            let screen = ctx.content_rect();
            let w = (1050.0f32 + 56.0 + 4.0).min(screen.width() - 64.0);
            let list_h = if self.from.is_none() { 520.0 } else { 330.0 };
            let painter = ui.painter().clone();
            let head = theme::head(40.0);
            let sub = if self.from.is_none() { theme::body(22.0) } else { theme::body(28.0) };
            let head_h = jtext::height(&painter, &head) + 4.0 + jtext::height(&painter, &sub);
            let h = (16.0 + 22.0 + head_h + 14.0 + list_h + 14.0 + 36.0 + 20.0 + 16.0).min(screen.height() - 64.0);
            let (outer, _) = ui.allocate_exact_size(vec2(w + 32.0, h), Sense::hover());
            let r = outer.shrink(16.0);
            painter.rect(r, 0.0, p.paper, Stroke::new(2.0, p.line), StrokeKind::Inside);
            let x = r.left() + 28.0;
            let mut y = r.top() + 22.0;
            let title = if self.from.is_none() { "Release Notes" } else { "Loot Gobbler was updated!" };
            jtext::draw(&painter, pos2(x, y), Align2::LEFT_TOP, title, &head, p.gold);
            y += jtext::height(&painter, &head) + 4.0;
            match &self.from {
                Some(from) => {
                    let t = jtext::draw(&painter, pos2(x, y), Align2::LEFT_TOP, from, &sub, p.ink);
                    arrow(&painter, pos2(t.right() + 12.0, t.center().y - 11.0), 22.0, p.gold);
                    jtext::draw(&painter, pos2(t.right() + 12.0 + 22.0 + 12.0, y), Align2::LEFT_TOP, version, &sub, p.ink);
                }
                None => {
                    let t = format!("Every release, the latest first. You have {version}.");
                    jtext::draw(&painter, pos2(x, y), Align2::LEFT_TOP, &t, &sub, p.ink);
                }
            }
            y += jtext::height(&painter, &sub) + 14.0;
            let list = Rect::from_min_max(pos2(x, y), pos2(r.right() - 28.0, r.bottom() - 20.0 - 36.0 - 14.0));
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(list).layout(egui::Layout::top_down(egui::Align::Min)));
            egui::ScrollArea::vertical().id_salt("release-notes-list").auto_shrink([false, false]).show(&mut child, |ui| {
                ui.set_width(list.width() - 16.0);
                let plain = |ui: &mut Ui, t: &str| {
                    ui.label(egui::RichText::new(t).font(theme::body(20.0)).color(p.ink));
                };
                match &self.notes {
                    None => plain(ui, "Loading the release notes..."),
                    Some(Err(e)) => plain(ui, e),
                    Some(Ok(notes)) if notes.is_empty() => plain(ui, "No release notes were published for this version."),
                    Some(Ok(notes)) => {
                        for n in notes {
                            ui.add_space(10.0);
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(&n.version).font(theme::head(30.0)).color(p.ink));
                                let d = date(zone, &n.published);
                                if !d.is_empty() {
                                    ui.add_space(14.0);
                                    ui.label(egui::RichText::new(d).font(theme::body(24.0)).color(fixed::MUTED));
                                }
                            });
                            ui.add_space(4.0);
                            for raw in n.body.lines() {
                                let line = raw.trim();
                                if let Some(item) = line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")) {
                                    inline(ui, p, item, 20.0, true, &mut out);
                                } else if line.starts_with('#') {
                                    let t = line.trim_start_matches('#').trim();
                                    ui.label(egui::RichText::new(t).font(theme::body(20.0)).color(p.ink).strong());
                                } else if !line.is_empty() {
                                    inline(ui, p, line, 20.0, false, &mut out);
                                }
                            }
                        }
                    }
                }
            });
            // ALL RELEASE NOTES / ON GITHUB, and NICE / CLOSE
            let (other, ok) = if self.from.is_none() { ("ON GITHUB", "CLOSE") } else { ("ALL RELEASE NOTES", "NICE") };
            let (ow, kw) = (button_w(ui, other), button_w(ui, ok));
            let kr = Rect::from_min_size(pos2(r.right() - 28.0 - kw, r.bottom() - 20.0 - 36.0), vec2(kw, 36.0));
            let orr = Rect::from_min_size(pos2(kr.left() - 8.0 - ow, kr.top()), vec2(ow, 36.0));
            if button(ui, p, orr, other, false, "notes-other") {
                out = if self.from.is_none() { Out::Open(String::new()) } else { Out::ShowAll };
            }
            if button(ui, p, kr, ok, true, "notes-ok") || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                out = Out::Close;
            }
        });
        if modal.should_close() && matches!(out, Out::Keep) {
            out = Out::Close;
        }
        out
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn dates_as_java() {
        let zone = lg_app::localtime::Zone::default();
        assert_eq!(super::date(&zone, "2026-11-01T15:44:44Z"), "1 Nov 2026");
        assert_eq!(super::date(&zone, ""), "");
    }
}
