//! The window around the views (Java: `TomatoGUI.create`): the menu bar, the row of main tabs (`TabBar`), the
//! status bar (`StatusBar`). Painted as Java paints them, at the same sizes.

use crate::jtext;
use crate::theme::{self, Palette, fixed, s};
use egui::{Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};

/// The main tabs in the order the app opens with (Java: `restoreOrder`'s default).
pub const TABS: [&str; 9] = ["Profile", "Inventory", "Characters", "Game Modes", "Battles", "Social", "Guild", "Server", "Settings"];

/// Height of the menu bar, the tab row (without its bottom line) and the status bar.
const MENU_H: f32 = 44.0; // as tall as Java's FlatLaf title bar
const TAB_H: f32 = 46.0;
/// The focus line under the tabs.
const FOCUS_LINE: f32 = 4.0;
/// The focus boxes standing on it: the others, and the one picked (taller, wider, above them).
const BOX_H: f32 = 34.0;
const BOX_ON_H: f32 = 44.0;
const STATUS_H: f32 = 43.0;

/// How the update button looks: gold; dark while busy; glowing with a sliding shine when an update is there; a
/// progress bar while downloading.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateLook {
    Normal,
    Busy,
    Available,
    Downloading,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Server {
    NotConfigured,
    Offline,
    Connected,
}

/// What the frame shows and what was clicked in it.
#[derive(Debug, Clone)]
pub struct Shell {
    pub order: Vec<String>,
    pub selected: String,
    pub sniffing: bool,
    pub server: Server,
    /// The server label's tooltip.
    pub server_tip: String,
    /// The bell: its entries (title, line), whether something is new, whether its list is open; clicks this frame.
    pub notifications: Vec<(String, String)>,
    pub unread: bool,
    pub bell_open: bool,
    pub clicked_bell: bool,
    pub clicked_notification: Option<usize>,
    /// The Info menu: open, and the item picked this frame.
    pub info_open: bool,
    pub clicked_info: Option<&'static str>,
    /// Info > Newcomer's Guide's own menu (Quick tour, Extensive tour) is open.
    pub guide_menu_open: bool,
    /// Where the menus end in the menu bar (the dungeon you are in shows after them), and the bar.
    pub menu_end: f32,
    pub menu_rect: Rect,
    /// E.g. "rust stable v0.0.2" (Java: "stable" for a released app).
    pub version: String,
    pub overlays_on: bool,
    /// Clicks this frame.
    pub clicked_sniffer: bool,
    pub clicked_overlay: bool,
    pub clicked_updates: bool,
    pub clicked_server: bool,
    /// The update button (Java `UpdateButton`): what it says, its tooltip, how it looks, the download's progress.
    pub update_text: String,
    pub update_tip: String,
    pub update_look: UpdateLook,
    pub update_progress: f32,
    /// Tabs being reordered (Java `TabBar`): each tab's x as drawn (gliding to its slot), the tab dragged (its
    /// title, where it was grabbed, its x), when the last frame was, and whether a drag ended this frame.
    pub tab_x: std::collections::HashMap<String, f32>,
    pub dragged: Option<(String, f32, f32)>,
    pub tab_time: f64,
    pub order_changed: bool,
    /// The focus (see `focus.rs`): the runs that can be one, the one picked, and what was clicked this frame
    /// (`Some(None)`: the door, leaving the focus).
    pub focus_boxes: Vec<FocusBox>,
    pub focus_on: Option<i32>,
    pub clicked_focus: Option<Option<i32>>,
    /// A focus box being dragged to another place: its id and where it was grabbed (from its left edge).
    focus_drag: Option<(i32, f32)>,
    /// The boxes' new order after a drag ended (their ids), this frame.
    pub focus_order: Option<Vec<i32>>,
}

/// One focus in the row under the line: a run's character.
#[derive(Debug, Clone)]
pub struct FocusBox {
    pub id: i32,
    pub title: String,
    pub sub: String,
    pub icon: Option<egui::TextureId>,
}

impl Default for Shell {
    fn default() -> Self {
        Shell {
            order: TABS.iter().map(|t| t.to_string()).collect(),
            selected: TABS[0].into(),
            sniffing: false,
            server: Server::Offline,
            server_tip: String::new(),
            notifications: Vec::new(),
            unread: false,
            bell_open: false,
            clicked_bell: false,
            clicked_notification: None,
            info_open: false,
            clicked_info: None,
            guide_menu_open: false,
            menu_end: 0.0,
            menu_rect: Rect::NOTHING,
            version: concat!("rust stable v", env!("CARGO_PKG_VERSION")).into(),
            overlays_on: false,
            clicked_sniffer: false,
            clicked_overlay: false,
            clicked_updates: false,
            clicked_server: false,
            update_text: "CHECK UPDATES".into(),
            update_tip: String::new(),
            update_look: UpdateLook::Normal,
            update_progress: 0.0,
            tab_x: Default::default(),
            dragged: None,
            tab_time: 0.0,
            order_changed: false,
            focus_boxes: Vec::new(),
            focus_on: None,
            clicked_focus: None,
            focus_drag: None,
            focus_order: None,
        }
    }
}

fn text_width(ui: &Ui, text: &str, font: &FontId) -> f32 {
    jtext::width(ui.painter(), text, font)
}

fn text(ui: &Ui, pos: Pos2, align: Align2, text: &str, font: FontId, color: Color32) -> Rect {
    jtext::draw(ui.painter(), pos, align, text, &font, color)
}

impl Shell {
    /// The frame in the whole of `ui`; `view` fills the content area with the selected tab.
    pub fn show(&mut self, ui: &mut Ui, p: &Palette, view: impl FnOnce(&mut Ui, &str)) {
        self.clicked_sniffer = false;
        self.clicked_overlay = false;
        self.clicked_updates = false;
        self.clicked_server = false;
        self.clicked_bell = false;
        self.clicked_notification = None;
        self.clicked_info = None;
        self.clicked_focus = None;
        self.focus_order = None;
        let all = ui.max_rect();
        let painter = ui.painter().clone();
        painter.rect_filled(all, 0.0, p.sand);

        // the title bar with the menus in it (FlatLaf's: icon, menus, the title in the middle, the window's buttons);
        // dragged it moves the window, a double click maximizes it
        let menu = Rect::from_min_size(all.min, vec2(all.width(), MENU_H));
        // the tab row's colour and no line under it: Java's title bar runs straight into the tabs
        painter.rect_filled(menu, 0.0, p.sand);
        let maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
        let bar = ui.interact(menu, ui.scope_id().with("title-bar"), Sense::click_and_drag());
        // the title bar is clickable (double click) but drags the window: no hand over it
        ui.ctx().data_mut(|d| d.insert_temp(egui::Id::unique(crate::widgets::NO_HAND), bar.id));
        if bar.drag_started() {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }
        if bar.double_clicked() {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
        }
        let icon = ui.ctx().data_mut(|d| d.get_temp::<egui::TextureHandle>(egui::Id::unique("lg-title-icon"))).unwrap_or_else(|| {
            let img = image::load_from_memory(include_bytes!("../../../assets/icon/lootgobbler-32.png"))
                .map(|i| i.to_rgba8())
                .unwrap_or_default();
            let t = ui.ctx().load_texture(
                "title-icon",
                egui::ColorImage::from_rgba_unmultiplied([img.width() as usize, img.height() as usize], img.as_raw()),
                egui::TextureOptions::LINEAR,
            );
            ui.ctx().data_mut(|d| d.insert_temp(egui::Id::unique("lg-title-icon"), t.clone()));
            t
        });
        painter.image(
            icon.id(),
            Rect::from_center_size(pos2(menu.left() + 8.0 + 11.0, menu.center().y), vec2(22.0, 22.0)),
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            Color32::WHITE,
        );
        let title_font = theme::head(22.0);
        let title_w = jtext::width(&painter, "Loot Gobbler", &title_font);
        let title_r = Rect::from_center_size(menu.center(), vec2(title_w + 16.0, menu.height() - 8.0));
        let icon_r = Rect::from_center_size(pos2(menu.left() + 8.0 + 11.0, menu.center().y), vec2(30.0, 30.0));
        // the goblin and the name open About, as Info > About does
        let about_icon = ui.interact(icon_r, ui.scope_id().with("title-icon-about"), Sense::click()).on_hover_text("About Loot Gobbler");
        let about_title = ui.interact(title_r, ui.scope_id().with("title-about"), Sense::click()).on_hover_text("About Loot Gobbler");
        if about_icon.hovered() || about_title.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if about_icon.clicked() || about_title.clicked() {
            self.clicked_info = Some("About");
        }
        let title_ink = if about_title.hovered() { p.gold } else { mix(p.title_fg, p.gold, 0.5) };
        jtext::draw(&painter, menu.center(), Align2::CENTER_CENTER, "Loot Gobbler", &title_font, title_ink);
        // minimize, maximize / restore, close
        let mut bx = menu.right();
        for which in [2, 1, 0] {
            // as wide as Java's (FlatLaf's) window buttons
            let r = Rect::from_min_max(pos2(bx - 66.0, menu.top()), pos2(bx, menu.bottom()));
            bx -= 66.0;
            let resp = ui.interact(r, ui.scope_id().with(("window-button", which)), Sense::click());
            if resp.hovered() {
                let bg = if which == 2 { Color32::from_rgb(0xC4, 0x2B, 0x1C) } else { p.title_fg.gamma_multiply(0.15) };
                painter.rect_filled(r, 0.0, bg);
            }
            let ink = if resp.hovered() && which == 2 { Color32::WHITE } else { p.title_fg };
            let c = r.center();
            let st = Stroke::new(1.0, ink);
            match which {
                0 => {
                    painter.line_segment([pos2(c.x - 8.0, c.y), pos2(c.x + 8.0, c.y)], st);
                }
                1 if maximized => {
                    painter.rect_stroke(Rect::from_min_size(pos2(c.x - 8.0, c.y - 4.0), vec2(12.0, 12.0)), 0.0, st, StrokeKind::Middle);
                    painter.line_segment([pos2(c.x - 4.0, c.y - 8.0), pos2(c.x + 8.0, c.y - 8.0)], st);
                    painter.line_segment([pos2(c.x + 8.0, c.y - 8.0), pos2(c.x + 8.0, c.y + 4.0)], st);
                }
                1 => {
                    painter.rect_stroke(Rect::from_center_size(c, vec2(16.0, 16.0)), 0.0, st, StrokeKind::Middle);
                }
                _ => {
                    painter.line_segment([pos2(c.x - 8.0, c.y - 8.0), pos2(c.x + 8.0, c.y + 8.0)], st);
                    painter.line_segment([pos2(c.x - 8.0, c.y + 8.0), pos2(c.x + 8.0, c.y - 8.0)], st);
                }
            }
            if resp.clicked() {
                ui.ctx().send_viewport_cmd(match which {
                    0 => egui::ViewportCommand::Minimized(true),
                    1 => egui::ViewportCommand::Maximized(!maximized),
                    _ => egui::ViewportCommand::Close,
                });
            }
        }
        // the Info menu's button (Java's JMenu): gold while its menu is open, lighter while the pointer is on it
        let info_font = theme::body(20.0);
        let info_x = menu.left() + 8.0 + 22.0 + 6.0;
        let info_zone = Rect::from_min_max(
            pos2(info_x, menu.top()),
            pos2(info_x + 10.0 + jtext::width(&painter, "Info", &info_font) + 10.0, menu.bottom()),
        );
        let info_hover = ui.rect_contains_pointer(info_zone);
        if self.info_open {
            painter.rect_filled(info_zone, 0.0, p.gold_dark);
        } else if info_hover {
            painter.rect_filled(info_zone, 0.0, p.title_fg.gamma_multiply(0.15));
        }
        text(ui, pos2(info_x + 10.0, menu.center().y - 0.75), Align2::LEFT_CENTER, "Info", info_font, p.title_fg);
        self.menu_rect = menu;
        self.menu_end = info_zone.right();
        crate::guide::mark(ui, "menu.info", info_zone);
        if ui.interact(info_zone, ui.scope_id().with("info-menu"), Sense::click()).clicked() {
            self.info_open = !self.info_open;
            self.guide_menu_open = false;
        }
        if !self.info_open {
            self.guide_menu_open = false;
        }
        if self.info_open {
            // Java's Info menu: Logs, Release Notes, What's New Tour | Newcomer's Guide | About, Net traffic (no Java)
            const ITEMS: [(&str, bool); 8] = [
                ("Logs", true),
                ("Release Notes", true),
                ("What's New Tour", true),
                ("-", false),
                ("Newcomer's Guide", true),
                ("-", false),
                ("About", true),
                ("Net traffic", true),
            ];
            let mut guide_item = Rect::NOTHING;
            let mut guide_hovered = false;
            let mut others_hovered = false;
            // right under the Info button, at once (no fade-in), like Java's popup menu
            let area = egui::Area::new(egui::Id::unique("info-popup"))
                .order(egui::Order::Foreground)
                .fade_in(false)
                .fixed_pos(pos2(info_zone.left(), menu.bottom()))
                .show(ui.ctx(), |ui| {
                    menu_frame(p).show(ui, |ui| {
                        ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                        for (label, on) in ITEMS {
                            if label == "-" {
                                menu_separator(ui, p, MENU_W);
                                continue;
                            }
                            let sub = label == "Newcomer's Guide";
                            let lit = sub && self.guide_menu_open;
                            let resp = menu_row(ui, p, MENU_W, label, on, sub, lit);
                            let resp = if on { resp } else { resp.on_hover_text("Not in the Rust app yet") };
                            if sub {
                                // a menu of its own: it opens as the pointer comes on it
                                guide_item = resp.rect;
                                guide_hovered = resp.hovered() || resp.clicked();
                            } else if resp.hovered() {
                                others_hovered = true;
                            }
                            if on && resp.clicked() && !sub {
                                self.clicked_info = Some(label);
                            }
                        }
                    });
                });
            if guide_hovered {
                self.guide_menu_open = true;
            } else if others_hovered {
                self.guide_menu_open = false;
            }
            crate::guide::mark(ui, "menu.info.popup", area.response.rect);
            let mut sub = Rect::NOTHING;
            if self.guide_menu_open {
                let a = egui::Area::new(egui::Id::unique("guide-popup"))
                    .order(egui::Order::Foreground)
                    .fade_in(false)
                    .fixed_pos(pos2(area.response.rect.right() - 2.0, guide_item.top() - MENU_PAD))
                    .show(ui.ctx(), |ui| {
                        menu_frame(p).show(ui, |ui| {
                            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                            if menu_row(ui, p, 180.0, "Quick tour", true, false, false).clicked() {
                                self.clicked_info = Some("Quick tour");
                            }
                            menu_row(ui, p, 180.0, "Extensive tour", false, false, false).on_hover_text("Coming soon");
                        });
                    });
                sub = a.response.rect;
                crate::guide::mark(ui, "menu.guide.popup", sub);
            }
            let on_menus = ui.input(|i| i.pointer.interact_pos()).is_some_and(|q| info_zone.contains(q) || sub.contains(q));
            if self.clicked_info.is_some() || (area.response.clicked_elsewhere() && !on_menus) {
                self.info_open = false;
                self.guide_menu_open = false;
            }
        }

        // the tab row: drag a tab to reorder them; the others glide out of its way (Java `TabBar`)
        self.order_changed = false;
        // the tabs right under the title bar; under them the focus line: full gold while a focus is on, dimmed without
        let row = Rect::from_min_size(pos2(all.left(), menu.bottom()), vec2(all.width(), TAB_H));
        let line = Rect::from_min_size(pos2(all.left(), row.bottom()), vec2(all.width(), FOCUS_LINE));
        painter.rect_filled(row, 0.0, p.sand);
        let font = theme::head(17.0);
        let widths: Vec<f32> = self.order.iter().map(|t| text_width(ui, &t.to_uppercase(), &font) + s(26.0)).collect();
        let strip_w: f32 = widths.iter().sum();
        let now = ui.input(|i| i.time);
        // the glide: 28 % of the way every 15 ms (Java's timer), for however long the frame took; counted in whole
        // 15 ms steps, frames shorter than that (fast screens) moved nothing and left tabs halfway
        let dt = (now - self.tab_time).clamp(0.0, 1.0);
        let step = (1.0 - 0.72f64.powf(dt / 0.015)) as f32;
        self.tab_time = now;
        let mut target = 0.0;
        let mut moving = false;
        for (t, w) in self.order.iter().zip(&widths) {
            let x = self.tab_x.entry(t.clone()).or_insert(target);
            if self.dragged.as_ref().is_some_and(|d| d.0 == *t) {
                *x = self.dragged.as_ref().map_or(target, |d| d.2);
            } else {
                *x += (target - *x) * step;
                if (target - *x).abs() < 0.5 {
                    *x = target;
                } else {
                    moving = true;
                }
            }
            target += w;
        }
        if moving {
            ui.ctx().request_repaint();
        }
        // the dragged tab last, above the ones it passes
        let mut drawn: Vec<(String, f32)> = self.order.iter().cloned().zip(widths.iter().copied()).collect();
        if let Some(d) = &self.dragged
            && let Some(i) = drawn.iter().position(|(t, _)| *t == d.0)
        {
            let it = drawn.remove(i);
            drawn.push(it);
        }
        let mut under: Option<Rect> = None;
        for (title, w) in drawn {
            let label = title.to_uppercase();
            let x = row.left() + self.tab_x.get(&title).copied().unwrap_or(0.0);
            let r = Rect::from_min_size(pos2(x, row.top()), vec2(w, TAB_H));
            let resp = ui.interact(r, ui.scope_id().with(("tab", &title)), Sense::click_and_drag());
            if resp.drag_started()
                && let Some(q) = resp.interact_pointer_pos()
            {
                self.dragged = Some((title.clone(), q.x - x, x - row.left()));
            }
            if resp.dragged()
                && let Some(q) = ui.input(|i| i.pointer.interact_pos())
                && let Some(d) = self.dragged.as_mut().filter(|d| d.0 == title)
            {
                // it follows the mouse; past a neighbour's centre the order changes
                d.2 = (q.x - row.left() - d.1).clamp(0.0, (strip_w - w).max(0.0));
                let center = d.2 + w / 2.0;
                let mut index = 0;
                let mut tx = 0.0;
                for (t, tw) in self.order.iter().zip(&widths) {
                    if *t != title && tx + tw / 2.0 < center {
                        index += 1;
                    }
                    if *t != title {
                        tx += tw;
                    }
                }
                if let Some(i) = self.order.iter().position(|t| *t == title)
                    && i != index
                {
                    let t = self.order.remove(i);
                    self.order.insert(index.min(self.order.len()), t);
                }
                ui.ctx().set_cursor_icon(egui::CursorIcon::Move);
                ui.ctx().request_repaint();
            }
            if resp.drag_stopped() {
                self.dragged = None; // glides into its slot
                self.order_changed = true;
                ui.ctx().request_repaint();
            }
            let on = title == self.selected;
            let bg = if on {
                p.paper
            } else if resp.hovered() {
                p.header
            } else {
                p.sand
            };
            painter.rect_filled(r, 0.0, bg);
            let underline = s(3.0);
            if on {
                under = Some(r);
            }
            painter.rect_filled(Rect::from_min_max(pos2(r.right() - 1.0, r.top()), r.max), 0.0, Color32::BLACK);
            let (tw, th) = (jtext::width(&painter, &label, &font), jtext::height(&painter, &font));
            let top = r.top() + ((TAB_H - underline - th) / 2.0).floor();
            let left = r.left() + ((w - 1.0 - tw) / 2.0).floor();
            jtext::draw(&painter, pos2(left, top), Align2::LEFT_TOP, &label, &font, if on { p.gold } else { p.ink_soft });
            if resp.clicked() {
                self.selected = title.clone();
            }
        }
        // the selected tab's bar: round-ended, gliding over to the tab clicked
        if let Some(r) = under {
            let (h, inset) = (s(4.0), s(6.0));
            let ctx = ui.ctx();
            let l = ctx.animate_value_with_time(ui.scope_id().with("tab-bar-l"), r.left() + inset, 0.18);
            let rr = ctx.animate_value_with_time(ui.scope_id().with("tab-bar-r"), r.right() - 1.0 - inset, 0.18);
            // with a focus on it stands on the (lit) focus line, its top corners rounded down into it; without, a pill
            // just above the line
            let focused = self.focus_on.is_some_and(|id| self.focus_boxes.iter().any(|b| b.id == id));
            let t = ctx.animate_bool_with_time(ui.scope_id().with("tab-bar-focus"), focused, 0.18);
            let gap = s(1.0) * (1.0 - t);
            if t < 0.5 {
                let bar = Rect::from_min_max(pos2(l, r.bottom() - h - gap), pos2(rr.max(l + h), r.bottom() - gap));
                painter.rect_filled(bar, h / 2.0, p.gold);
            } else {
                // the top half of a pill, cut along its middle and sitting on the focus line: a quarter circle at
                // each end, flat on the line
                let (rad, base) = (s(3.5), r.bottom());
                let rr = rr.max(l + 2.0 * rad);
                let mut pts = Vec::new();
                const STEPS: usize = 8;
                for k in 0..=STEPS {
                    let a = std::f32::consts::PI + std::f32::consts::FRAC_PI_2 * k as f32 / STEPS as f32;
                    pts.push(pos2(l + rad + rad * a.cos(), base + rad * a.sin()));
                }
                for k in 0..=STEPS {
                    let a = std::f32::consts::PI * 1.5 + std::f32::consts::FRAC_PI_2 * k as f32 / STEPS as f32;
                    pts.push(pos2(rr - rad + rad * a.cos(), base + rad * a.sin()));
                }
                painter.add(egui::Shape::convex_polygon(pts, p.gold, Stroke::NONE));
            }
        }
        // the notifications bell at the right end of the tab row
        let bell = Rect::from_min_size(pos2(row.right() - 36.0, row.top()), vec2(36.0, TAB_H));
        let bell_resp = ui.interact(bell, ui.scope_id().with("bell"), Sense::click());
        let hover = bell_resp.hovered();
        if bell_resp.clicked() {
            self.bell_open = !self.bell_open;
            self.clicked_bell = self.bell_open;
        }
        let bell_resp = bell_resp.on_hover_text("Notifications");
        let _ = bell_resp;
        paint_bell(&painter, bell.center(), if hover { p.ink } else { p.ink.gamma_multiply(190.0 / 255.0) });
        if self.unread {
            // a ring in the background colour sets the dot off the bell
            let (x, y) = ((bell.center().x + 3.0).round(), (bell.center().y - 11.0).round());
            painter.circle_filled(pos2(x + 4.0, y + 4.0), 6.0, p.sand);
            painter.circle_filled(pos2(x + 4.0, y + 4.0), 4.0, Color32::from_rgb(0xE5, 0x48, 0x4D));
        }
        self.focus_row(ui, p, line, bell.left() - 6.0);
        if self.bell_open {
            // Java's popup menu under the bell, its right edge at the bell's
            let area = egui::Area::new(egui::Id::unique("bell-menu"))
                .order(egui::Order::Foreground)
                .pivot(Align2::RIGHT_TOP)
                .fixed_pos(pos2(bell.right(), bell.bottom()))
                .show(ui.ctx(), |ui| {
                    egui::Frame::NONE.fill(p.paper).stroke(Stroke::new(1.0, p.line)).inner_margin(4.0).show(ui, |ui| {
                        let f = theme::body(18.0);
                        if self.notifications.is_empty() {
                            ui.add_space(6.0);
                            ui.horizontal(|ui| {
                                ui.add_space(10.0);
                                ui.label(egui::RichText::new("No notifications").font(f.clone()).color(fixed::MUTED));
                                ui.add_space(10.0);
                            });
                            ui.add_space(6.0);
                        }
                        for (i, (title, text)) in self.notifications.iter().enumerate() {
                            let resp =
                                egui::Frame::NONE.inner_margin(egui::Margin { left: 8, right: 12, top: 6, bottom: 6 }).show(ui, |ui| {
                                    ui.label(egui::RichText::new(title).font(f.clone()).strong().color(p.ink));
                                    ui.label(egui::RichText::new(text).font(f.clone()).color(Color32::from_rgb(0x9A, 0x9A, 0x9A)));
                                });
                            let r = resp.response.rect;
                            let click = ui.interact(r, ui.scope_id().with(("bell-item", i)), Sense::click());
                            if click.hovered() {
                                ui.painter().rect_filled(r, 0.0, p.sand.gamma_multiply(0.35));
                            }
                            if click.clicked() {
                                self.clicked_notification = Some(i);
                            }
                        }
                    });
                });
            if self.clicked_notification.is_some() || (area.response.clicked_elsewhere() && !bell_resp_contains(ui, bell)) {
                self.bell_open = false;
            }
        }

        // the status bar
        let bar = Rect::from_min_max(pos2(all.left(), all.bottom() - STATUS_H), all.max);
        painter.rect_filled(bar, 0.0, p.line);
        let label = theme::head(16.0);
        let mut lx = bar.left() + s(8.0);
        let square = |x: f32, c: Color32| {
            painter.rect_filled(Rect::from_min_size(pos2(x, bar.center().y - 5.0), vec2(10.0, 10.0)), 0.0, c);
        };
        square(lx, if self.sniffing { fixed::GREEN } else { fixed::STONE });
        lx += 10.0 + s(8.0);
        let r = text(
            ui,
            pos2(lx, bar.center().y),
            Align2::LEFT_CENTER,
            if self.sniffing { "SNIFFING" } else { "SNIFFER OFF" },
            label.clone(),
            p.ink,
        );
        lx = r.right() + s(8.0) + 24.0;
        let (color, server) = match self.server {
            Server::NotConfigured => (fixed::STONE, "NO SERVER CONFIGURED YET"),
            Server::Offline => (fixed::RED, "NO SERVER CONNECTION"),
            Server::Connected => (fixed::GREEN, "CONNECTED"),
        };
        square(lx, color);
        let r = text(ui, pos2(lx + 10.0 + s(8.0), bar.center().y), Align2::LEFT_CENTER, server, label.clone(), p.ink);
        let zone = Rect::from_min_max(pos2(lx, bar.top()), pos2(r.right(), bar.bottom()));
        crate::widgets::tip(ui, zone, "server-tip", &self.server_tip);
        if ui.interact(zone, ui.scope_id().with("server"), Sense::click()).clicked() {
            self.clicked_server = true;
        }

        let mut rx = bar.right();
        let buttons = [
            (if self.sniffing { "STOP SNIFFER" } else { "START SNIFFER" }, 0),
            (if self.overlays_on { "HIDE OVERLAY" } else { "SHOW OVERLAY" }, 1),
            (self.update_text.as_str(), 2),
        ];
        for (i, (t, which)) in buttons.iter().enumerate() {
            if i > 0 {
                rx -= 6.0;
            }
            // the update button keeps the width of its longest text
            let widest = if *which == 2 { "UPDATE AVAILABLE" } else { t };
            let w = text_width(ui, widest, &label) + 2.0 * s(10.0) + 2.0;
            let r = Rect::from_min_max(pos2(rx - w, bar.top() + 3.0), pos2(rx, bar.bottom() - 3.0));
            let resp = ui.interact(r, ui.scope_id().with(("status", which)), Sense::click());
            let resp = match which {
                1 => resp.on_hover_text("Hides the overlays that are on, or shows them again (each is set up under Battles > Overlay)"),
                2 => resp.on_hover_text(if self.update_tip.is_empty() {
                    "Check for a new Loot Gobbler version"
                } else {
                    self.update_tip.as_str()
                }),
                _ => resp,
            };
            if *which == 0 {
                crate::guide::mark(ui, "sniffer", r);
            }
            let fill = if resp.is_pointer_button_down_on() {
                p.gold_dark
            } else if resp.hovered() {
                p.gold_hover
            } else {
                p.gold
            };
            if *which == 2 && self.update_look != UpdateLook::Normal {
                update_button(ui, p, r, t, &label, self.update_look, self.update_progress, resp.is_pointer_button_down_on());
            } else {
                painter.rect(r, 0.0, fill, Stroke::new(1.0, Color32::BLACK), StrokeKind::Inside);
                text(ui, r.center(), Align2::CENTER_CENTER, t, label.clone(), p.on_accent);
            }
            if resp.clicked() {
                match which {
                    0 => self.clicked_sniffer = true,
                    1 => self.clicked_overlay = true,
                    _ => self.clicked_updates = true,
                }
            }
            rx -= w;
        }
        let m = fixed::MUTED;
        let vr = text(
            ui,
            pos2(rx - 12.0, bar.center().y),
            Align2::RIGHT_CENTER,
            &self.version,
            label,
            Color32::from_rgba_unmultiplied(m.r(), m.g(), m.b(), 110),
        );
        // a build from source until the app is released (Java `stable`)
        crate::widgets::tip(ui, vr, "version-tip", "Development build (run from source, not a published release)");

        let content = Rect::from_min_max(pos2(all.left(), line.bottom()), pos2(all.right(), bar.top()));
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(content));
        child.set_clip_rect(content);
        view(&mut child, &self.selected.clone());
        // without the system's frame: the edges and corners resize the window (above everything else)
        if !ui.input(|i| i.viewport().maximized.unwrap_or(false)) {
            use egui::{CursorIcon as C, ResizeDirection as D};
            let (e, k) = (5.0, 12.0);
            let zones = [
                (Rect::from_min_max(all.min, pos2(all.left() + k, all.top() + k)), D::NorthWest, C::ResizeNwSe),
                (Rect::from_min_max(pos2(all.right() - k, all.top()), pos2(all.right(), all.top() + k)), D::NorthEast, C::ResizeNeSw),
                (Rect::from_min_max(pos2(all.left(), all.bottom() - k), pos2(all.left() + k, all.bottom())), D::SouthWest, C::ResizeNeSw),
                (Rect::from_min_max(pos2(all.right() - k, all.bottom() - k), all.max), D::SouthEast, C::ResizeNwSe),
                (Rect::from_min_max(all.min, pos2(all.right(), all.top() + e)), D::North, C::ResizeVertical),
                (Rect::from_min_max(pos2(all.left(), all.bottom() - e), all.max), D::South, C::ResizeVertical),
                (Rect::from_min_max(all.min, pos2(all.left() + e, all.bottom())), D::West, C::ResizeHorizontal),
                (Rect::from_min_max(pos2(all.right() - e, all.top()), all.max), D::East, C::ResizeHorizontal),
            ];
            let layer = egui::LayerId::new(egui::Order::Foreground, egui::Id::unique("lg-resize"));
            let edges = ui.new_child(egui::UiBuilder::new().layer_id(layer).max_rect(all));
            for (i, (r, dir, cursor)) in zones.into_iter().enumerate() {
                let resp = edges.interact(r, edges.scope_id().with(("resize", i)), Sense::drag()).on_hover_cursor(cursor);
                if resp.drag_started() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::BeginResize(dir));
                }
            }
        }
    }
}

/// Between two colours (`t` 0 is `a`).
fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_premultiplied(m(a.r(), b.r()), m(a.g(), b.g()), m(a.b(), b.b()), m(a.a(), b.a()))
}

/// A box standing on the focus line: square at the bottom (cut at `base`), rounded above; with `shadow` it is raised.
fn standing(painter: &egui::Painter, r: Rect, base: f32, fill: Color32, stroke: Stroke, shadow: f32) {
    let clip = painter.with_clip_rect(Rect::from_min_max(pos2(r.left() - 12.0, r.top() - 12.0), pos2(r.right() + 12.0, base)));
    let round = egui::CornerRadius { nw: 8, ne: 8, sw: 0, se: 0 };
    let shape = Rect::from_min_max(r.min, pos2(r.right(), base + 10.0));
    for i in (1..=6).rev() {
        let a = (shadow * 70.0 * (1.0 - (i - 1) as f32 / 6.0)).round() as u8;
        if a > 0 {
            clip.rect_filled(shape.translate(vec2(0.0, -2.0)).expand(i as f32), round, Color32::from_black_alpha(a / 3));
        }
    }
    clip.rect(shape, round, fill, stroke, StrokeKind::Inside);
}

/// A door: closed, or open (its leaf swung towards you).
fn paint_door(painter: &egui::Painter, c: Pos2, open: bool, ink: Color32, fill: Color32) {
    let frame = Rect::from_center_size(c, vec2(13.0, 19.0));
    let st = Stroke::new(1.6, ink);
    painter.line_segment([pos2(frame.left() - 3.0, frame.bottom()), pos2(frame.right() + 3.0, frame.bottom())], st);
    if open {
        painter.rect_filled(frame.shrink(1.0), 0.0, Color32::from_black_alpha(140));
        painter.rect_stroke(frame, 0.0, st, StrokeKind::Middle);
        let leaf = vec![
            frame.left_top(),
            pos2(frame.left() + 8.0, frame.top() + 3.0),
            pos2(frame.left() + 8.0, frame.bottom() + 2.0),
            frame.left_bottom(),
        ];
        painter.add(egui::Shape::convex_polygon(leaf, fill, st));
        painter.circle_filled(pos2(frame.left() + 5.5, c.y + 1.0), 1.1, ink);
    } else {
        painter.rect(frame, 0.0, fill, st, StrokeKind::Middle);
        painter.circle_filled(pos2(frame.right() - 3.5, c.y + 1.0), 1.2, ink);
    }
}

impl Shell {
    /// The focus line, and standing on it before `right` (the bell) the focus boxes, the door tucked behind the one
    /// picked (the first without a focus); the one picked taller, wider and above the others.
    fn focus_row(&mut self, ui: &Ui, p: &Palette, line: Rect, right: f32) {
        const OVERLAP: f32 = 8.0;
        const DOOR_VIS: f32 = 34.0;
        const TUCK: f32 = 14.0;
        let painter = ui.painter().clone();
        let ctx = ui.ctx().clone();
        let on = self.focus_on.filter(|id| self.focus_boxes.iter().any(|b| b.id == *id));
        let lit = ctx.animate_bool_with_time(ui.scope_id().with("focus-line"), on.is_some(), 0.2);
        painter.rect_filled(line, 0.0, mix(p.gold.gamma_multiply(0.28), p.gold, lit));
        if lit > 0.0 {
            // its glow, under it
            for i in 0..4 {
                let a = 0.22 * lit * (1.0 - i as f32 / 4.0);
                painter.rect_filled(
                    Rect::from_min_size(pos2(line.left(), line.bottom() + i as f32), vec2(line.width(), 1.0)),
                    0.0,
                    p.gold.gamma_multiply(a),
                );
            }
        }
        if self.focus_boxes.is_empty() {
            return;
        }
        // the boxes stand on the line (their bottom at its top)
        let base = line.top();
        // each box: how far it is the one picked (animated), its sizes and width
        struct Laid {
            t: f32,
            icon: f32,
            title: FontId,
            sub: FontId,
            w: f32,
        }
        let laid: Vec<Laid> = self
            .focus_boxes
            .iter()
            .map(|b| {
                // eased in and out; the width blended between the two ends' (measuring every in-between font size made
                // it twitch), the text growing smoothly
                let raw = ctx.animate_bool_with_time(ui.scope_id().with(("focus-box", b.id)), on == Some(b.id), 0.22);
                let t = raw * raw * (3.0 - 2.0 * raw);
                let width = |title: &FontId, sub: &FontId, icon: f32, extra: f32| {
                    6.0 + icon + 8.0 + jtext::width(&painter, &b.title, title).max(jtext::width(&painter, &b.sub, sub)) + 12.0 + extra
                };
                let (w0, w1) =
                    (width(&theme::body(17.0), &theme::body(13.0), 26.0, 0.0), width(&theme::body(20.0), &theme::body(15.0), 36.0, 4.0));
                let (title, sub) = (theme::body(17.0 + 3.0 * t), theme::body(13.0 + 2.0 * t));
                let icon = 26.0 + 10.0 * t;
                Laid { t, icon, w: w0 + (w1 - w0) * t, title, sub }
            })
            .collect();
        // all at the right end, the door at their left; a box dragged follows the pointer, the others make room
        let total = DOOR_VIS + laid.iter().map(|l| l.w).sum::<f32>() - OVERLAP * (laid.len() - 1) as f32;
        let start = right - total;
        let door_x = start;
        let mut seq: Vec<usize> = (0..laid.len()).collect();
        let pointer = ui.input(|i| i.pointer.interact_pos());
        let dragged = self.focus_drag.and_then(|(id, grab)| self.focus_boxes.iter().position(|b| b.id == id).map(|i| (i, grab)));
        let mut drag_left = None;
        if let (Some((di, grab)), Some(q)) = (dragged, pointer) {
            let left = (q.x - grab).clamp(start + DOOR_VIS - laid[di].w / 2.0, right - laid[di].w / 2.0);
            drag_left = Some(left);
            // its new place: before the first other box whose middle is right of its own middle
            seq.retain(|i| *i != di);
            let mid = left + laid[di].w / 2.0;
            let mut x = start + DOOR_VIS;
            let mut k = seq.len();
            for (n, &i) in seq.iter().enumerate() {
                if x + laid[i].w / 2.0 > mid {
                    k = n;
                    break;
                }
                x += laid[i].w - OVERLAP;
            }
            seq.insert(k, di);
        }
        let mut xs = vec![0.0; laid.len()];
        let mut x = start + DOOR_VIS;
        for &i in &seq {
            // each right after the ones before as wide as they are now, so place and size change together (one
            // diagonal move); the one dragged right under the pointer
            xs[i] = match (dragged, drag_left) {
                (Some((di, _)), Some(left)) if di == i => left,
                _ => x,
            };
            x += laid[i].w - OVERLAP;
        }
        // the door, behind them all: its visible part left of the box it belongs to, the icon in its middle
        let door_r = Rect::from_min_max(pos2(door_x, base - BOX_H), pos2(door_x + DOOR_VIS + TUCK, base));
        let door_resp = ui
            .interact(
                Rect::from_min_max(pos2(door_x, base - BOX_H), pos2(door_x + DOOR_VIS, base)),
                ui.scope_id().with("focus-door"),
                Sense::click(),
            )
            .on_hover_text(if on.is_some() { "Leave the focus: everything shown as before" } else { "No focus: everything is shown" });
        let door_ink = if on.is_some() { if door_resp.hovered() { p.gold_hover } else { p.gold } } else { fixed::MUTED };
        let door_edge = if on.is_some() { p.gold } else { p.gold.gamma_multiply(0.45) };
        standing(&painter, door_r, base, mix(p.sand, p.header, 0.35), Stroke::new(1.5, door_edge), 0.0);
        paint_door(&painter, pos2(door_x + DOOR_VIS / 2.0, base - BOX_H / 2.0), on.is_some(), door_ink, p.header);
        if on.is_some() && door_resp.clicked() {
            self.clicked_focus = Some(None);
        }
        // the boxes left to right, each over the one before; the one picked above them, the one dragged on top
        let drag_i = dragged.map(|(i, _)| i);
        let mut order: Vec<usize> = seq.iter().copied().filter(|i| on != Some(self.focus_boxes[*i].id) && Some(*i) != drag_i).collect();
        order.extend(seq.iter().copied().filter(|i| on == Some(self.focus_boxes[*i].id) && Some(*i) != drag_i));
        order.extend(drag_i);
        let ids: Vec<i32> = seq.iter().map(|i| self.focus_boxes[*i].id).collect();
        for i in order {
            let (b, l) = (&self.focus_boxes[i], &laid[i]);
            let h = BOX_H + (BOX_ON_H - BOX_H) * l.t;
            let r = Rect::from_min_size(pos2(xs[i], base - h), vec2(l.w, h));
            let picked = on == Some(b.id);
            let resp = ui
                .interact(r, ui.scope_id().with(("focus-box-click", b.id)), Sense::click_and_drag())
                .on_hover_cursor(egui::CursorIcon::PointingHand);
            // drag it sideways to give it another place in the row
            if resp.drag_started()
                && let Some(q) = pointer
            {
                self.focus_drag = Some((b.id, q.x - r.left()));
            }
            if resp.dragged() {
                ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
                ctx.request_repaint();
            }
            if resp.drag_stopped() && self.focus_drag.is_some_and(|(id, _)| id == b.id) {
                self.focus_drag = None;
                self.focus_order = Some(ids.clone());
            }
            let resp = if picked {
                resp.on_hover_text("The focus (drag to move it)")
            } else {
                resp.on_hover_text(format!("Focus on this {} (drag to move it)", b.sub))
            };
            // every box with the gold outline; the one picked thicker (below), a hovered one lit
            let edge = if resp.hovered() && l.t < 0.5 { p.gold_hover } else { p.gold };
            standing(&painter, r, base, mix(mix(p.sand, p.header, 0.5), p.header, l.t), Stroke::new(1.5 + 1.0 * l.t, edge), l.t);
            let ir = Rect::from_min_size(pos2(r.left() + 6.0, r.center().y - l.icon / 2.0), vec2(l.icon, l.icon));
            if let Some(t) = b.icon {
                painter.image(
                    t,
                    ir,
                    Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                    Color32::from_white_alpha((190.0 + 65.0 * l.t) as u8),
                );
            }
            let tx = ir.right() + 8.0;
            let (th, sh) = (jtext::height(&painter, &l.title), jtext::height(&painter, &l.sub));
            let ty = r.center().y - (th + sh) / 2.0;
            jtext::draw(&painter, pos2(tx, ty), Align2::LEFT_TOP, &b.title, &l.title, mix(p.ink_soft, p.ink, l.t));
            jtext::draw(&painter, pos2(tx, ty + th), Align2::LEFT_TOP, &b.sub, &l.sub, mix(fixed::MUTED, p.gold, l.t));
            if resp.clicked() && !picked {
                self.clicked_focus = Some(Some(b.id));
            }
        }
    }
}

/// Java `UpdateButton.paintComponent` and the status bar's glow: dark while busy; hover-gold with a pulsing glow
/// around it and a glint sliding across when an update is available; a gold progress fill while downloading.
fn update_button(ui: &Ui, p: &Palette, r: Rect, t: &str, font: &FontId, look: UpdateLook, progress: f32, pressed: bool) {
    let painter = ui.painter();
    let time = ui.input(|i| i.time);
    match look {
        UpdateLook::Busy => {
            painter.rect(r, 0.0, p.gold_dark, Stroke::new(1.0, Color32::BLACK), StrokeKind::Inside);
            text(ui, r.center(), Align2::CENTER_CENTER, t, font.clone(), p.on_accent);
        }
        UpdateLook::Downloading => {
            painter.rect_filled(r, 0.0, p.header);
            text(ui, r.center(), Align2::CENTER_CENTER, t, font.clone(), p.ink);
            let fill = Rect::from_min_size(r.min, vec2((r.width() * progress.clamp(0.0, 1.0)).round(), r.height()));
            painter.rect_filled(fill, 0.0, p.gold);
            // the text dark on the gold part
            jtext::draw(&painter.with_clip_rect(fill), r.center(), Align2::CENTER_CENTER, t, font, p.on_accent);
            painter.rect_stroke(r, 0.0, Stroke::new(1.0, Color32::BLACK), StrokeKind::Inside);
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
        }
        _ => {
            // the glow: rings from the outside in, strongest next to the button, one slow pulse every 1.6 s
            let pulse = 0.35 + 0.65 * (0.5 - 0.5 * (std::f64::consts::TAU * time / 1.6).cos()) as f32;
            let g = p.gold_hover;
            for i in (1..=7).rev() {
                let falloff = 1.0 - (i - 1) as f32 / 7.0;
                let a = (255.0 * pulse * 0.45 * falloff * falloff).round() as u8;
                painter.rect_stroke(
                    r.expand(i as f32 - 0.5),
                    0.0,
                    Stroke::new(1.0, Color32::from_rgba_unmultiplied(g.r(), g.g(), g.b(), a)),
                    StrokeKind::Middle,
                );
            }
            painter.rect(r, 0.0, if pressed { p.gold_dark } else { p.gold_hover }, Stroke::new(1.0, Color32::BLACK), StrokeKind::Inside);
            text(ui, r.center(), Align2::CENTER_CENTER, t, font.clone(), p.on_accent);
            // a glint sliding across every 3.2 s (resting the first half of each cycle)
            let c = (time % 3.2) / 3.2;
            if (0.55..=0.85).contains(&c) {
                let q = (c - 0.55) / 0.30;
                let q = if q < 0.5 { 2.0 * q * q } else { 1.0 - 2.0 * (1.0 - q) * (1.0 - q) } as f32;
                let (band, slant) = (22.0, r.height() / 3.0);
                let x = r.left() - band - slant + q * (r.width() + 2.0 * band + 2.0 * slant);
                let light = Color32::from_rgba_unmultiplied(255, 250, 225, 215);
                let clear = Color32::from_rgba_unmultiplied(255, 250, 225, 0);
                let clip = painter.with_clip_rect(r);
                let mut mesh = egui::Mesh::default();
                for (dx, col) in [(0.0, clear), (band * 0.45, light), (band * 0.55, light), (band, clear)] {
                    mesh.colored_vertex(pos2(x + dx + slant, r.top()), col);
                    mesh.colored_vertex(pos2(x + dx - slant, r.bottom()), col);
                }
                for k in 0..3u32 {
                    let i = 2 * k;
                    mesh.add_triangle(i, i + 1, i + 2);
                    mesh.add_triangle(i + 1, i + 3, i + 2);
                }
                clip.add(egui::Shape::mesh(mesh));
            }
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(40));
        }
    }
}

/// Whether the pointer is on the bell (its own click toggles the list).
fn bell_resp_contains(ui: &Ui, bell: Rect) -> bool {
    ui.input(|i| i.pointer.interact_pos()).is_some_and(|p| bell.contains(p))
}

/// Java `NotificationBell`: a dome widening to a flared rim, the clapper below, a knob on top.
fn paint_bell(painter: &egui::Painter, c: Pos2, color: Color32) {
    use egui::epaint::{CubicBezierShape, PathShape};
    let pt = |x: f32, y: f32| pos2(c.x + x, c.y + y);
    let curves = [
        [pt(-7.0, 4.0), pt(-6.0, 1.0), pt(-6.0, -1.0), pt(-6.0, -3.0)],
        [pt(-6.0, -3.0), pt(-6.0, -7.0), pt(-3.5, -9.0), pt(0.0, -9.0)],
        [pt(0.0, -9.0), pt(3.5, -9.0), pt(6.0, -7.0), pt(6.0, -3.0)],
        [pt(6.0, -3.0), pt(6.0, -1.0), pt(6.0, 1.0), pt(7.0, 4.0)],
    ];
    let mut points = Vec::new();
    for b in curves {
        let shape = CubicBezierShape::from_points_stroke(b, false, Color32::TRANSPARENT, Stroke::NONE);
        points.extend(shape.flatten(Some(0.2)));
    }
    painter.add(PathShape::convex_polygon(points, color, Stroke::NONE));
    painter.rect_filled(Rect::from_min_size(pt(-8.5, 3.5), vec2(17.0, 2.5)), 1.0, color);
    painter.add(egui::Shape::ellipse_filled(pt(0.0, 8.5), vec2(2.5, 2.0), color));
    painter.circle_filled(pt(0.0, -9.5), 1.5, color);
}

/// A popup menu's width (the Info menu), its rows' height and the space around them (Java's FlatLaf menus).
const MENU_W: f32 = 200.0;
const MENU_ROW_H: f32 = 36.0;
const MENU_PAD: f32 = 5.0;

fn menu_frame(p: &Palette) -> egui::Frame {
    egui::Frame::NONE
        .fill(p.paper)
        .stroke(Stroke::new(1.0, p.line))
        .corner_radius(4.0)
        .inner_margin(egui::Margin::symmetric(0, MENU_PAD as i8))
}

/// One row of a popup menu: gold under the pointer (or while its submenu is open), the label inset from the left,
/// a thin chevron for a submenu; greyed out when not available.
fn menu_row(ui: &mut Ui, p: &Palette, w: f32, label: &str, on: bool, sub: bool, lit: bool) -> egui::Response {
    let (r, resp) = ui.allocate_exact_size(vec2(w, MENU_ROW_H), if on { Sense::click() } else { Sense::hover() });
    let painter = ui.painter();
    let gold = on && (lit || resp.hovered());
    if gold {
        painter.rect_filled(r.shrink2(vec2(MENU_PAD, 0.0)), 3.0, p.gold);
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    let ink = if !on {
        fixed::MUTED
    } else if gold {
        p.on_accent
    } else {
        p.ink
    };
    jtext::draw(painter, pos2(r.left() + 28.0, r.center().y), Align2::LEFT_CENTER, label, &theme::body(18.0), ink);
    if sub {
        let c = pos2(r.right() - 18.0, r.center().y);
        let st = Stroke::new(1.5, ink);
        painter.line_segment([pos2(c.x - 3.0, c.y - 6.0), pos2(c.x + 3.0, c.y)], st);
        painter.line_segment([pos2(c.x + 3.0, c.y), pos2(c.x - 3.0, c.y + 6.0)], st);
    }
    resp
}

fn menu_separator(ui: &mut Ui, p: &Palette, w: f32) {
    let (r, _) = ui.allocate_exact_size(vec2(w, 11.0), Sense::hover());
    ui.painter().line_segment([pos2(r.left(), r.center().y), pos2(r.right(), r.center().y)], Stroke::new(1.0, p.line));
}
