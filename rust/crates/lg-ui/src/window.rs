//! The whole window (Java: `TomatoGUI`): the frame around the views, every tab's view with its state, the
//! settings it shows, the icons. Shown by the app each frame, and by `ui-shot` for pictures.

use crate::icons::Icons;
use crate::{battles, characters, game_modes, guild, inventory, profile, shell, social, theme};
use lg_app::properties::Properties;
use std::path::Path;

/// What a view asks the app to do (the views only read the records and the settings).
#[derive(Debug, Clone, PartialEq)]
pub enum Ask {
    /// Save a setting.
    Set(String, String),
    /// Empty the chat log.
    ClearChat,
    /// Mark a finished dungeon visit (by its start) removed.
    HideVisit(i64),
    /// Every dungeon visit gone.
    ClearVisits,
    /// Mark a saved run (by its start) removed.
    HideRun(i64),
    /// Every saved run gone.
    ClearRuns,
    /// The whole collection history gone.
    ClearCollection,
    /// A character's favourite (true) or dead (false) mark switched by hand.
    ToggleChar(i32, bool),
    /// Nothing (what a message's button asks).
    Nothing,
    /// Play a sound: a bundled file ("pm.wav"), "item:<setting key>" or "startup:<id>" (Settings' previews).
    Play(String),
    /// Game Modes: start (with the mode and no-pet choice) or end a character's run.
    UpeMark { char_id: i32, on: bool, mode: Option<String>, no_pet: Option<bool> },
    /// Game Modes: a run's mode changed (its guild version on or off).
    UpeMode(i32, String),
    /// Game Modes: playing without a pet on or off.
    UpeNoPet(i32, bool),
    /// Settings > Ping Lists: the editor (0 chat, 1 item, 2 enchant, 3 entity, 4 dungeon pings, 5 ping sounds).
    OpenDialog(usize),
    /// A file (a WAV for an item sound) or a folder (the logs') picked for a setting, with the dialog's title.
    Choose { key: String, title: String, folder: bool },
    /// Ask first (Java `JOptionPane.showConfirmDialog`): title, text, the yes and no buttons; yes asks `then`. With
    /// `no` empty it is a message (`showMessageDialog`): one button, nothing asked.
    Confirm { title: String, text: String, yes: String, no: String, then: Box<Ask> },
    /// The Server tab: something for the server or the sync.
    Server(ServerAsk),
    /// Characters: REFRESH (the whole character list from the game servers).
    RefreshCharacters,
    /// The Quests page's Refresh: the battle pass missions asked from the game again.
    RefreshMissions,
    /// Info > Logs: a log file saved elsewhere (its path).
    ExportLog(String),
    /// CHECK UPDATES (the status bar, About).
    CheckUpdates,
    /// Info > About's "Check for updates": checks now and answers in a message.
    CheckUpdatesAbout,
    /// Info > Release Notes: every release's notes, loaded from GitHub.
    ReleaseNotes,
    /// A web page to open (empty: the releases page).
    OpenUrl(String),
    /// The first start's greeting: the Java app's records taken over from this folder (the app starts again).
    TakeOverJava(String),
}

/// What the Server tab asks of the server (Java `ServerPanel`'s buttons).
#[derive(Debug, Clone, PartialEq)]
pub enum ServerAsk {
    Connect(String),
    Test(String),
    Forget,
    Login(String, String),
    /// The password and the role token (the IGN is the one the game showed).
    Register(String, String),
    Redeem(String),
    Logout,
    SyncNow,
    /// The files on this PC become the logged-in account's.
    Adopt,
    /// The password dialogs: Forgot password (with the IGN typed so far), Change password.
    OpenForgot(String),
    OpenChange,
    /// Their OK: a new password with a reset code; or proven with the current password or a reset code.
    Forgot {
        ign: String,
        code: String,
        password: String,
        again: String,
    },
    Change {
        with_code: bool,
        proof: String,
        password: String,
        again: String,
    },
    /// Moderation (moderators and admins): the lists again, and the actions on a player (id, IGN).
    RefreshModeration,
    Accept(i64, String),
    Deny(i64, String),
    /// Demoted to this role ("user" or "moderator").
    Demote(i64, String, String),
    ResetCode(i64),
    Remove(i64, String),
    /// Moderator tokens (admins): one made (note, hours valid), one copied, one revoked.
    CreateToken(String, i32),
    TokenCopied,
    RevokeToken(i64),
    /// Item score weights and guild settings (admins).
    SaveScoring(f64, f64, f64),
    SaveGuild(f64),
    /// A finished visit's guildmates from the server (dungeon, start, duration); asked again for those awaited.
    RunLooks(String, i64, i64),
    RetryLooks(String),
    /// The bell was opened: nothing new any more.
    MarkRead,
    /// The Guild tab: load the leaderboard and drops again; open a member's profile.
    GuildRefresh,
    /// The loot caravan again (the minute's check while it is shown, a click on Refresh).
    CaravanLoad,
    /// The quest maps again.
    MapsLoad,
    /// The Profile's guild card again.
    ProfileGuildLoad,
    OpenMember(i64),
    /// A line under a section (0 item scores, 1 guild): what was typed cannot be used.
    Feedback(u8, String),
}

/// Asks the window to open a tab or page after this frame (see [`Window::open`]; also "Battles:run:<start>" for a
/// run under DPS, "Inventory:item:<id>" for an item). For views deep in helpers that have no context.
pub fn open_later(ctx: &egui::Context, spec: impl Into<String>) {
    let spec = spec.into();
    ctx.data_mut(|d| d.insert_temp(egui::Id::unique("lg-open"), spec));
}

/// Asks after this frame, from helpers deep in a view without its context (see [`open_later`]).
pub fn ask_later(ctx: &egui::Context, ask: Ask) {
    ctx.data_mut(|d| d.get_temp_mut_or_default::<Vec<Ask>>(egui::Id::unique("lg-asks")).push(ask));
}

pub struct Window {
    pub shell: shell::Shell,
    pub palette: theme::Palette,
    pub icons: Icons,
    pub en: lg_app::loadout::Enchantments,
    pub loadout: lg_app::loadout::Settings,
    pub props: Properties,
    pub profile: profile::Profile,
    pub info: crate::info::Info,
    pub characters: characters::CharactersView,
    pub inventory: inventory::InventoryView,
    pub game_modes: game_modes::GameModesView,
    pub battles: battles::BattlesView,
    pub social: social::SocialView,
    pub guild: guild::GuildView,
    pub guide: crate::guide::Guide,
    pub tour: crate::guide::FeatureTour,
    /// The greeting of the very first start, the release notes window, and what of the start is still to come
    /// (Java `TomatoGUI`: the release notes after an update, then the greeting, then the Newcomer's Guide).
    pub welcome: bool,
    /// The first start's offer to take over the Java app's records (shown in the greeting).
    pub java_offer: Option<crate::welcome::JavaOffer>,
    pub release_notes: Option<crate::welcome::ReleaseNotes>,
    start: StartStep,
    /// The Newcomer's Guide shows a sample dungeon in the menu bar: since when it is "in" it.
    preview_since: Option<i64>,
    themed: bool,
    /// What the views asked for in the last frame; the app takes them (`take_asks`).
    asks: Vec<Ask>,
    /// The question shown over the window, if any.
    confirm: Option<Ask>,
    /// "Copy as image": the character whose card is drawn for the grab, and for how many frames yet.
    card: Option<(i32, u8)>,
    /// A ping editor open over the window.
    dialog: Option<crate::dialogs::Dialog>,
    /// A ping editor to open next frame (it needs the game data).
    pending_dialog: Option<usize>,
}

/// Java `withNewTabs`: the saved order plus the tabs new since it was saved, each in front of the saved tab that
/// follows it in the default order; unknown names left out.
fn with_new_tabs(saved: &str) -> Vec<String> {
    let mut names: Vec<String> = saved.split('|').filter(|t| shell::TABS.contains(t)).map(str::to_string).collect();
    names.dedup();
    for i in (0..shell::TABS.len()).rev() {
        if names.iter().any(|n| n == shell::TABS[i]) {
            continue;
        }
        let at = shell::TABS[i + 1..].iter().find_map(|t| names.iter().position(|n| n == t)).unwrap_or(names.len());
        names.insert(at, shell::TABS[i].to_string());
    }
    names
}

/// What of the start waits for a window to close.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StartStep {
    Done,
    Notes,
    Welcome,
}

fn now_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as i64)
}

const GUILD_PAGES: [&str; 7] = crate::guild::PAGES;

/// The folder picker's key for the Java app's folder (the first start's greeting): the pick fills the offer.
pub const JAVA_FOLDER_PICK: &str = "javaFolderPick";

impl Window {
    /// With the game data in `assets` and the settings (`realmShark.properties`).
    pub fn new(assets: &Path, props: Properties) -> Window {
        if let Some(folder) = assets.parent() {
            crate::quests::set_folder(folder);
        }
        let loadout = lg_app::loadout::Settings::from(&props);
        let palette = theme::palette(props.get("theme"));
        // the saved tab order (Java `restoreOrder`); the app opens on the tab dragged to the far left
        let mut shell = shell::Shell::default();
        if let Some(saved) = props.get("tabOrder").filter(|s| !s.trim().is_empty()) {
            shell.order = with_new_tabs(&saved.replace("DPS Logger", "Battles"));
            shell.selected = shell.order[0].clone();
        }
        Window {
            shell,
            palette,
            icons: Icons::new(assets),
            en: lg_app::loadout::Enchantments::load(&assets.join("xml")),
            loadout,
            props,
            profile: profile::Profile::default(),
            characters: characters::CharactersView::default(),
            info: Default::default(),
            inventory: inventory::InventoryView::default(),
            game_modes: game_modes::GameModesView::default(),
            battles: battles::BattlesView { selected: -1, ..Default::default() },
            social: social::SocialView::default(),
            guild: guild::GuildView::default(),
            guide: Default::default(),
            tour: Default::default(),
            welcome: false,
            java_offer: None,
            release_notes: None,
            start: StartStep::Done,
            preview_since: None,
            themed: false,
            asks: Vec::new(),
            confirm: None,
            card: None,
            dialog: None,
            pending_dialog: None,
        }
    }

    /// Opens a tab, or a page of it: "Inventory:st", "Inventory:log", "Characters:<charId|classes|pets>",
    /// "Battles:dps|overlay", "Social:quests|discord", "Guild:<page name>".
    pub fn open(&mut self, spec: &str) {
        let (tab, page) = spec.split_once(':').map_or((spec, ""), |(t, p)| (t, p));
        self.shell.selected = tab.to_string();
        match (tab, page) {
            ("Inventory", p) if p.starts_with("upe:") => {
                // Java `showUpe`: the run's own finds
                self.inventory.page = 0;
                self.inventory.upe_char = p[4..].parse().unwrap_or(0);
            }
            ("Inventory", p) if p.starts_with("item:") => {
                // Java `showItem`: the Items page, the item scrolled to and pulsing, its history beside the grid
                self.inventory.page = 0;
                self.inventory.upe_char = 0;
                match p[5..].parse::<i32>() {
                    Ok(id) => self.inventory.show_item = Some(id),
                    Err(_) => self.inventory.search = p[5..].to_string(),
                }
            }
            ("Inventory", p) => self.inventory.page = ["", "st", "log"].iter().position(|x| *x == p).unwrap_or(0),
            ("Characters", "classes") => self.characters.page = 1,
            ("Characters", "pets") => self.characters.page = 2,
            ("Characters", p) if p.starts_with("class:") => {
                self.characters.page = 1;
                self.characters.open_class = p[6..].parse().ok();
            }
            ("Characters", id) if !id.is_empty() => {
                self.characters.page = 0;
                self.characters.selected = id.parse().unwrap_or(0);
            }
            ("Profile", p) => self.profile.page = ["", "quests", "stats"].iter().position(|x| *x == p).unwrap_or(0),
            ("Battles", p) => self.battles.page = ["", "dps", "overlay"].iter().position(|x| *x == p).unwrap_or(0),
            ("Social", p) => self.social.page = ["", "quests", "discord"].iter().position(|x| *x == p).unwrap_or(0),
            ("Guild", p) => self.guild.page = GUILD_PAGES.iter().position(|x| *x == p).unwrap_or(0),
            _ => {}
        }
    }

    /// What the views asked for since the last call. Settings asked to be saved are taken into the shown settings
    /// at once (the caller writes them to the file); a new theme is applied.
    pub fn take_asks(&mut self) -> Vec<Ask> {
        let mut asks = std::mem::take(&mut self.asks);
        // the ping editors open here (Java plays a sound as one opens)
        if let Some(i) = asks.iter().position(|a| matches!(a, Ask::OpenDialog(_))) {
            let Ask::OpenDialog(k) = asks.remove(i) else { unreachable!() };
            self.pending_dialog = Some(k);
            match k {
                0 => asks.push(Ask::Play("pm.wav".into())),
                1 | 3 => asks.push(Ask::Play("custom.wav".into())),
                _ => {}
            }
        }
        // the password dialogs open here
        if let Some(i) = asks.iter().position(|a| matches!(a, Ask::Server(ServerAsk::OpenForgot(_) | ServerAsk::OpenChange))) {
            self.dialog = Some(match asks.remove(i) {
                Ask::Server(ServerAsk::OpenForgot(ign)) => crate::dialogs::Dialog::password(false, &ign),
                _ => crate::dialogs::Dialog::password(true, ""),
            });
        }
        // a question is shown, not done
        if let Some(i) = asks.iter().position(|a| matches!(a, Ask::Confirm { .. })) {
            self.confirm = Some(asks.remove(i));
        }
        for a in &asks {
            if let Ask::Set(k, v) = a {
                self.props.set_in_memory(k, v);
                if k == "theme" {
                    self.palette = theme::palette(Some(v));
                    self.themed = false;
                }
            }
        }
        asks
    }

    /// Once the window is up (Java `TomatoGUI`): with `notes` the release notes after an update come first (shown
    /// while they load); then on the very first start the greeting; then the Newcomer's Guide.
    pub fn begin_start(&mut self, notes: Option<String>) {
        match notes {
            Some(from) => {
                self.release_notes = Some(crate::welcome::ReleaseNotes { from: Some(from), notes: None });
                self.start = StartStep::Notes;
            }
            None => self.after_notes(),
        }
    }

    fn after_notes(&mut self) {
        if self.props.get("welcomeShown") != Some("true") {
            self.asks.push(Ask::Set("welcomeShown".into(), "true".into()));
            self.welcome = true;
            self.start = StartStep::Welcome;
            // new to the app: nothing is new to them
            self.tour.first_start(&self.props);
        } else {
            self.after_welcome();
        }
    }

    /// The Newcomer's Guide, then (not while it runs) the What's New tour not seen yet.
    fn after_welcome(&mut self) {
        self.start = StartStep::Done;
        self.guide.after_start(&self.props);
        self.tour.after_start(&self.props, self.guide.running());
    }

    /// Asks as a view would (for parts outside the views, e.g. the overlay windows).
    pub fn ask(&mut self, a: Ask) {
        self.asks.push(a);
    }

    /// The question shown over the window, if any.
    pub fn asking(&self) -> Option<&Ask> {
        self.confirm.as_ref()
    }

    /// Java `LoadoutCard.copyButton`: the card drawn over the window for two frames, the first grabbed, cropped and
    /// put on the clipboard; the button says "Copied" a moment.
    fn grab_card(&mut self, ui: &egui::Ui, app: &lg_app::App) {
        let ctx = ui.ctx().clone();
        if let Some(id) = ctx.data_mut(|d| d.remove_temp::<i32>(egui::Id::unique("lg-copy-card"))) {
            self.card = Some((id, 2));
        }
        let Some((id, left)) = self.card else { return };
        let Some(ch) = app.characters.get(id) else {
            self.card = None;
            return;
        };
        let ppp = ctx.pixels_per_point();
        let k = crate::card::SCALE / ppp;
        let rarity = self.characters.rarity(&ctx);
        let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::unique("lg-card")));
        let mode = crate::card::mode_of(app, id);
        let r =
            crate::card::paint(&painter, &ctx, &mut self.icons, app.world.data(), &rarity, &self.palette, ch, mode, ui.max_rect().min, k);
        if left == 2 {
            let c2 = ctx.clone();
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::ScreenshotCallback::new(move |frame| {
                c2.copy_image(crate::card::crop(&frame, r, ppp, k));
                c2.data_mut(|d| d.insert_temp(egui::Id::unique("lg-card-copied"), now_ms()));
                c2.request_repaint();
            })));
        }
        self.card = if left > 1 { Some((id, left - 1)) } else { None };
        ctx.request_repaint();
    }

    /// The question asked over the window: its buttons answer it, closing it is "no".
    fn question(&mut self, ctx: &egui::Context) {
        let Some(Ask::Confirm { title, text, yes, no, then }) = &self.confirm else { return };
        let p = self.palette;
        let mut answer: Option<bool> = None;
        let modal = egui::Modal::new(egui::Id::unique("question")).frame(egui::Frame::NONE).show(ctx, |ui| {
            let f = crate::widgets::text(18.0);
            let painter = ui.painter().clone();
            let lines: Vec<&str> = text.lines().collect();
            let lh = crate::jtext::height(&painter, &f);
            let tw = lines.iter().map(|l| crate::jtext::width(&painter, l, &f)).fold(0.0, f32::max);
            let w = (tw + 48.0).max(360.0);
            let h = 34.0 + 20.0 + lines.len() as f32 * lh + 20.0 + 36.0 + 16.0;
            let (r, _) = ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::hover());
            painter.rect(r, 0.0, p.paper, egui::Stroke::new(1.0, p.line), egui::StrokeKind::Inside);
            let head = egui::Rect::from_min_size(r.min, egui::vec2(w, 34.0));
            painter.rect_filled(head, 0.0, p.title);
            crate::jtext::draw(&painter, egui::pos2(head.left() + 12.0, head.center().y), egui::Align2::LEFT_CENTER, title, &f, p.title_fg);
            let mut y = head.bottom() + 20.0;
            for l in &lines {
                crate::jtext::draw(&painter, egui::pos2(r.left() + 24.0, y), egui::Align2::LEFT_TOP, l, &f, p.ink);
                y += lh;
            }
            let small = crate::widgets::small();
            let bw = |t: &str| (crate::jtext::width(&painter, &t.to_uppercase(), &small) + 32.0).max(108.0);
            let nr = egui::Rect::from_min_size(egui::pos2(r.right() - 16.0 - bw(no), r.bottom() - 16.0 - 36.0), egui::vec2(bw(no), 36.0));
            let yr = egui::Rect::from_min_size(egui::pos2(nr.left() - 8.0 - bw(yes), nr.top()), egui::vec2(bw(yes), 36.0));
            if !no.is_empty() && crate::inventory::button(ui, &p, yr, yes, false, "question-yes") {
                answer = Some(true);
            }
            if no.is_empty() {
                // a message: its one button at the right
                let one = egui::Rect::from_min_size(egui::pos2(r.right() - 16.0 - bw(yes), nr.top()), egui::vec2(bw(yes), 36.0));
                if crate::inventory::button(ui, &p, one, yes, false, "question-ok") {
                    answer = Some(false);
                }
                return;
            }
            if crate::inventory::button(ui, &p, nr, no, false, "question-no") {
                answer = Some(false);
            }
        });
        if modal.should_close() && answer.is_none() {
            answer = Some(false);
        }
        if let Some(a) = answer {
            if a {
                self.asks.push((**then).clone());
            }
            self.confirm = None;
        }
    }

    /// The whole window in `ui`.
    pub fn show(&mut self, ui: &mut egui::Ui, app: &lg_app::App, now: i64) {
        crate::guide::begin_frame(ui.ctx());
        if !self.themed {
            theme::apply(ui.ctx(), &self.palette);
            self.themed = true;
        }
        let palette = self.palette;
        self.shell.notifications = app.server.notifications.iter().map(|n| (n.title.clone(), n.text.clone())).collect();
        self.shell.unread = app.server.unread;
        // Java `StatusBar.showServer`: grey when no server is set up, green when connected, else red
        let s = &app.server;
        (self.shell.server, self.shell.server_tip) = if s.url.trim().is_empty() {
            (shell::Server::NotConfigured, "No Loot Gobbler server set up. Click to open the Server tab.".into())
        } else if s.state == lg_app::server::State::Online {
            let who = s.user.as_ref().map_or(" (not logged in)".to_string(), |u| format!(" as {}", u.ign));
            (shell::Server::Connected, format!("Connected to {}{who}. Click to open the Server tab.", s.url))
        } else {
            let why = if s.state == lg_app::server::State::Connecting {
                "connecting..."
            } else if s.error.trim().is_empty() {
                "not connected"
            } else {
                &s.error
            };
            (shell::Server::Offline, format!("{}: {why}. Click to open the Server tab.", s.url))
        };
        // the focus boxes under the focus line: the runs that can be the focus, the one picked
        let data = app.world.data();
        self.shell.focus_boxes = crate::focus::candidates(app)
            .into_iter()
            .map(|(r, ch)| {
                let skin = if ch.skin > 0 { ch.skin } else { ch.class_type };
                shell::FocusBox {
                    id: r.char_id,
                    title: ch.class_name.clone(),
                    sub: format!("{}{}", lg_app::upe::label(&r.mode), if ch.seasonal { " · seasonal" } else { "" }),
                    icon: self.icons.skin(ui.ctx(), data, skin, ch.tex1, ch.tex2, 72).map(|t| t.id()),
                }
            })
            .collect();
        self.shell.focus_on = crate::focus::current(app, &self.props).map(|(r, _)| r.char_id);
        let Window { shell, icons, en, loadout, props, profile, characters, inventory, game_modes, battles, social, guild, asks, .. } =
            self;
        shell.show(ui, &palette, |ui, tab| {
            let mut c = profile::Ctx { app, p: &palette, icons, now, en, loadout, props, asks };
            match tab {
                "Profile" => {
                    let rarity = characters.rarity(ui.ctx());
                    profile.show(ui, &mut c, &rarity);
                }
                "Inventory" => {
                    let rarity = characters.rarity(ui.ctx());
                    inventory.show(ui, &mut c, &rarity);
                    inventory.grab_step(ui, &mut c, &rarity)
                }
                "Characters" => characters.show(ui, &mut c),
                "Game Modes" => game_modes.show(ui, &mut c),
                "Battles" => {
                    let rarity = characters.rarity(ui.ctx());
                    battles.show(ui, &mut c, &rarity)
                }
                "Social" => social.show(ui, &mut c),
                "Guild" => {
                    let rarity = characters.rarity(ui.ctx());
                    guild.show(ui, &mut c, &rarity);
                }
                "Server" => crate::server::show(ui, &mut c),
                "Settings" => crate::settings::show(ui, &mut c),
                _ => {}
            }
        });
        // the dungeon you are in, in the menu bar after the menus: as high as Java's menu bar (30 px less its
        // line), centred in the taller title bar
        let bar = self.shell.menu_rect;
        let badge_h = 30.0 - 1.5;
        let (badge, open_run) = crate::title_badge::show(
            ui,
            &mut self.icons,
            app,
            now,
            egui::pos2(self.shell.menu_end + 12.0, bar.center().y - badge_h / 2.0),
            badge_h,
            self.preview_since,
        );
        if let Some(r) = badge {
            crate::guide::mark(ui, "title.dungeon", r);
        }
        if open_run {
            // the dungeon you are in, under Battles > Dungeons
            self.open("Battles");
            if let Some(v) = &app.dungeon_visits.current {
                self.battles.selected = v.start;
            }
        }
        if let Some(pick) = self.shell.clicked_focus {
            self.asks.push(Ask::Set(crate::focus::KEY.into(), crate::focus::value(pick)));
        }
        if self.shell.order_changed {
            self.asks.push(Ask::Set("tabOrder".into(), self.shell.order.join("|")));
        }
        if self.shell.clicked_server {
            self.open("Server");
        }
        match self.shell.clicked_info {
            Some("Quick tour") => self.guide.start_quick(),
            Some("Release Notes") => self.asks.push(Ask::ReleaseNotes),
            Some("What's New Tour") => {
                if !self.tour.show_latest(self.guide.running()) {
                    self.asks.push(Ask::Confirm {
                        title: "What's New Tour".into(),
                        text: "No tour yet: the new features of the next releases are shown here.".into(),
                        yes: "OK".into(),
                        no: String::new(),
                        then: Box::new(Ask::Nothing),
                    });
                }
            }
            Some(item) => self.info.open(item, &mut self.asks),
            None => {}
        }
        if self.shell.clicked_updates {
            self.asks.push(Ask::CheckUpdates);
        }
        self.info.show(ui.ctx(), &self.palette, &mut self.asks);
        // the game mode wizard, over whichever tab is open (Java: a window of its own)
        if let Some(w) = &mut self.game_modes.wizard {
            match w.show(ui.ctx(), &self.palette, app, &mut self.icons, self.shell.sniffing) {
                crate::game_mode_wizard::Out::Keep => {}
                crate::game_mode_wizard::Out::Close => self.game_modes.wizard = None,
                crate::game_mode_wizard::Out::Asks(a) => self.asks.extend(a),
            }
        }
        // the bell: opening it takes the dot away; an entry opens where it is dealt with (the approval queue)
        if self.shell.clicked_bell {
            self.asks.push(Ask::Server(ServerAsk::MarkRead));
        }
        if self.shell.clicked_notification.is_some() {
            self.open("Server");
            self.asks.push(Ask::Server(ServerAsk::RefreshModeration));
        }
        if let Some(k) = self.pending_dialog.take() {
            use crate::dialogs::Dialog;
            let data = app.world.data();
            self.dialog = Some(match k {
                0 => Dialog::list(&self.props, "chatPingMessages", "Chat Messages Ping Customization", "Chat Message", false),
                1 => Dialog::list(&self.props, "itemPings", "Item Ping Customization", "Item ID or Name", false),
                2 => Dialog::enchants(&self.props, data),
                3 => Dialog::list(&self.props, "entityIdPings", "Entity ID Ping Customization", "Entity ID", true),
                4 => Dialog::dungeons(&self.props, data),
                _ => Dialog::ping_sounds(&self.props),
            });
        }
        if let Some(d) = &mut self.dialog
            && !crate::dialogs::show(ui.ctx(), &self.palette, &self.props, d, &mut self.asks)
        {
            self.dialog = None;
        }
        // the greeting and the release notes
        if self.welcome {
            match crate::welcome::welcome(ui.ctx(), &self.palette, &mut self.java_offer) {
                crate::welcome::Greeting::Keep => {}
                crate::welcome::Greeting::Browse => self.asks.push(Ask::Choose {
                    key: JAVA_FOLDER_PICK.into(),
                    title: "The Java Loot Gobbler's folder".into(),
                    folder: true,
                }),
                crate::welcome::Greeting::Close => {
                    self.welcome = false;
                    // the Java app's records wanted: the app starts again with them
                    if let Some(o) = self.java_offer.take().filter(|o| o.take && !o.path.trim().is_empty()) {
                        self.asks.push(Ask::TakeOverJava(o.path.trim().to_string()));
                    } else if self.start == StartStep::Welcome {
                        self.after_welcome();
                    }
                }
            }
        }
        if let Some(n) = &self.release_notes {
            match n.show(ui.ctx(), &self.palette, &app.zone, env!("CARGO_PKG_VERSION")) {
                crate::welcome::Out::Keep => {}
                crate::welcome::Out::Close => {
                    self.release_notes = None;
                    if self.start == StartStep::Notes {
                        self.after_notes();
                    }
                }
                crate::welcome::Out::ShowAll => {
                    self.release_notes = None;
                    self.asks.push(Ask::ReleaseNotes);
                }
                crate::welcome::Out::Open(url) => self.asks.push(Ask::OpenUrl(url)),
            }
        }
        self.question(ui.ctx());
        self.grab_card(ui, app);
        // the Newcomer's Guide over all of it
        let seen =
            crate::guide::Seen { app, sniffing: self.shell.sniffing, tab: &self.shell.selected, menu_open: self.shell.info_open, now };
        let mut out = self.guide.show(ui.ctx(), &self.palette, &seen, &mut self.asks);
        let tour = self.tour.show(ui.ctx(), &self.palette, &self.props, &mut self.asks);
        out.open.extend(tour.open);
        for spec in &out.open {
            match spec.strip_prefix("tab:") {
                Some(tab) => self.shell.selected = tab.to_string(),
                None => self.open(spec),
            }
        }
        if let Some(open) = out.menu {
            self.shell.info_open = open;
            self.shell.guide_menu_open = open;
        }
        self.battles.example = out.example_run;
        self.preview_since = match (out.preview_dungeon, self.preview_since) {
            (true, Some(t)) => Some(t),
            (true, None) => Some(now - 72_000), // in it for 1:12
            (false, _) => None,
        };
        if let Some(later) = ui.ctx().data_mut(|d| d.remove_temp::<Vec<Ask>>(egui::Id::unique("lg-asks"))) {
            self.asks.extend(later);
        }
        if let Some(spec) = ui.ctx().data_mut(|d| d.remove_temp::<String>(egui::Id::unique("lg-open"))) {
            match spec.split_once(":run:").filter(|(t, _)| *t == "Battles") {
                Some((_, start)) => {
                    self.shell.selected = "Battles".into();
                    self.battles.page = 1;
                    self.battles.dps.show_run(&app.dps_history, start.parse().ok());
                }
                None => self.open(&spec),
            }
            ui.ctx().request_repaint();
        }
    }
}
