//! Building blocks the Java views share: text as Java's labels draw it, cards (paper with a double line border),
//! rows of equal cards as tall as the tallest, captions, headings.

use crate::jtext;
use crate::theme::{self, Palette};
use egui::{Align2, Color32, FontId, Rect, Sense, Shape, Stroke, StrokeKind, Ui, pos2, vec2};

/// A label: one line of text at the cursor, Java's spacing; gives its response.
pub fn label(ui: &mut Ui, text: &str, font: &FontId, color: Color32) -> egui::Response {
    let p = ui.painter().clone();
    let size = vec2(jtext::width(&p, text, font), jtext::height(&p, font));
    let (r, resp) = ui.allocate_exact_size(size, Sense::click());
    jtext::draw(&p, r.min, Align2::LEFT_TOP, text, font, color);
    // its words for screen readers (and the tests)
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Label, true, text));
    resp
}

/// Label at the right edge of the available width.
pub fn label_right(ui: &mut Ui, text: &str, font: &FontId, color: Color32, inset: f32) -> egui::Response {
    let p = ui.painter().clone();
    let w = ui.available_width();
    let size = vec2(w, jtext::height(&p, font));
    let (r, resp) = ui.allocate_exact_size(size, Sense::hover());
    jtext::draw(&p, pos2(r.right() - inset, r.top()), Align2::RIGHT_TOP, text, font, color);
    resp
}

/// Text font of the Java labels (Jersey 15) at a pixel size.
pub fn text(px: f32) -> FontId {
    theme::body(px)
}

/// The `small` style: Jersey 10 at 16, scaled.
pub fn small() -> FontId {
    theme::head(16.0)
}

/// Jersey 10 at a pixel size as it is (Java `new Font(HEAD_FONT, PLAIN, px)`).
pub fn head_px(px: f32) -> FontId {
    FontId::new(px, egui::FontFamily::Name(theme::HEAD.into()))
}

/// Java `card`: paper, a two-pixel line border, 14/18 padding; content at the top. The background is drawn after
/// the content (in a reserved slot) at `min_height` or the content's height.
pub fn card<R>(ui: &mut Ui, p: &Palette, width: f32, min_height: f32, add: impl FnOnce(&mut Ui) -> R) -> (Rect, R) {
    let slot = ui.painter().add(Shape::Noop);
    let top = ui.cursor().min;
    let inner = Rect::from_min_size(pos2(top.x + 18.0, top.y + 14.0), vec2(width - 36.0, f32::INFINITY));
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner).layout(egui::Layout::top_down(egui::Align::Min)));
    child.spacing_mut().item_spacing = vec2(0.0, 0.0);
    let r = add(&mut child);
    let h = (child.min_rect().height() + 28.0).max(min_height);
    let rect = Rect::from_min_size(top, vec2(width, h));
    paint_card(ui, slot, rect, p);
    (rect, r)
}

pub fn paint_card(ui: &Ui, slot: egui::layers::ShapeIdx, rect: Rect, p: &Palette) {
    ui.painter().set(
        slot,
        Shape::Vec(vec![
            Shape::rect_filled(rect, 0.0, p.paper),
            Shape::rect_stroke(rect, 0.0, Stroke::new(1.0, p.line), StrokeKind::Inside),
            Shape::rect_stroke(rect.shrink(1.0), 0.0, Stroke::new(1.0, p.line), StrokeKind::Inside),
        ]),
    );
}

/// A card's content, given what the cards of a row share (e.g. the icons), one card after the other.
pub type Card<'a, S> = Box<dyn FnOnce(&mut Ui, &mut S) + 'a>;

/// Java `row`: cards of equal width side by side (gap between), all as tall as the tallest; the cursor moves below.
pub fn row<S>(ui: &mut Ui, p: &Palette, gap: f32, state: &mut S, cards: Vec<Card<'_, S>>) {
    let n = cards.len() as f32;
    let left = ui.cursor().min;
    let w = ((ui.available_width() - gap * (n - 1.0)) / n).floor();
    let mut slots = Vec::new();
    let mut tallest: f32 = 0.0;
    for (i, add) in cards.into_iter().enumerate() {
        let x = left.x + i as f32 * (w + gap);
        let slot = ui.painter().add(Shape::Noop);
        let inner = Rect::from_min_size(pos2(x + 18.0, left.y + 14.0), vec2(w - 36.0, f32::INFINITY));
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner).layout(egui::Layout::top_down(egui::Align::Min)));
        child.spacing_mut().item_spacing = vec2(0.0, 0.0);
        add(&mut child, state);
        tallest = tallest.max(child.min_rect().height() + 28.0);
        slots.push((slot, x));
    }
    for (slot, x) in slots {
        paint_card(ui, slot, Rect::from_min_size(pos2(x, left.y), vec2(w, tallest)), p);
    }
    ui.allocate_exact_size(vec2(ui.available_width(), tallest), Sense::hover());
}

/// The muted caption above a value (Java `caption`).
pub fn caption(ui: &mut Ui, p: &Palette, t: &str) -> egui::Response {
    label(ui, t, &small(), p.ink_muted)
}

/// A gold heading in the head font at 30 (Java `heading`).
pub fn heading(ui: &mut Ui, p: &Palette, t: &str) -> egui::Response {
    label(ui, t, &head_px(30.0), p.gold)
}

pub fn soft(ui: &mut Ui, p: &Palette, t: &str) -> egui::Response {
    label(ui, t, &text(18.0), p.ink_soft)
}

pub fn muted(ui: &mut Ui, t: &str) -> egui::Response {
    label(ui, t, &text(18.0), theme::fixed::MUTED)
}

/// An icon at its size at the cursor.
pub fn icon(ui: &mut Ui, tex: Option<&egui::TextureHandle>, size: f32) -> egui::Response {
    let (r, resp) = ui.allocate_exact_size(vec2(size, size), Sense::click());
    if let Some(t) = tex {
        ui.painter().image(t.id(), r, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
    }
    resp
}

/// Java's `Section`: a line-bordered panel with a header strip (a coloured square, the upper-case title in the tab
/// font, controls at the right) above a paper body. `actions` draws right-to-left from the header's right end and
/// gives back how wide they were; `body` fills the rest. Gives the whole rect.
pub fn section(
    ui: &mut Ui,
    p: &Palette,
    rect: Rect,
    title: &str,
    accent: Color32,
    actions: impl FnOnce(&mut Ui, Rect),
    body: impl FnOnce(&mut Ui, Rect),
) {
    let painter = ui.painter().clone();
    let font = theme::head(17.0);
    let th = jtext::height(&painter, &font);
    let head_h = th + 12.0 + 1.0;
    painter.rect_filled(rect, 0.0, p.paper);
    let head = Rect::from_min_size(rect.min + vec2(1.0, 1.0), vec2(rect.width() - 2.0, head_h - 1.0));
    painter.rect_filled(head, 0.0, p.header);
    painter.rect_filled(Rect::from_min_size(pos2(rect.left(), head.bottom()), vec2(rect.width(), 1.0)), 0.0, p.line);
    let sq = Rect::from_min_size(pos2(head.left() + 10.0, head.center().y - 4.0), vec2(8.0, 8.0));
    painter.rect_filled(sq, 0.0, accent);
    jtext::draw(&painter, pos2(sq.right() + 8.0, head.center().y), Align2::LEFT_CENTER, &title.to_uppercase(), &font, p.ink_soft);
    let mut a = ui.new_child(egui::UiBuilder::new().max_rect(head));
    actions(&mut a, Rect::from_min_max(pos2(head.left(), head.top() + 2.0), pos2(head.right() - 4.0, head.bottom() - 2.0)));
    let body_r = Rect::from_min_max(pos2(rect.left() + 1.0, head.bottom() + 1.0), pos2(rect.right() - 1.0, rect.bottom() - 1.0));
    let mut b = ui.new_child(egui::UiBuilder::new().max_rect(body_r).layout(egui::Layout::top_down(egui::Align::Min)));
    b.set_clip_rect(body_r.intersect(ui.clip_rect()));
    b.spacing_mut().item_spacing = vec2(0.0, 0.0);
    body(&mut b, body_r);
    painter.rect_stroke(rect, 0.0, Stroke::new(1.0, p.line), StrokeKind::Inside);
}

/// The height of a section's header strip.
pub fn section_head_height(painter: &egui::Painter) -> f32 {
    jtext::height(painter, &theme::head(17.0)) + 12.0
}

/// The width a row of buttons takes (see [`segmented`]).
pub fn segmented_width(painter: &egui::Painter, names: &[&str], joined: bool) -> f32 {
    let font = small();
    let w: f32 = names.iter().map(|n| jtext::width(painter, &n.to_uppercase(), &font) + 30.0).sum();
    w + if joined { 0.0 } else { 8.0 * (names.len() as f32 - 1.0) }
}

/// Java `Segmented`: toggle buttons in a row (`joined`: touching, as one control), the selected one gold. Drawn with
/// its left top at `at`, `h` high; gives the clicked one.
/// Where [`segmented`]'s buttons stand (the same arguments).
pub fn segment_rects(painter: &egui::Painter, at: egui::Pos2, h: f32, names: &[&str], joined: bool) -> Vec<Rect> {
    let font = small();
    let mut x = at.x;
    names
        .iter()
        .map(|n| {
            let w = jtext::width(painter, &n.to_uppercase(), &font) + 30.0;
            let r = Rect::from_min_size(pos2(x, at.y), vec2(w, h));
            x += w + if joined { 0.0 } else { 8.0 };
            r
        })
        .collect()
}

/// Where a row of section buttons keeps the one last selected and its pour (from, to, how far 0..1): by its names,
/// not where it stands (a page sliding in moves it).
fn pour_id(ui: &Ui, _at: egui::Pos2, names: &[&str]) -> egui::Id {
    ui.scope_id().with(("seg-pour", names))
}

/// A pour starting now from buttons `from..=end` shown as one (`label`, e.g. two tabs melted) into `to`, which is then
/// taken as the row's selection.
pub fn segmented_pour_merged(ui: &Ui, at: egui::Pos2, names: &[&str], from: usize, end: usize, label: &str, to: usize) {
    let id = pour_id(ui, at, names);
    ui.data_mut(|d| {
        d.insert_temp(id, to);
        d.insert_temp(id.with("t"), (from, to, 0.0f32));
        d.insert_temp(id.with("span"), (end, label.to_string()));
    });
}

/// The row's selection taken as it is, without a pour (e.g. when the view animates the change itself).
pub fn segmented_settle(ui: &Ui, at: egui::Pos2, names: &[&str], selected: usize) {
    let id = pour_id(ui, at, names);
    ui.data_mut(|d| {
        d.insert_temp(id, selected);
        d.remove::<(usize, usize, f32)>(id.with("t"));
        d.remove::<(usize, String)>(id.with("span"));
    });
}

pub fn segmented(
    ui: &Ui,
    p: &Palette,
    at: egui::Pos2,
    h: f32,
    names: &[&str],
    selected: usize,
    joined: bool,
    disabled: &[usize],
) -> Option<usize> {
    let font = small();
    let painter = ui.painter().clone();
    // another one selected: gold pours from the one before into it (not when nothing or something gone was)
    let id = pour_id(ui, at, names);
    let before = ui.data(|d| d.get_temp::<usize>(id));
    if let Some(b) = before
        && b != selected
        && b < names.len()
        && selected < names.len()
    {
        ui.data_mut(|d| {
            d.insert_temp(id.with("t"), (b, selected, 0.0f32));
            d.remove::<(usize, String)>(id.with("span"));
        });
    }
    if before != Some(selected) {
        ui.data_mut(|d| d.insert_temp(id, selected));
    }
    let mut x = at.x;
    let mut clicked = None;
    for (i, n) in names.iter().enumerate() {
        let t = n.to_uppercase();
        let w = jtext::width(&painter, &t, &font) + 30.0;
        let r = Rect::from_min_size(pos2(x, at.y), vec2(w, h));
        let off = disabled.contains(&i);
        let resp =
            ui.interact(r, ui.scope_id().with(("seg", n, at.x as i32, at.y as i32)), if off { Sense::hover() } else { Sense::click() });
        let on = selected == i;
        let (bg, fg) = if on {
            (p.gold, p.on_accent)
        } else if off {
            (p.paper, p.ink_muted)
        } else if resp.hovered() {
            (p.header, p.ink_soft)
        } else {
            (p.paper, p.ink_soft)
        };
        painter.rect(r, 0.0, bg, Stroke::new(1.5, Color32::BLACK), StrokeKind::Inside);
        jtext::draw(&painter, r.center(), Align2::CENTER_CENTER, &t, &font, fg);
        if resp.clicked() {
            clicked = Some(i);
        }
        x += w + if joined { 0.0 } else { 8.0 };
    }
    if let Some((from, to, t)) = ui.data(|d| d.get_temp::<(usize, usize, f32)>(id.with("t"))) {
        let length = 0.45 + 0.15 * (from.abs_diff(to) as f32 - 1.0);
        let t = (t + ui.input(|i| i.stable_dt).min(0.1) / length).min(1.0);
        let span = ui.data(|d| d.get_temp::<(usize, String)>(id.with("span")));
        pour(ui, p, at, h, names, joined, from, to, t, span.as_ref().map(|(e, l)| (*e, l.as_str())));
        ui.data_mut(|d| {
            if t < 1.0 {
                d.insert_temp(id.with("t"), (from, to, t));
            } else {
                d.remove::<(usize, usize, f32)>(id.with("t"));
                d.remove::<(usize, String)>(id.with("span"));
            }
        });
        ui.ctx().request_repaint();
    }
    clicked
}

/// Gold pouring from one section button into another (`t` 0..1), drawn over the row: the gold moves as one body about a
/// button long, so every button on the way fills and drains in turn (a stream over the gaps); the label dark where the
/// gold is. `merged`: the buttons `from..=end` shown as one box with that label (where it pours from).
#[allow(clippy::too_many_arguments)]
pub fn pour(
    ui: &Ui,
    p: &Palette,
    at: egui::Pos2,
    h: f32,
    names: &[&str],
    joined: bool,
    from: usize,
    to: usize,
    t: f32,
    merged: Option<(usize, &str)>,
) {
    let painter = ui.painter().clone();
    let font = small();
    // the boxes as shown: the merged ones as one
    let mut rects = Vec::new();
    let mut labels = Vec::new();
    let (mut from_c, mut to_c) = (usize::MAX, usize::MAX);
    let all = segment_rects(&painter, at, h, names, joined);
    let mut k = 0;
    while k < all.len() {
        let end = merged.filter(|_| k == from).map_or(k, |(e, _)| e.min(all.len() - 1));
        if k == from {
            from_c = rects.len();
        }
        if (k..=end).contains(&to) {
            to_c = rects.len();
        }
        rects.push(all[k].union(all[end]));
        labels.push(if end > k { merged.map_or(String::new(), |(_, l)| l.to_uppercase()) } else { names[k].to_uppercase() });
        k = end + 1;
    }
    let (from, to) = (from_c, to_c);
    if from == to {
        return;
    }
    let (Some(src), Some(dst)) = (rects.get(from).copied(), rects.get(to).copied()) else { return };
    let ease = |v: f32| {
        let v = v.clamp(0.0, 1.0);
        v * v * (3.0 - 2.0 * v)
    };
    let right = to > from;
    // along the row in the pour's direction: the body's front and back, from filling the one left to filling the one
    // selected
    let (lead0, lead1, back0, back1) =
        if right { (src.right(), dst.right(), src.left(), dst.left()) } else { (src.left(), dst.left(), src.right(), dst.right()) };
    // the body about one button long (a little shorter on the way, stretched thin), moving as one
    let e = ease(t);
    let lead = lead0 + (lead1 - lead0) * e;
    let len = ((lead0 - back0) + ((lead1 - back1) - (lead0 - back0)) * e) * (1.0 - 0.3 * (std::f32::consts::PI * e).sin());
    let back = lead - len;
    // the front a drop's tip on its side: a point widening to the full height; none at the start and the end
    let dir = if right { 1.0 } else { -1.0 };
    let tip_len = (h * 0.9).min(len.abs()) * (std::f32::consts::PI * e).sin();
    let neck = lead - dir * tip_len;
    let (lo, hi) = (lead.min(back), lead.max(back));
    let gold = p.gold;
    let cy = src.center().y;
    // the stream over the gaps the body spans
    for k in from.min(to)..from.max(to) {
        let (g0, g1) = (rects[k].right() - 1.5, rects[k + 1].left() + 1.5);
        let (x0, x1) = (g0.max(lo), g1.min(hi));
        if x1 > x0 {
            painter.rect_filled(Rect::from_min_max(pos2(x0, cy - 3.0), pos2(x1, cy + 3.0)), 2.0, gold);
        }
    }
    // each button on the way: the part of the body in it, its front rounded
    for k in from.min(to)..=from.max(to) {
        let r = rects[k];
        let label = labels[k].clone();
        painter.rect(r, 0.0, p.paper, Stroke::new(1.5, Color32::BLACK), StrokeKind::Inside);
        jtext::draw(&painter, r.center(), Align2::CENTER_CENTER, &label, &font, p.ink_soft);
        let inner = r.shrink(1.5);
        let (x0, x1) = (inner.left().max(lo), inner.right().min(hi));
        if x1 <= x0 {
            continue;
        }
        let gr = Rect::from_min_max(pos2(x0, inner.top()), pos2(x1, inner.bottom()));
        let clip = painter.with_clip_rect(inner.intersect(painter.clip_rect()));
        // the full-height part up to the neck, then the tip
        let (b0, b1) = (back.min(neck), back.max(neck));
        clip.rect_filled(
            Rect::from_min_max(pos2(b0.max(inner.left()), inner.top()), pos2(b1.min(inner.right()), inner.bottom())),
            0.0,
            gold,
        );
        if tip_len > 0.5 {
            let half = inner.height() / 2.0;
            let mut mesh = egui::Mesh::default();
            let n = 16;
            for i in 0..=n {
                // u 0 at the tip, 1 at the neck: pointed at the tip, smooth into the full height
                let u = i as f32 / n as f32;
                let x = lead - dir * tip_len * u;
                let y = half * (std::f32::consts::FRAC_PI_2 * u).sin().powf(1.5);
                mesh.colored_vertex(pos2(x, inner.center().y - y), gold);
                mesh.colored_vertex(pos2(x, inner.center().y + y), gold);
                if i > 0 {
                    let k = 2 * i as u32;
                    mesh.add_triangle(k - 2, k - 1, k);
                    mesh.add_triangle(k - 1, k + 1, k);
                }
            }
            clip.add(Shape::mesh(mesh));
        }
        // the label dark where the gold is full height (the body, half the tip)
        let mid = lead - dir * tip_len * 0.5;
        let full = Rect::from_min_max(pos2(back.min(mid), inner.top()), pos2(back.max(mid), inner.bottom()));
        let dark = painter.with_clip_rect(gr.intersect(full).intersect(painter.clip_rect()));
        jtext::draw(&dark, r.center(), Align2::CENTER_CENTER, &label, &font, p.on_accent);
    }
}

/// Java's banner (SEASONAL / CRUCIBLE / UPE): a slab with notched corners, a dark bottom side, white text with a
/// grey outline; faded when off. Gives its size for a text.
pub fn banner_size(painter: &egui::Painter, text: &str) -> egui::Vec2 {
    let f = head_px(24.0);
    // Java's FontMetrics.getAscent (rounded up)
    let asc: f32 = (24.0f32 * 1125.0 / 1400.0).ceil();
    let h = asc + 18.0;
    let u = (h / 5.6).round().max(3.0);
    vec2(jtext::width(painter, &text.to_uppercase(), &f) + 2.0 * u + 12.0, h)
}

/// `under`: the colour it lies on. A faded banner (alpha below 1) is drawn in solid colours mixed with it, like Java
/// fading the finished banner as a whole: fading each part on its own let the overlapping parts (the text's outline
/// copies) show through each other.
pub fn banner(painter: &egui::Painter, r: Rect, text: &str, bg: Color32, alpha: f32, under: Color32) {
    let f = head_px(24.0);
    let (w, h) = (r.width(), r.height());
    let u = (h / 5.6).round().max(3.0);
    let fade = |c: Color32| if alpha >= 1.0 { c } else { crate::theme::mix(c, under, f64::from(alpha.max(0.0))) };
    let rect = |x: f32, y: f32, ww: f32, hh: f32, c: Color32| {
        painter.rect_filled(Rect::from_min_size(pos2(r.left() + x, r.top() + y), vec2(ww, hh)), 0.0, fade(c))
    };
    let dark = crate::theme::mix(bg, Color32::BLACK, 0.76);
    rect(u, 0.0, w - 2.0 * u, h, Color32::BLACK);
    rect(0.0, u, w, h - 2.0 * u, Color32::BLACK);
    rect(u + 1.0, 1.0, w - 2.0 * u - 2.0, h - 2.0, dark);
    rect(1.0, u + 1.0, w - 2.0, h - 2.0 * u - 2.0, dark);
    rect(u + 1.0, 1.0, w - 2.0 * u - 2.0, h - 2.0 - u, bg);
    rect(1.0, u + 1.0, w - 2.0, h - 2.0 * u - 2.0 - ((u + 1.0) / 2.0).floor(), bg);
    let t = text.to_uppercase();
    let face = h - u;
    let tw = jtext::width(painter, &t, &f);
    let asc: f32 = (24.0f32 * 1125.0 / 1400.0).ceil();
    let base = ((face + asc) / 2.0).floor() - 1.0;
    let top = r.top() + base - asc;
    let x = r.left() + ((w - tw) / 2.0).floor();
    for dx in -1..=1 {
        for dy in -1..=1 {
            if dx != 0 || dy != 0 {
                jtext::draw(
                    painter,
                    pos2(x + dx as f32, top + dy as f32),
                    Align2::LEFT_TOP,
                    &t,
                    &f,
                    fade(Color32::from_rgb(0x5C, 0x5C, 0x5C)),
                );
            }
        }
    }
    jtext::draw(painter, pos2(x, top), Align2::LEFT_TOP, &t, &f, fade(Color32::WHITE));
}

/// Java `OutlinedText`: pixel-font text with a 2 px black outline and a shadow below; its size.
pub fn outlined_size(painter: &egui::Painter, text: &str, px: f32) -> egui::Vec2 {
    let f = head_px(px);
    vec2(jtext::width(painter, text, &f) + 6.0, (px * 1125.0 / 1400.0).ceil() + 6.0)
}

pub fn outlined_text(painter: &egui::Painter, r: Rect, text: &str, px: f32, color: Color32) {
    let f = head_px(px);
    let asc = (px * 1125.0 / 1400.0).ceil();
    let base = ((r.height() + asc) / 2.0).floor() - 1.0;
    let (x, top) = (r.left() + 2.0, r.top() + base - asc);
    for dx in -2..=2 {
        for dy in -2..=3 {
            if dx != 0 || dy != 0 {
                jtext::draw(painter, pos2(x + dx as f32, top + dy as f32), Align2::LEFT_TOP, text, &f, Color32::BLACK);
            }
        }
    }
    jtext::draw(painter, pos2(x, top), Align2::LEFT_TOP, text, &f, color);
}

/// Java `kpi`: a header-coloured box with a line border, the caption above a gold value.
pub fn kpi(ui: &mut Ui, p: &Palette, r: Rect, caption_text: &str, value: &str) {
    let painter = ui.painter();
    painter.rect(r, 0.0, p.header, Stroke::new(1.0, p.line), StrokeKind::Inside);
    let c = jtext::draw(painter, pos2(r.left() + 9.0, r.top() + 5.0), Align2::LEFT_TOP, &caption_text.to_uppercase(), &small(), p.ink_soft);
    jtext::draw(painter, pos2(r.left() + 9.0, c.bottom()), Align2::LEFT_TOP, value, &text(24.0), crate::theme::fixed::GOLD);
}

/// A KPI box's height.
pub fn kpi_height(painter: &egui::Painter) -> f32 {
    jtext::height(painter, &small()) + jtext::height(painter, &text(24.0)) + 10.0
}

/// A [`section`] as tall as its content (Java `CharactersView.box`: the content inside a 10 px border, 8 px below the
/// box): `body` draws from the given top left within the width and gives back how tall it was. Gives the box's
/// height with the gap.
pub fn section_box(
    ui: &mut Ui,
    p: &Palette,
    at: egui::Pos2,
    w: f32,
    title: &str,
    accent: Color32,
    actions: impl FnOnce(&mut Ui, Rect),
    body: impl FnOnce(&mut Ui, Rect) -> f32,
) -> f32 {
    let painter = ui.painter().clone();
    let bg = painter.add(Shape::Noop);
    let font = theme::head(17.0);
    let head_h = jtext::height(&painter, &font) + 12.0;
    let head = Rect::from_min_size(at + vec2(1.0, 1.0), vec2(w - 2.0, head_h));
    let sq = Rect::from_min_size(pos2(head.left() + 10.0, head.center().y - 4.0), vec2(8.0, 8.0));
    painter.rect_filled(sq, 0.0, accent);
    jtext::draw(&painter, pos2(sq.right() + 8.0, head.center().y), Align2::LEFT_CENTER, &title.to_uppercase(), &font, p.ink_soft);
    let mut a = ui.new_child(egui::UiBuilder::new().max_rect(head));
    actions(&mut a, Rect::from_min_max(pos2(head.left(), head.top() + 2.0), pos2(head.right() - 4.0, head.bottom() - 2.0)));
    let inner = Rect::from_min_size(pos2(at.x + 11.0, head.bottom() + 1.0 + 10.0), vec2(w - 22.0, f32::INFINITY));
    let mut b = ui.new_child(egui::UiBuilder::new().max_rect(inner).layout(egui::Layout::top_down(egui::Align::Min)));
    b.spacing_mut().item_spacing = vec2(0.0, 0.0);
    let content = body(&mut b, inner);
    let h = 1.0 + head_h + 1.0 + 10.0 + content + 10.0 + 1.0;
    let rect = Rect::from_min_size(at, vec2(w, h));
    painter.set(
        bg,
        Shape::Vec(vec![
            Shape::rect_filled(rect, 0.0, p.paper),
            Shape::rect_filled(head, 0.0, p.header),
            Shape::rect_filled(Rect::from_min_size(pos2(rect.left(), head.bottom()), vec2(w, 1.0)), 0.0, p.line),
        ]),
    );
    painter.rect_stroke(rect, 0.0, Stroke::new(1.0, p.line), StrokeKind::Inside);
    h + 8.0
}

/// Text cut to fit a width, ending in "..." (Java's label when it gets less room than it needs).
pub fn fit(painter: &egui::Painter, t: &str, font: &FontId, w: f32) -> String {
    if jtext::width(painter, t, font) <= w {
        return t.to_string();
    }
    let mut s: String = t.to_string();
    while !s.is_empty() && jtext::width(painter, &format!("{s}..."), font) > w {
        s.pop();
    }
    format!("{}...", s.trim_end())
}

/// Text wrapped at word boundaries to a width (Java `JTextArea` with word wrap): its lines.
pub fn wrap(painter: &egui::Painter, t: &str, font: &FontId, width: f32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in t.split(' ') {
        let candidate = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
        if !line.is_empty() && jtext::width(painter, &candidate, font) > width {
            lines.push(std::mem::take(&mut line));
            line = word.to_string();
        } else {
            line = candidate;
        }
    }
    lines.push(line);
    lines
}

/// Wrapped text (with Java's 2 px above and below) at the top left; gives its height.
pub fn wrapped(painter: &egui::Painter, at: egui::Pos2, t: &str, font: &FontId, width: f32, color: Color32) -> f32 {
    let lh = jtext::height(painter, font);
    let lines = wrap(painter, t, font, width);
    for (i, l) in lines.iter().enumerate() {
        jtext::draw(painter, pos2(at.x, at.y + 2.0 + i as f32 * lh), Align2::LEFT_TOP, l, font, color);
    }
    lines.len() as f32 * lh + 4.0
}

/// Java `Color.brighter`.
pub fn brighter(c: Color32) -> Color32 {
    let f = |v: u8| -> u8 {
        let v = if v == 0 { 3.0 } else { f32::from(v) };
        (v / 0.7).min(255.0) as u8
    };
    if c.r() == 0 && c.g() == 0 && c.b() == 0 {
        return Color32::from_rgb(3, 3, 3);
    }
    Color32::from_rgb(if c.r() > 0 { f(c.r()) } else { 0 }, if c.g() > 0 { f(c.g()) } else { 0 }, if c.b() > 0 { f(c.b()) } else { 0 })
}

/// A one-line text field (Java `JTextField` with a placeholder): paper with a line border, gold when focused;
/// the text typed goes into `value`. Gives whether it changed.
pub fn search_field(ui: &mut Ui, p: &Palette, r: Rect, value: &mut String, placeholder: &str, id: &str) -> bool {
    let painter = ui.painter().clone();
    let bg = painter.add(Shape::Noop);
    let font = text(18.0);
    let inner = r.shrink2(vec2(8.0, 2.0));
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner).layout(egui::Layout::left_to_right(egui::Align::Center)));
    child.visuals_mut().extreme_bg_color = Color32::TRANSPARENT;
    child.visuals_mut().selection.bg_fill = p.text_sel;
    child.visuals_mut().text_cursor.stroke = Stroke::new(1.5, p.ink);
    let edit = egui::TextEdit::singleline(value)
        .id_salt(id)
        .frame(egui::Frame::NONE)
        .font(font.clone())
        .text_color(p.ink)
        .hint_text(egui::RichText::new(placeholder).font(font).color(p.ink_muted))
        .desired_width(inner.width())
        .margin(vec2(0.0, 0.0));
    let resp = child.add(edit);
    let focused = resp.has_focus();
    painter.set(bg, Shape::rect_filled(r, 0.0, p.paper));
    painter.rect_stroke(r, 0.0, Stroke::new(1.0, if focused { p.gold } else { p.line }), StrokeKind::Inside);
    resp.changed()
}

/// Java `setToolTipText` on a painted part: the text when the pointer rests over `r` (several lines with `\n`).
pub fn tip(ui: &Ui, r: Rect, id: impl std::hash::Hash + std::fmt::Debug, t: &str) {
    if t.is_empty() {
        return;
    }
    let resp = ui.interact(r, ui.scope_id().with(("tip", id)), Sense::hover());
    let p_ink = ui.visuals().override_text_color.unwrap_or(Color32::WHITE);
    show_tip(ui, resp, |ui| {
        let f = text(18.0);
        for line in t.lines() {
            label(ui, line, &f, p_ink);
        }
    });
}

/// A tooltip drawn by `add` when the pointer rests over `r`.
pub fn custom_tip(ui: &Ui, r: Rect, id: impl std::hash::Hash + std::fmt::Debug, add: impl FnOnce(&mut Ui)) {
    let resp = ui.interact(r, ui.scope_id().with(("custom-tip", id)), Sense::hover());
    show_tip(ui, resp, add);
}

/// A tooltip over a hover-only area. egui hovers nothing below a clickable widget, so a tip under (registered
/// before) an item link or a button would never show: it then shows once the pointer rests on the area.
fn show_tip(ui: &Ui, resp: egui::Response, add: impl FnOnce(&mut Ui)) {
    if resp.hovered() {
        resp.on_hover_ui(add);
        return;
    }
    if !resp.contains_pointer() || ui.ctx().dragged_id().is_some() || ui.input(|i| i.pointer.any_down()) {
        return;
    }
    let delay = ui.style().interaction.tooltip_delay;
    let rest = ui.input(|i| i.pointer.time_since_last_movement());
    if rest >= delay {
        egui::Tooltip::for_widget(&resp).show(add);
    } else {
        ui.ctx().request_repaint_after_secs(delay - rest);
    }
}

/// Java `ItemLinks.link`: an item's icon that opens the item in the Inventory tab when clicked, with a hand cursor;
/// only for the items the Inventory tracks (UTs, STs and top tiers). Marked for the Newcomer's Guide.
pub fn item_link(ui: &Ui, data: &lg_gamedata::GameData, r: Rect, id: i32) {
    let Some(item) = (id > 0).then(|| data.items.item(id)).flatten() else { return };
    if !item.is_collectible() {
        // say why a click does nothing
        tip(ui, r, ("item-link-note", id, r.min.x as i32, r.min.y as i32), "Not in the Inventory (it tracks UT/ST and top-tier items)");
        return;
    }
    crate::guide::mark_item(ui, r, id);
    let resp = ui.interact(r, ui.scope_id().with(("item-link", id, r.min.x as i32, r.min.y as i32)), Sense::click());
    if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
        crate::window::open_later(ui.ctx(), format!("Inventory:item:{id}"));
    }
}

const SPLIT_GAP: f32 = 8.0;

fn split_id(name: &str) -> egui::Id {
    egui::Id::unique("lg-side-width").with(name)
}

/// A split's state: the side's width (set by dragging), the main area's width on screen (gliding towards where it
/// settles), and when it was last drawn.
#[derive(Clone, Copy)]
struct Split {
    side: f32,
    shown: f32,
    at: f64,
}

/// Snaps a main area's width to whole columns: `(base, step)` means base + n * step; nearest or down.
fn snap_main(w: f32, snap: Option<(f32, f32)>, nearest: bool) -> f32 {
    match snap {
        None => w,
        Some((base, step)) => {
            let n = (w - base) / step;
            let n = if nearest { n.round() } else { (n + 0.001).floor() }.max(1.0);
            base + n * step
        }
    }
}

/// Java `SideSplit`: a main area on the left, a side column on the right and an 8 px gap between them; dragging the
/// gap makes the side wider or narrower (saved per view as "sideWidth.<name>"). With `snap` (base, step) the main
/// area settles only where its grid shows whole columns: while dragging at the nearest such place, and it glides
/// there. Gives the main area and the side.
pub fn side_split(
    ui: &Ui,
    p: &Palette,
    props: &lg_app::properties::Properties,
    name: &str,
    body: Rect,
    default: f32,
    min_side: f32,
    min_main: f32,
    snap: Option<(f32, f32)>,
) -> (Rect, Rect) {
    let id = split_id(name);
    let now = ui.input(|i| i.time);
    let total = body.width();
    let saved = props.get(&format!("sideWidth.{name}")).and_then(|v| v.trim().parse::<f32>().ok()).unwrap_or(default);
    let mut st = ui.ctx().data(|d| d.get_temp::<Split>(id)).unwrap_or(Split { side: saved, shown: -1.0, at: now });
    let effective = |side: f32| side.min((total - SPLIT_GAP - min_main).max(min_side)).max(min_side);
    // where the main area settles: from the side's width, snapped down to whole columns (the side takes the rest)
    let target_of = |side: f32| {
        let m = snap_main((total - effective(side) - SPLIT_GAP).max(0.0), snap, false);
        let max = total - SPLIT_GAP - min_side;
        let m = if m > max { snap_main(max, snap, false).min(max.max(0.0)) } else { m };
        m.max(0.0)
    };
    let target = target_of(st.side);
    // glide: about a third of the way every 8 ms, for however long the frame took (counted in whole 8 ms steps,
    // short frames moved nothing)
    if st.shown < 0.0 {
        st.shown = target;
    } else {
        let dt = (now - st.at).clamp(0.0, 1.0);
        let shown = st.shown + (target - st.shown) * (1.0 - 0.65f64.powf(dt / 0.008)) as f32;
        st.shown = if (target - shown).abs() < 0.5 { target } else { shown };
    }
    if (st.shown - target).abs() > 0.0 {
        ui.ctx().request_repaint();
    }
    st.at = now;
    let main_w = st.shown.round();
    let main = Rect::from_min_size(body.min, vec2(main_w, body.height()));
    let side = Rect::from_min_max(pos2(main.right() + SPLIT_GAP, body.top()), body.max);
    // the gap: a resize cursor and a thin gold line while hovered or dragged
    let gap = Rect::from_min_max(pos2(main.right(), body.top()), pos2(main.right() + SPLIT_GAP, body.bottom()));
    let resp = ui
        .interact(gap, ui.scope_id().with(("split-gap", name)), Sense::drag())
        .on_hover_cursor(egui::CursorIcon::ResizeHorizontal)
        .on_hover_text("Drag to resize the column");
    if resp.hovered() || resp.dragged() {
        ui.painter().rect_filled(
            Rect::from_min_max(pos2(gap.center().x - 1.0, gap.top()), pos2(gap.center().x + 1.0, gap.bottom())),
            0.0,
            p.gold,
        );
    }
    if resp.dragged()
        && let Some(q) = ui.input(|i| i.pointer.interact_pos())
    {
        // the nearest whole-column place to the pointer
        let m = snap_main((q.x - SPLIT_GAP / 2.0 - body.left()).max(0.0), snap, true);
        st.side = effective(total - m - SPLIT_GAP);
        ui.ctx().request_repaint();
    }
    if resp.drag_stopped() {
        crate::window::ask_later(ui.ctx(), crate::window::Ask::Set(format!("sideWidth.{name}"), format!("{}", st.side.round() as i64)));
    }
    ui.ctx().data_mut(|d| d.insert_temp(id, st));
    (main, side)
}

/// A line of a rich tooltip: pieces of text, each with its colour (None: the ink) and whether bold.
pub type TipLine = Vec<(String, Option<Color32>, bool)>;

/// A tooltip of several lines with coloured and bold parts (Java's HTML tooltips).
pub fn rich_tip(ui: &Ui, r: Rect, id: impl std::hash::Hash + std::fmt::Debug, lines: &[TipLine]) {
    if lines.is_empty() {
        return;
    }
    let resp = ui.interact(r, ui.scope_id().with(("rich-tip", id)), Sense::hover());
    let ink = ui.visuals().override_text_color.unwrap_or(Color32::WHITE);
    show_tip(ui, resp, |ui| {
        ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
        let f = text(18.0);
        for line in lines {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                for (t, c, bold) in line {
                    let color = c.unwrap_or(ink);
                    let mut rt = egui::RichText::new(t).font(f.clone()).color(color);
                    if *bold {
                        rt = rt.strong();
                    }
                    ui.label(rt);
                }
            });
        }
    });
}

/// Java `CharactersView.itemTooltip`: "Slot: name (Rarity)" and its enchantments, one per line.
pub fn item_tip_lines(data: &lg_gamedata::GameData, id: i32, enchants: Option<&[i32]>, slot: &str) -> Vec<TipLine> {
    let name = data
        .items
        .item(id)
        .filter(|i| !i.name.is_empty())
        .map_or_else(|| data.object_name(id).map_or(format!("Item {id}"), str::to_string), |i| i.name.clone());
    let mut first: TipLine = vec![(format!("{slot}:"), None, true), (format!(" {name}"), None, false)];
    let n = enchants.map_or(0, <[i32]>::len);
    let mut lines = Vec::new();
    if n > 0 {
        const RARITY: [&str; 5] = ["Common", "Uncommon", "Rare", "Legendary", "Divine"];
        first.push((format!("  ({})", RARITY[n.min(4)]), Some(crate::detail::RARITY_COLORS[n.min(4)]), false));
    }
    lines.push(first);
    for e in enchants.into_iter().flatten() {
        let t = match *e {
            -1 => "empty slot".to_string(),
            e if e < 0 => "refresh to see".to_string(),
            e => data.enchant_looks.get(&e).map_or(format!("Enchantment {e}"), |l| l.name.clone()),
        };
        lines.push(vec![(format!("  • {t}"), if *e < 0 { Some(theme::fixed::MUTED) } else { None }, false)]);
    }
    lines
}

/// An item's tooltip over `r` (Java `itemTooltip`).
pub fn item_tip(ui: &Ui, r: Rect, data: &lg_gamedata::GameData, id: i32, enchants: Option<&[i32]>, slot: &str) {
    if id > 0 {
        rich_tip(ui, r, ("item-tip", id, r.min.x as i32, r.min.y as i32), &item_tip_lines(data, id, enchants, slot));
    }
}

/// A rectangle's outline at its exact, fractional position: egui snaps rectangles to whole pixels, which makes a
/// frame that grows or moves a little each frame jump in steps. For animations only.
pub fn smooth_stroke(painter: &egui::Painter, r: Rect, radius: f32, stroke: Stroke, kind: StrokeKind) {
    painter.add(egui::epaint::RectShape::stroke(r, radius, stroke, kind).with_round_to_pixels(false));
}

/// A filled rectangle at its exact, fractional position (see [`smooth_stroke`]).
pub fn smooth_fill(painter: &egui::Painter, r: Rect, radius: f32, color: Color32) {
    painter.add(egui::epaint::RectShape::filled(r, radius, color).with_round_to_pixels(false));
}

/// Mouse wheel as in Java's scroll panes: one notch scrolls 3 lines of 16 px (Swing's unit increment, Windows'
/// 3 lines per notch) at once. egui would scroll 40 px and ease it in over about a tenth of a second, which feels
/// sluggish next to the Java app; it applies small steps (under 8 px) at once, so a notch is handed on in such steps.
pub fn java_wheel(raw: &mut egui::RawInput) {
    const PER_LINE: f32 = 3.0 * 16.0;
    let events = std::mem::take(&mut raw.events);
    for e in events {
        match e {
            egui::Event::MouseWheel { unit: egui::MouseWheelUnit::Line, delta, modifiers, phase: egui::TouchPhase::Move, source } => {
                let d = delta * PER_LINE;
                let n = (d.length() / 7.0).ceil().max(1.0);
                for _ in 0..n as usize {
                    raw.events.push(egui::Event::MouseWheel {
                        unit: egui::MouseWheelUnit::Point,
                        delta: d / n,
                        modifiers,
                        phase: egui::TouchPhase::Move,
                        source,
                    });
                }
            }
            e => raw.events.push(e),
        }
    }
}

/// A window of the app's own (Logs, Net Traffic, the new run wizard), drawn like the main window and its dialogs:
/// a title bar in the title colour with a close cross (dragged by it to move the window), the paper below, a 1 px
/// line around. It keeps its size (no growing as its content comes), centred on the screen at first. `add` fills
/// the body; false once closed.
pub fn app_window(ctx: &egui::Context, p: &Palette, id: &str, title: &str, size: egui::Vec2, add: impl FnOnce(&mut Ui)) -> bool {
    const HEAD: f32 = 38.0;
    let screen = ctx.content_rect();
    let size = vec2(size.x.min(screen.width() - 24.0), size.y.min(screen.height() - 24.0));
    let pos_id = egui::Id::unique(("lg-window-pos", id));
    let mut pos = ctx.data(|d| d.get_temp::<egui::Pos2>(pos_id)).unwrap_or_else(|| screen.center() - size / 2.0);
    // kept on the screen, its title bar at least
    pos.x = pos.x.clamp(screen.left() - size.x + 80.0, screen.right() - 80.0);
    pos.y = pos.y.clamp(screen.top(), screen.bottom() - HEAD);
    let mut open = true;
    egui::Window::new(title)
        .id(egui::Id::unique(("lg-window", id)))
        .title_bar(false)
        .frame(egui::Frame::NONE)
        .resizable(false)
        .collapsible(false)
        .movable(false)
        .fixed_size(size)
        .current_pos(pos)
        .show(ctx, |ui| {
            let (r, _) = ui.allocate_exact_size(size, Sense::hover());
            let painter = ui.painter().clone();
            painter.rect(r, 0.0, p.paper, Stroke::new(1.0, p.line), StrokeKind::Inside);
            let head = Rect::from_min_size(r.min, vec2(r.width(), HEAD));
            painter.rect_filled(head.shrink(1.0), 0.0, p.title);
            jtext::draw(&painter, pos2(head.left() + 14.0, head.center().y), Align2::LEFT_CENTER, title, &theme::head(19.0), p.title_fg);
            // the close cross, as the main window's
            let close = Rect::from_min_max(pos2(head.right() - 46.0, head.top() + 1.0), pos2(head.right() - 1.0, head.bottom()));
            let x = ui.interact(close, ui.scope_id().with("close"), Sense::click());
            if x.hovered() {
                painter.rect_filled(close, 0.0, Color32::from_rgb(0xC4, 0x2B, 0x1C));
            }
            let ink = if x.hovered() { Color32::WHITE } else { p.title_fg };
            let c = close.center();
            painter.line_segment([pos2(c.x - 7.0, c.y - 7.0), pos2(c.x + 7.0, c.y + 7.0)], Stroke::new(1.0, ink));
            painter.line_segment([pos2(c.x - 7.0, c.y + 7.0), pos2(c.x + 7.0, c.y - 7.0)], Stroke::new(1.0, ink));
            if x.clicked() {
                open = false;
            }
            // dragged by its title bar
            let bar = Rect::from_min_max(head.min, pos2(close.left(), head.bottom()));
            let drag = ui.interact(bar, ui.scope_id().with("drag"), Sense::drag());
            if drag.dragged() {
                pos += drag.drag_delta();
            }
            let body = Rect::from_min_max(pos2(r.left() + 12.0, head.bottom() + 10.0), r.max - vec2(12.0, 10.0));
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(body));
            child.set_clip_rect(body.expand(2.0));
            add(&mut child);
        });
    ctx.data_mut(|d| d.insert_temp(pos_id, pos));
    open
}

/// Where an id is kept (in the context's data) of a clickable area that is not to show the hand: the title bar.
pub const NO_HAND: &str = "lg-no-hand";

/// The hand over anything clickable: when the pointer is over a widget that takes clicks (and is enabled) and no view
/// chose a cursor this frame, the pointing hand. Called once a frame, after everything is drawn.
pub fn pointing_hand(ctx: &egui::Context) {
    if ctx.output(|o| o.cursor_icon) != egui::CursorIcon::Default {
        return;
    }
    let skip = ctx.data(|d| d.get_temp::<egui::Id>(egui::Id::unique(NO_HAND)));
    let hovered: Vec<egui::Id> = ctx.interaction_snapshot(|s| s.hovered.iter().copied().collect());
    let clickable = hovered
        .into_iter()
        .filter(|id| Some(*id) != skip)
        .any(|id| ctx.read_response(id).is_some_and(|r| r.sense.senses_click() && r.enabled()));
    if clickable {
        ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
    }
}

/// The optimization icon (Suggest Inventory Optimization, the collection's optimization button): a backpack with three
/// sparkles at its top right. `size` its height, centred on `c`.
pub fn optimize_icon(painter: &egui::Painter, c: egui::Pos2, size: f32, ink: Color32) {
    use egui::{Shape, Stroke, pos2, vec2};
    let s = size;
    let line = Stroke::new((s * 0.085).max(1.3), ink);
    let spark = |at: egui::Pos2, r: f32| {
        let t = r * 0.28;
        painter.add(Shape::convex_polygon(
            vec![at + vec2(0.0, -r), at + vec2(t, 0.0), at + vec2(0.0, r), at + vec2(-t, 0.0)],
            ink,
            Stroke::NONE,
        ));
        painter.add(Shape::convex_polygon(
            vec![at + vec2(-r, 0.0), at + vec2(0.0, -t), at + vec2(r, 0.0), at + vec2(0.0, t)],
            ink,
            Stroke::NONE,
        ));
    };
    // the backpack: its body, the pocket on its front, the flap's seam, the handle on top
    let body = Rect::from_center_size(c + vec2(-s * 0.1, s * 0.1), vec2(s * 0.56, s * 0.62));
    painter.rect_stroke(body, s * 0.13, line, egui::StrokeKind::Middle);
    painter.rect_stroke(
        Rect::from_center_size(pos2(body.center().x, body.bottom() - s * 0.15), vec2(s * 0.32, s * 0.19)),
        s * 0.05,
        line,
        egui::StrokeKind::Middle,
    );
    painter.line_segment([pos2(body.left() + s * 0.09, body.top() + s * 0.19), pos2(body.right() - s * 0.09, body.top() + s * 0.19)], line);
    let handle: Vec<egui::Pos2> = (0..=8)
        .map(|i| {
            let a = std::f32::consts::PI * (1.0 + i as f32 / 8.0);
            pos2(body.center().x + a.cos() * s * 0.12, body.top() + a.sin() * s * 0.09)
        })
        .collect();
    painter.add(Shape::line(handle, line));
    // the sparkles: a big one, two small
    spark(c + vec2(s * 0.3, -s * 0.24), s * 0.17);
    spark(c + vec2(s * 0.43, s * 0.06), s * 0.08);
    spark(c + vec2(s * 0.12, -s * 0.44), s * 0.07);
}

/// The divider between an inventory's packs (Characters' inventory): a gold line with short caps at its ends.
pub fn pack_divider(painter: &egui::Painter, x: f32, y0: f32, y1: f32, gold: Color32) {
    let line = egui::Stroke::new(2.0, Color32::from_rgba_unmultiplied(gold.r(), gold.g(), gold.b(), 170));
    painter.line_segment([egui::pos2(x, y0), egui::pos2(x, y1)], line);
    painter.line_segment([egui::pos2(x - 3.0, y0), egui::pos2(x + 3.0, y0)], line);
    painter.line_segment([egui::pos2(x - 3.0, y1), egui::pos2(x + 3.0, y1)], line);
}

/// Candidates for the optimization icon (`k`), drawn like [`optimize_icon`]; 0 is that one. For picking one.
pub fn optimize_icon_variant(painter: &egui::Painter, c: egui::Pos2, size: f32, ink: Color32, k: usize) {
    use egui::{Shape, Stroke, pos2, vec2};
    let s = size;
    let w = (s * 0.085).max(1.3);
    let line = Stroke::new(w, ink);
    let spark = |at: egui::Pos2, r: f32| {
        let t = r * 0.28;
        painter.add(Shape::convex_polygon(
            vec![at + vec2(0.0, -r), at + vec2(t, 0.0), at + vec2(0.0, r), at + vec2(-t, 0.0)],
            ink,
            Stroke::NONE,
        ));
        painter.add(Shape::convex_polygon(
            vec![at + vec2(-r, 0.0), at + vec2(0.0, -t), at + vec2(r, 0.0), at + vec2(0.0, t)],
            ink,
            Stroke::NONE,
        ));
    };
    let gem = |at: egui::Pos2, r: f32, filled: bool| {
        let pts = vec![
            at + vec2(-r, -r * 0.3),
            at + vec2(-r * 0.5, -r * 0.8),
            at + vec2(r * 0.5, -r * 0.8),
            at + vec2(r, -r * 0.3),
            at + vec2(0.0, r),
        ];
        if filled {
            painter.add(Shape::convex_polygon(pts, ink, Stroke::NONE));
        } else {
            painter.add(Shape::convex_polygon(pts, Color32::TRANSPARENT, line));
            painter.line_segment([at + vec2(-r, -r * 0.3), at + vec2(r, -r * 0.3)], line);
        }
    };
    match k {
        // 1: three sparkles, the "suggest" sign
        1 => {
            spark(c + vec2(-s * 0.08, s * 0.06), s * 0.34);
            spark(c + vec2(s * 0.3, -s * 0.28), s * 0.15);
            spark(c + vec2(s * 0.28, s * 0.32), s * 0.1);
        }
        // 2: a backpack with a sparkle
        2 => {
            let body = Rect::from_center_size(c + vec2(-s * 0.06, s * 0.08), vec2(s * 0.6, s * 0.66));
            painter.rect_stroke(body, s * 0.14, line, egui::StrokeKind::Middle);
            painter.rect_stroke(
                Rect::from_center_size(pos2(body.center().x, body.bottom() - s * 0.16), vec2(s * 0.34, s * 0.2)),
                s * 0.05,
                line,
                egui::StrokeKind::Middle,
            );
            painter.line_segment(
                [pos2(body.left() + s * 0.1, body.top() + s * 0.2), pos2(body.right() - s * 0.1, body.top() + s * 0.2)],
                line,
            );
            let handle: Vec<egui::Pos2> = (0..=8)
                .map(|i| {
                    let a = std::f32::consts::PI * (1.0 + i as f32 / 8.0);
                    pos2(body.center().x + a.cos() * s * 0.13, body.top() + a.sin() * s * 0.1)
                })
                .collect();
            painter.add(Shape::line(handle, line));
            spark(c + vec2(s * 0.34, -s * 0.3), s * 0.15);
        }
        // 3: a magnifier finding a gem
        3 => {
            let lc = c + vec2(-s * 0.08, -s * 0.08);
            painter.circle_stroke(lc, s * 0.3, Stroke::new(w * 1.2, ink));
            painter.line_segment([lc + vec2(s * 0.22, s * 0.22), c + vec2(s * 0.44, s * 0.44)], Stroke::new(w * 2.0, ink));
            gem(lc + vec2(0.0, -s * 0.02), s * 0.15, true);
        }
        // 4: a light bulb (a suggestion) with rays
        4 => {
            let bc = c + vec2(0.0, -s * 0.1);
            painter.circle_stroke(bc, s * 0.24, line);
            painter.line_segment([bc + vec2(-s * 0.12, s * 0.2), bc + vec2(-s * 0.1, s * 0.36)], line);
            painter.line_segment([bc + vec2(s * 0.12, s * 0.2), bc + vec2(s * 0.1, s * 0.36)], line);
            painter.line_segment([bc + vec2(-s * 0.11, s * 0.36), bc + vec2(s * 0.11, s * 0.36)], line);
            painter.line_segment([bc + vec2(-s * 0.08, s * 0.44), bc + vec2(s * 0.08, s * 0.44)], line);
            for a in [-2.4f32, -1.57, -0.74, -3.1, -0.04] {
                let d = vec2(a.cos(), a.sin());
                painter.line_segment([bc + d * s * 0.32, bc + d * s * 0.42], line);
            }
            spark(bc, s * 0.1);
        }
        // 5: a chest with a check
        5 => {
            let body = Rect::from_center_size(c + vec2(0.0, s * 0.12), vec2(s * 0.8, s * 0.5));
            painter.rect_stroke(body, s * 0.05, line, egui::StrokeKind::Middle);
            painter.line_segment([pos2(body.left(), body.top() + s * 0.14), pos2(body.right(), body.top() + s * 0.14)], line);
            painter.add(Shape::line(
                vec![c + vec2(-s * 0.18, s * 0.16), c + vec2(-s * 0.04, s * 0.3), c + vec2(s * 0.2, s * 0.02)],
                Stroke::new(w * 1.4, ink),
            ));
            spark(c + vec2(s * 0.3, -s * 0.32), s * 0.14);
        }
        // 6: two items trading places (arrows both ways)
        6 => {
            let (l, r) = (c + vec2(-s * 0.28, 0.0), c + vec2(s * 0.28, 0.0));
            painter.rect_stroke(
                Rect::from_center_size(l + vec2(0.0, -s * 0.2), vec2(s * 0.3, s * 0.3)),
                2.0,
                line,
                egui::StrokeKind::Middle,
            );
            gem(r + vec2(0.0, s * 0.2), s * 0.16, true);
            painter.line_segment([c + vec2(-s * 0.06, -s * 0.2), c + vec2(s * 0.3, -s * 0.2)], line);
            painter.add(Shape::convex_polygon(
                vec![c + vec2(s * 0.42, -s * 0.2), c + vec2(s * 0.28, -s * 0.3), c + vec2(s * 0.28, -s * 0.1)],
                ink,
                Stroke::NONE,
            ));
            painter.line_segment([c + vec2(s * 0.06, s * 0.2), c + vec2(-s * 0.3, s * 0.2)], line);
            painter.add(Shape::convex_polygon(
                vec![c + vec2(-s * 0.42, s * 0.2), c + vec2(-s * 0.28, s * 0.1), c + vec2(-s * 0.28, s * 0.3)],
                ink,
                Stroke::NONE,
            ));
        }
        // 7: a gem going up (an upgrade)
        7 => {
            gem(c + vec2(-s * 0.08, s * 0.12), s * 0.3, false);
            painter.line_segment([c + vec2(s * 0.34, s * 0.36), c + vec2(s * 0.34, -s * 0.3)], Stroke::new(w * 1.2, ink));
            painter.add(Shape::convex_polygon(
                vec![c + vec2(s * 0.34, -s * 0.44), c + vec2(s * 0.46, -s * 0.26), c + vec2(s * 0.22, -s * 0.26)],
                ink,
                Stroke::NONE,
            ));
            spark(c + vec2(-s * 0.3, -s * 0.34), s * 0.12);
        }
        // 8: a sack (loot bag) with a coin
        8 => {
            let sack = vec![
                c + vec2(-s * 0.12, -s * 0.24),
                c + vec2(s * 0.12, -s * 0.24),
                c + vec2(s * 0.36, s * 0.1),
                c + vec2(s * 0.3, s * 0.36),
                c + vec2(-s * 0.3, s * 0.36),
                c + vec2(-s * 0.36, s * 0.1),
            ];
            painter.add(Shape::convex_polygon(sack, Color32::TRANSPARENT, line));
            painter.add(Shape::line(vec![c + vec2(-s * 0.16, -s * 0.36), c + vec2(0.0, -s * 0.26), c + vec2(s * 0.16, -s * 0.36)], line));
            painter.line_segment([c + vec2(-s * 0.14, -s * 0.24), c + vec2(s * 0.14, -s * 0.24)], Stroke::new(w * 1.4, ink));
            painter.circle_stroke(c + vec2(0.0, s * 0.1), s * 0.12, line);
            spark(c + vec2(s * 0.34, -s * 0.34), s * 0.13);
        }
        _ => optimize_icon(painter, c, size, ink),
    }
}
