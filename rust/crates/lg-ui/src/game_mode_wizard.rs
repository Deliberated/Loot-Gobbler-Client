//! Starts a game mode run on a new character, step by step (Java `GameModeWizard`): pick the mode (PPE or a UPE
//! variant; the guild version; without a pet), then create the character in-game as the mode asks (sniffer running,
//! upgrades disabled, no pet) while the wizard waits for it, then the run starts by itself. A character created the
//! wrong way is named, and the wizard keeps waiting for the next one.

use crate::icons::Icons;
use crate::jtext;
use crate::mode_badge;
use crate::theme::{Palette, fixed, mix};
use crate::widgets::text;
use crate::window::Ask;
use egui::{Align2, Color32, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};
use lg_app::upe::{self, CATEGORY_INFO, UPE_VARIANTS};
use std::collections::HashSet;

#[derive(Default)]
pub struct Wizard {
    step: u8,
    category: String,
    variant: String,
    guild: bool,
    no_pet: bool,
    /// The characters there were when step 2 began, and the new ones already judged.
    before: HashSet<i32>,
    told: HashSet<i32>,
    problem: Option<String>,
    /// Step 3: the character whose run started, and its mode.
    started: Option<(i32, String)>,
    /// The characters there are now are to be noted (step 2 began); the run is to be marked (step 3 began).
    before_pending: bool,
    mark: bool,
}

/// What the wizard asks of the window.
pub enum Out {
    Keep,
    Close,
    Asks(Vec<Ask>),
}

fn gold(p: &Palette) -> Color32 {
    p.gold
}

/// Words wrapped at `w`; gives their height.
fn words(ui: &mut Ui, s: &str, size: f32, color: Color32, w: f32) {
    ui.allocate_ui(vec2(w, 0.0), |ui| {
        ui.add(egui::Label::new(egui::RichText::new(s).font(text(size)).color(color)).wrap());
    });
}

/// A button as FlatLaf draws them (primary: gold); true when clicked.
fn button(ui: &mut Ui, p: &Palette, label: &str, primary: bool) -> bool {
    let f = crate::widgets::small();
    let t = label.to_uppercase();
    let w = jtext::width(ui.painter(), &t, &f) + 32.0;
    let (r, resp) = ui.allocate_exact_size(vec2(w, 36.0), Sense::click());
    let (fill, ink) = if primary {
        (if resp.hovered() { p.gold_hover } else { p.gold }, p.on_accent)
    } else {
        (if resp.hovered() { p.sand } else { p.header }, p.ink)
    };
    ui.painter().rect(r, 4.0, fill, Stroke::new(1.0, Color32::BLACK), StrokeKind::Inside);
    jtext::draw(ui.painter(), r.center(), Align2::CENTER_CENTER, &t, &f, ink);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
}

/// Buttons at the right end of a row.
fn buttons(ui: &mut Ui, f: impl FnOnce(&mut Ui)) {
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), f);
}

/// A gold circle with the step's number.
fn number(ui: &mut Ui, p: &Palette, n: usize) {
    let (r, _) = ui.allocate_exact_size(vec2(34.0, 34.0), Sense::hover());
    ui.painter().circle_filled(r.center(), 15.5, gold(p));
    jtext::draw_bold(ui.painter(), r.center(), Align2::CENTER_CENTER, &n.to_string(), &text(17.0), Color32::from_rgb(0x2A, 0x1E, 0x0A));
}

/// A game-like switch, shown OFF: what to set when creating the character.
fn switch_off(ui: &mut Ui, label: &str) {
    let (r, _) = ui.allocate_exact_size(vec2(190.0, 46.0), Sense::hover());
    let painter = ui.painter();
    let plate = Rect::from_min_max(pos2(r.left(), r.top() + 2.0), pos2(r.right() - 1.0, r.bottom() - 3.0));
    painter.rect(
        plate,
        5.0,
        Color32::from_rgb(0x1E, 0x1A, 0x16),
        Stroke::new(1.0, Color32::from_rgb(0x5A, 0x50, 0x48)),
        StrokeKind::Inside,
    );
    jtext::draw(painter, pos2(r.left() + 12.0, r.center().y), Align2::LEFT_CENTER, label, &text(15.0), Color32::from_rgb(0xE8, 0xE2, 0xD6));
    let (sw, sh) = (58.0, 26.0);
    let s = Rect::from_min_size(pos2(r.right() - sw - 12.0, r.center().y - sh / 2.0), vec2(sw, sh));
    painter.rect(
        s,
        sh / 2.0,
        Color32::from_rgb(0x6B, 0x35, 0x30),
        Stroke::new(1.0, Color32::from_rgb(0xC2, 0x52, 0x4A)),
        StrokeKind::Inside,
    );
    painter.circle_filled(pos2(s.left() + 2.0 + (sh - 4.0) / 2.0, s.center().y), (sh - 4.0) / 2.0, Color32::WHITE);
    jtext::draw_bold(painter, pos2(s.right() - 8.0, s.center().y), Align2::RIGHT_CENTER, "OFF", &text(11.0), Color32::WHITE);
}

/// A checklist entry: a numbered gold circle, the step, a live state, an explanation and an illustration.
fn item(ui: &mut Ui, p: &Palette, n: usize, what: &str, state: Option<(String, Color32)>, why: &str, picture: Option<&str>) {
    ui.horizontal_top(|ui| {
        number(ui, p, n);
        ui.add_space(14.0);
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(what).font(text(18.0)).color(p.ink));
                if let Some((s, c)) = &state {
                    ui.add_space(14.0);
                    ui.label(egui::RichText::new(s).font(text(15.0)).color(*c));
                }
            });
            words(ui, why, 17.0, fixed::MUTED, 380.0);
        });
        if let Some(label) = picture {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| switch_off(ui, label));
        }
    });
}

/// A mode's badge at a scale, as on the loadout card.
fn badge(ui: &mut Ui, mode: &str, no_pet: bool, k: f32) -> egui::Response {
    let w = mode_badge::width(ui.painter(), mode, no_pet);
    let (r, resp) = ui.allocate_exact_size(vec2((w + 16.0) * k, (mode_badge::H + 16.0) * k), Sense::hover());
    mode_badge::draw(ui.painter(), r.min + vec2(8.0, 8.0) * k, mode, no_pet, k);
    resp
}

impl Wizard {
    /// Opens it at step 1, for a UPE.
    pub fn new() -> Wizard {
        Wizard { step: 1, category: upe::UPE.into(), variant: upe::UPE.into(), ..Default::default() }
    }

    fn mode(&self) -> String {
        upe::with_guild(if self.category == upe::UPE { &self.variant } else { &self.category }, self.guild)
    }

    /// The wizard's window; `sniffing`: the sniffer runs.
    pub fn show(&mut self, ctx: &egui::Context, p: &Palette, app: &lg_app::App, icons: &mut Icons, sniffing: bool) -> Out {
        if self.step == 2 {
            self.look_for_new_character(app);
        }
        let mut out = Out::Keep;
        let open = crate::widgets::app_window(ctx, p, "mode-wizard", "New game mode run", vec2(860.0, 760.0), |ui| {
            ui.spacing_mut().item_spacing = vec2(6.0, 4.0);
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add_space(8.0);
                out = match self.step {
                    1 => self.choose(ui, p),
                    2 => self.create(ui, p, sniffing),
                    _ => self.done(ui, p, app, icons),
                };
            });
        });
        if !open {
            return Out::Close;
        }
        if self.step == 2 {
            ctx.request_repaint_after(std::time::Duration::from_millis(700));
        }
        out
    }

    fn step_line(ui: &mut Ui, n: u8) {
        ui.label(egui::RichText::new(format!("STEP {n} OF 3")).font(text(13.0)).color(fixed::MUTED));
    }

    fn title(ui: &mut Ui, p: &Palette, s: &str) {
        ui.label(egui::RichText::new(s).font(text(26.0)).color(gold(p)));
    }

    // ---- step 1: the mode

    fn choose(&mut self, ui: &mut Ui, p: &Palette) -> Out {
        Self::step_line(ui, 1);
        Self::title(ui, p, "Choose your challenge");
        ui.add_space(14.0);
        let cw = ((ui.available_width() - 10.0 * (CATEGORY_INFO.len() as f32 - 1.0)) / CATEGORY_INFO.len() as f32).floor();
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing = vec2(10.0, 0.0);
            for m in &CATEGORY_INFO {
                let on = m.code == self.category;
                let (r, resp) = ui.allocate_exact_size(vec2(cw, 150.0), Sense::click());
                let painter = ui.painter().clone();
                painter.rect(
                    r.shrink(1.0),
                    6.0,
                    if on { mix(p.gold, p.paper, 0.22) } else { p.paper },
                    Stroke::new(if on { 2.5 } else { 1.0 }, if on { p.gold } else { p.line }),
                    StrokeKind::Inside,
                );
                let inner = r.shrink2(vec2(14.0, 12.0));
                let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner).layout(egui::Layout::top_down(egui::Align::Min)));
                badge(&mut child, m.code, false, 1.2);
                child.add_space(8.0);
                child.label(egui::RichText::new(m.name).font(text(16.0)).color(p.ink));
                child.add_space(4.0);
                words(&mut child, m.summary, 17.0, fixed::MUTED, inner.width());
                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    self.category = m.code.to_string();
                }
            }
        });
        if self.category == upe::UPE {
            ui.add_space(16.0);
            ui.label(egui::RichText::new("VARIANT").font(text(18.0)).color(gold(p)));
            ui.horizontal_wrapped(|ui| {
                for v in UPE_VARIANTS {
                    let on = v == self.variant;
                    let b = egui::Button::selectable(
                        on,
                        egui::RichText::new(upe::label(v)).font(text(24.0)).color(if on { p.on_accent } else { p.ink }),
                    );
                    if ui.add(b).on_hover_text(upe::info(v).summary).clicked() {
                        self.variant = v.to_string();
                    }
                }
            });
        }
        let mode = self.mode();
        // what the picked mode is, and its rules at a glance
        ui.add_space(10.0);
        words(ui, upe::info(&mode).summary, 20.0, p.ink, 560.0);
        ui.add_space(6.0);
        for (caption, chips) in
            [("Loadout", crate::game_modes::loadout_chips(&mode)), ("From", crate::game_modes::source_chips(self.guild))]
        {
            let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 34.0), Sense::hover());
            crate::game_modes::chip_row(ui.painter(), r.min, caption, &chips);
        }
        ui.add_space(14.0);
        let mut guild = self.guild;
        let mut no_pet = self.no_pet;
        Self::setting(ui, p, &mut guild, "Guild version: trade with Loot Gobbler guild mates (can always be opted into later)", true);
        ui.add_space(4.0);
        Self::setting(ui, p, &mut no_pet, "No pet: play the whole run without a pet (NPE)", false);
        self.guild = guild;
        self.no_pet = no_pet;
        ui.add_space(14.0);
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Its badge:").font(text(16.0)).color(fixed::MUTED));
            ui.add_space(10.0);
            badge(ui, &self.mode(), self.no_pet, 1.5);
        });
        ui.add_space(20.0);
        let mut out = Out::Keep;
        buttons(ui, |ui| {
            if button(ui, p, "Next: create the character", true) {
                self.start_create();
            }
            if button(ui, p, "Cancel", false) {
                out = Out::Close;
            }
        });
        out
    }

    /// A setting to tick: its box, the icon its badge shows, and the words (clicking them ticks it too).
    fn setting(ui: &mut Ui, p: &Palette, on: &mut bool, words_: &str, shield: bool) {
        ui.horizontal_top(|ui| {
            ui.checkbox(on, "");
            let (r, resp) = ui.allocate_exact_size(vec2(26.0, 24.0), Sense::click());
            if shield {
                mode_badge::shield(ui.painter(), r.min + vec2(4.0, 0.0), 24.0 / 21.0);
            } else {
                mode_badge::no_pet_icon(ui.painter(), r.min, 1.0);
            }
            ui.add_space(8.0);
            let l = ui.add(egui::Label::new(egui::RichText::new(words_).font(text(20.0)).color(p.ink)).wrap().sense(Sense::click()));
            if resp.clicked() || l.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                *on = !*on;
            }
        });
    }

    // ---- step 2: create the character

    fn start_create(&mut self) {
        self.step = 2;
        self.before.clear();
        self.told.clear();
        self.problem = None;
        self.before_pending = true;
    }

    fn create(&mut self, ui: &mut Ui, p: &Palette, sniffing: bool) -> Out {
        let mode = self.mode();
        Self::step_line(ui, 2);
        Self::title(ui, p, &format!("Create a new character for your {}", upe::label(&mode)));
        ui.add_space(4.0);
        words(
            ui,
            "Any class. Do these in-game; Loot Gobbler watches and starts the run as soon as the character enters the game.",
            17.0,
            fixed::MUTED,
            560.0,
        );
        ui.add_space(16.0);
        let state = if sniffing { ("✔ running".to_string(), fixed::GREEN) } else { ("✖ off - start it first".to_string(), fixed::RED) };
        item(
            ui,
            p,
            1,
            "Keep Loot Gobbler's sniffer running",
            Some(state),
            "It has to see the character from its very first moment, at level 1.",
            None,
        );
        ui.add_space(12.0);
        item(
            ui,
            p,
            2,
            "Turn upgrades OFF when you create it",
            None,
            "The character must start with a tier 0 weapon, ability and armor. That starting gear never counts against the rules.",
            Some("Upgrades"),
        );
        if self.no_pet {
            ui.add_space(12.0);
            item(
                ui,
                p,
                3,
                "No pet: leave it without one",
                None,
                "No pet may follow it during the whole run. Loot Gobbler sees the pets in the character list at each login.",
                Some("Pet"),
            );
        }
        ui.add_space(12.0);
        let dots = ((ui.input(|i| i.time) / 0.7) as usize) % 4;
        item(
            ui,
            p,
            if self.no_pet { 4 } else { 3 },
            "Create the character and enter the game",
            Some((format!("Waiting for a new character{}", ".".repeat(dots)), gold(p))),
            "The run starts by itself, and the character shows up in the Game Modes tab.",
            None,
        );
        if let Some(problem) = &self.problem {
            ui.add_space(12.0);
            words(ui, &format!("⚠ {problem}"), 15.0, fixed::RED, 680.0);
        }
        ui.add_space(20.0);
        let mut out = Out::Keep;
        buttons(ui, |ui| {
            if button(ui, p, "Cancel", false) {
                out = Out::Close;
            }
            if button(ui, p, "Back", false) {
                self.step = 1;
            }
        });
        out
    }

    /// A new character appeared: starts the run if it can, else says why not and waits for the next.
    fn look_for_new_character(&mut self, app: &lg_app::App) {
        if std::mem::take(&mut self.before_pending) {
            self.before = app.characters.all().map(|c| c.char_id).collect();
            return;
        }
        let mode = self.mode();
        for c in app.characters.all() {
            if self.before.contains(&c.char_id) || self.told.contains(&c.char_id) {
                continue;
            }
            let Some(r) = app.upe.runs.get(&c.char_id) else { continue }; // not looked at by the rules yet
            self.told.insert(c.char_id);
            let cls = if c.class_name.is_empty() { "character" } else { c.class_name.as_str() };
            if !r.fresh {
                self.problem = Some(format!(
                    "Your new {cls} was not seen from its creation (level {}). Create another one with the sniffer running.",
                    c.level
                ));
            } else if r.t0_start == Some(false) {
                self.problem = Some(format!("Your new {cls} was created with upgrades enabled. Create another one with upgrades OFF."));
            } else if !r.can_start(&mode, self.no_pet) {
                self.problem =
                    Some(format!("Your new {cls} cannot start a {}{}", upe::label(&mode), if r.pet { ": it has a pet." } else { "." }));
            } else {
                self.started = Some((c.char_id, mode.clone()));
                self.step = 3;
                self.mark = true;
                return;
            }
            let told: Vec<i32> = self.told.iter().copied().collect();
            self.before.extend(told);
        }
    }

    // ---- step 3: done

    fn done(&mut self, ui: &mut Ui, p: &Palette, app: &lg_app::App, icons: &mut Icons) -> Out {
        let Some((id, mode)) = self.started.clone() else { return Out::Close };
        let mut asks = Vec::new();
        if std::mem::take(&mut self.mark) {
            asks.push(Ask::UpeMark { char_id: id, on: true, mode: Some(mode.clone()), no_pet: Some(self.no_pet) });
        }
        Self::step_line(ui, 3);
        Self::title(ui, p, "Your run has started");
        ui.add_space(20.0);
        let c = app.characters.get(id);
        ui.horizontal(|ui| {
            if let Some(c) = c {
                let sprite = if c.skin > 0 { c.skin } else { c.class_type };
                let tex = icons.skin(ui.ctx(), app.world.data(), sprite, c.tex1, c.tex2, 96);
                crate::widgets::icon(ui, tex.as_ref(), 96.0);
            }
            ui.add_space(16.0);
            ui.vertical(|ui| {
                ui.label(egui::RichText::new(c.map_or("Character", |c| c.class_name.as_str())).font(text(28.0)).color(p.ink));
                ui.add_space(8.0);
                badge(ui, &mode, self.no_pet, 1.5);
            });
        });
        ui.add_space(18.0);
        words(
            ui,
            &format!(
                "It plays a {} ({}) now. Loot Gobbler follows the rules while the sniffer runs; what it finds shows under its name in Inventory > Collection.",
                upe::label(&mode),
                upe::info(&mode).name
            ),
            17.0,
            p.ink,
            560.0,
        );
        ui.add_space(20.0);
        let mut close = false;
        buttons(ui, |ui| {
            if button(ui, p, "Done", true) {
                close = true;
            }
            if button(ui, p, "Open character", false) {
                crate::window::open_later(ui.ctx(), format!("Characters:{id}"));
                close = true;
            }
        });
        if close {
            if !asks.is_empty() {
                // marked and closed in the same frame: both
                crate::window::ask_later(ui.ctx(), asks.remove(0));
            }
            return Out::Close;
        }
        if asks.is_empty() { Out::Keep } else { Out::Asks(asks) }
    }
}
