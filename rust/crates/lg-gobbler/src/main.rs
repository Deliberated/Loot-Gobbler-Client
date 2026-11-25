// a window app on Windows: no console window beside it
#![cfg_attr(windows, windows_subsystem = "windows")]
//! Loot Gobbler: its window over your records, captures the game as the Java app does (while the sniffer runs),
//! keeps your records and writes them into its own folder (next to the program, or `--data`) in the Java app's
//! formats. On a first start the Java app's folder (`--java`) is taken over: read only, copied in full.
//!
//!   loot-gobbler [--java <Java app folder>] [--data <folder>] [--path <resources.assets>] [--assets <folder>]
//!                [--offline] [--pcap <file>] [--no-window]
//!
//! The game data (ObjectID.list, xml/...) is extracted from the game into `assets` in the own folder when the game's
//! file changed, as the Java app does (`--path`: the game's `resources.assets`, if it is not found by itself).
//! `--assets`: game data from another folder instead, as it is.
//! `--offline`: never ask the game's web API (character list, exaltations, friends, missions).
//! `--pcap`: a recording played first, as if it happened now (without a window: instead of capturing).
//! `--no-window`: no window; captures at once until Ctrl+C.

mod capture_setup;
mod gui;
mod net;
mod sound;

use lg_replay::{Item, Session};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

struct Options {
    java: Option<PathBuf>,
    data: Option<PathBuf>,
    assets: Option<PathBuf>,
    game: Option<PathBuf>,
    offline: bool,
    pcap: Option<PathBuf>,
    no_window: bool,
    /// Asked from the first start's greeting: the Java app's records taken over from this folder (the app restarted).
    take_over: Option<PathBuf>,
}

fn options() -> Result<Options, String> {
    let mut o = Options { java: None, data: None, assets: None, game: None, offline: false, pcap: None, no_window: false, take_over: None };
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut value = || args.next().map(PathBuf::from).ok_or(format!("{a} needs a value"));
        match a.as_str() {
            "--java" => o.java = Some(value()?),
            "--data" => o.data = Some(value()?),
            "--assets" => o.assets = Some(value()?),
            "--path" => o.game = Some(value()?),
            "--pcap" => o.pcap = Some(value()?),
            "--take-over-java" => o.take_over = Some(value()?),
            "--offline" => o.offline = true,
            "--no-window" => o.no_window = true,
            _ => return Err(format!("unknown: {a}")),
        }
    }
    Ok(o)
}

fn now_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as i64)
}

fn main() -> ExitCode {
    // what version this program is (the updater asks a downloaded one before it puts it in place)
    if std::env::args().nth(1).as_deref() == Some("--version") {
        println!("{}", env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }
    lg_net::updater::clean_up();
    let mut o = match options() {
        Ok(o) => o,
        Err(e) => {
            eprintln!(
                "{e}\nusage: loot-gobbler [--java <dir>] [--data <dir>] [--path <resources.assets>] [--assets <dir>] [--offline] [--pcap <file>] [--no-window]"
            );
            return ExitCode::FAILURE;
        }
    };
    let own = match o.data.clone().map_or_else(lg_app::files::own_folder, Ok) {
        Ok(d) => d,
        Err(e) => {
            lg_app::applog::err(&format!("no own folder: {e}"));
            return ExitCode::FAILURE;
        }
    };
    if let Err(e) = std::fs::create_dir_all(&own) {
        lg_app::applog::err(&format!("cannot make {}: {e}", own.display()));
        return ExitCode::FAILURE;
    }
    // one app per records folder: two would write over each other's records
    #[cfg(windows)]
    let Some(_held) = hold_folder(&own) else {
        lg_app::applog::line("app", "already running on these records: showing that one");
        show_running();
        return ExitCode::SUCCESS;
    };
    // the greeting of the first start offered the Java app's records and they were wanted: what this first start
    // wrote is put aside (it holds nothing yet) and the Java app's folder taken over as on a first start
    if let Some(j) = o.take_over.clone()
        && lg_app::properties::Properties::load(&own).get(JAVA_OFFER) == Some("pending")
        && lg_app::files::holds_data(&j)
    {
        for f in lg_app::files::KEPT.iter().chain(&["char-list.xml", "resources.json", lg_app::properties::FILE]) {
            let _ = std::fs::remove_file(own.join(f));
        }
        o.java = Some(j);
    }
    // a first start without --java: the greeting offers the Java app's folder (where it keeps its records by default)
    let offer = (o.java.is_none() && !lg_app::files::holds_data(&own)).then(|| java_folder(&own)).flatten();
    if let Some(j) = &o.java {
        match lg_app::files::take_over_settings(&own, j) {
            Ok(true) => lg_app::applog::line("app", "took the Java app's settings over"),
            Ok(false) => {}
            Err(e) => lg_app::applog::err(&format!("cannot copy the Java app's settings: {e}")),
        }
    }
    // Java `AppLog.install`: the log files as the settings say, then what a bug report needs first
    lg_app::applog::configure(&lg_app::properties::Properties::load(&own), &own.join("logs"));
    lg_app::applog::banner();
    let assets = match &o.assets {
        Some(a) => a.clone(),
        None => own_assets(&own, &o),
    };
    let game = lg_gamedata::GameData::load(&assets);
    for p in &game.problems {
        lg_app::applog::err(&p.to_string());
    }
    if game.objects.is_empty() {
        lg_app::applog::err(&format!("no game data in {} (ObjectID.list, xml/...): give --assets or --java", assets.display()));
        return ExitCode::FAILURE;
    }
    let first = !lg_app::files::holds_data(&own);
    let mut app = match lg_app::App::start(lg_world::World::new(game), now_ms(), &own, o.java.as_deref()) {
        Ok(a) => a,
        Err(e) => {
            lg_app::applog::err(&format!("cannot write into {}: {e}", own.display()));
            return ExitCode::FAILURE;
        }
    };
    if first && o.java.as_deref().is_some_and(lg_app::files::holds_data) {
        lg_app::applog::line("app", &format!("took the Java app's records over into {}", own.display()));
    }
    lg_app::applog::line("app", &format!("your records: {}", own.display()));
    app.chat.load_blocked(&own); // the chat's spam phrases kept next to the records
    if !o.offline {
        app.web = Box::new(lg_net::Online::new());
    }
    if o.no_window {
        return match &o.pcap {
            Some(f) => play(&mut app, &own, f),
            None => capture(&mut app, &own),
        };
    }
    if let Some(f) = &o.pcap
        && play(&mut app, &own, f) != ExitCode::SUCCESS
    {
        return ExitCode::FAILURE;
    }
    // frames shown as soon as they are drawn: no waiting for the screen's refresh (vsync) and one frame in flight at
    // most, so hover and clicks show at once, as in the Java app. The window only redraws on input and while
    // something moves, so this costs nothing at rest
    #[allow(unused_mut)]
    let mut wgpu_options = eframe::egui_wgpu::WgpuConfiguration {
        surface: eframe::egui_wgpu::SurfaceConfig {
            present_mode: eframe::wgpu::PresentMode::AutoNoVsync,
            desired_maximum_frame_latency: Some(1),
        },
        ..Default::default()
    };
    // Windows: DirectX 12 through a DirectComposition visual, whose windows can be see-through pixel by pixel (the
    // overlays: soft edges, the slider's opacity). OpenGL drew every colour too light. WGPU_BACKEND /
    // WGPU_DX12_PRESENTATION_SYSTEM still choose otherwise.
    #[cfg(windows)]
    {
        let mut setup = eframe::egui_wgpu::WgpuSetupCreateNew::without_display_handle();
        if std::env::var_os("WGPU_BACKEND").is_none() {
            setup.instance_descriptor.backends = eframe::wgpu::Backends::DX12;
        }
        if std::env::var_os("WGPU_DX12_PRESENTATION_SYSTEM").is_none() {
            setup.instance_descriptor.backend_options.dx12.presentation_system = eframe::wgpu::Dx12SwapchainKind::DxgiFromVisual;
        }
        wgpu_options.wgpu_setup = eframe::egui_wgpu::WgpuSetup::CreateNew(setup);
    }
    let options = eframe::NativeOptions {
        // see-through windows possible (eframe asks the swapchain for alpha only so): the overlays; the main window
        // paints all of itself
        viewport: gui::viewport(&lg_app::properties::Properties::load(&own)).with_transparent(true),
        wgpu_options,
        ..Default::default()
    };
    // Linux: X11 (XWayland under Wayland), as Java's Swing: only there can the overlays be placed and kept on top
    #[cfg(target_os = "linux")]
    if std::env::var_os("DISPLAY").is_some() && std::env::var_os("LG_WAYLAND").is_none() {
        // SAFETY: no other thread runs yet
        unsafe { std::env::remove_var("WAYLAND_DISPLAY") };
    }
    // no update checks with --offline, unless a test points them elsewhere (LG_UPDATE_API)
    let gui = gui::Gui::new(app, own, assets, !o.offline || std::env::var_os("LG_UPDATE_API").is_some(), offer);
    match eframe::run_native("Loot Gobbler", options, Box::new(|_| Ok(Box::new(gui)))) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            lg_app::applog::err(&format!("cannot open the window: {e}"));
            ExitCode::FAILURE
        }
    }
}

/// The game data in the own folder, extracted if the game's file changed (Java: `AssetCheck`); without the game
/// and without own game data, the Java app's.
/// The setting marking that the first start's greeting offered the Java app's folder ("pending" once taken).
pub const JAVA_OFFER: &str = "javaOffer";

/// Where the Java app keeps its records by default, unless that is this app's own folder: on Windows
/// %APPDATA%\LootGobbler, on Linux ~/.local/share/LootGobbler (its rpm's; the portable folders are unknown).
fn java_folder(own: &Path) -> Option<PathBuf> {
    let dir = if cfg!(windows) {
        PathBuf::from(std::env::var_os("APPDATA")?).join("LootGobbler")
    } else {
        match std::env::var("XDG_DATA_HOME") {
            Ok(x) if x.starts_with('/') => PathBuf::from(x),
            _ => PathBuf::from(std::env::var_os("HOME")?).join(".local").join("share"),
        }
        .join("LootGobbler")
    };
    let same = |a: &Path, b: &Path| matches!((a.canonicalize(), b.canonicalize()), (Ok(x), Ok(y)) if x == y);
    (!same(&dir, own)).then_some(dir)
}

fn own_assets(own: &Path, o: &Options) -> PathBuf {
    let zone = lg_app::localtime::Zone::system();
    let mut props = lg_app::properties::Properties::load(own);
    let game = o.game.clone().or_else(lg_gamedata::extract::find_game_file).or_else(|| {
        // Java keeps the folder you chose but never reads it again
        props.get("realmResPath").map(PathBuf::from).filter(|p| p.is_file())
    });
    match &game {
        Some(g) => lg_app::applog::line("app", &format!("game files: {}", g.display())),
        None => lg_app::applog::err("game files not found; give --path <resources.assets>"),
    }
    let version = concat!("v", env!("CARGO_PKG_VERSION"));
    let started = std::time::Instant::now();
    let mut last = String::new();
    let mut progress = |s: &str| {
        // the counting steps only every so often
        let step = s.trim_end_matches(|c: char| c.is_ascii_digit() || c == 'K' || c == ' ');
        if step != last {
            lg_app::applog::err(&format!("  {s}"));
            last = step.to_string();
        }
    };
    match lg_app::assets::check(own, &mut props, game.as_deref(), version, now_ms(), &zone, &mut progress) {
        Ok(lg_app::assets::Outcome::Extracted) => {
            lg_app::applog::line("app", &format!("game data extracted in {:.1} s", started.elapsed().as_secs_f64()));
        }
        Ok(_) => {}
        Err(e) => lg_app::applog::err(&format!("cannot extract the game data: {e}")),
    }
    let mine = own.join("assets");
    match &o.java {
        Some(j) if !mine.join("ObjectID.list").exists() => j.join("assets"),
        _ => mine,
    }
}

/// One frame: the packets it completes go to the app, and what changed is written.
struct Feeder {
    session: Option<Session>,
    items: Vec<Item>,
    map: Option<String>,
}

impl Feeder {
    fn frame(&mut self, app: &mut lg_app::App, own: &Path, link_type: u32, now: i64, data: &[u8]) {
        self.session.get_or_insert_with(|| Session::new(link_type)).frame(0, data, &mut self.items);
        for item in self.items.drain(..) {
            let Item::Packet { packet, .. } = item else { continue };
            // the mission packets (claim 163, and 164 and 165 from the game), raw with the character playing: to
            // learn how they tell progress and claims apart before each game mode run gets its missions
            if matches!(packet.id, 163..=165) {
                let who = app.world.me().map_or("no character".to_string(), |m| format!("character {}", m.char_id));
                let hex: String = packet.body.iter().take(512).map(|b| format!("{b:02x}")).collect();
                let more = if packet.body.len() > 512 { format!(" (+{} bytes)", packet.body.len() - 512) } else { String::new() };
                lg_app::applog::line("missions", &format!("packet {} ({who}, {} bytes): {hex}{more}", packet.id, packet.body.len()));
            }
            if lg_app::RAW.contains(&packet.id) {
                app.raw(packet.id, &packet.body, now);
            } else if lg_app::FED.contains(&packet.id) || packet.id == lg_app::HELLO {
                let Some(Ok(d)) = lg_protocol::packets::decode(packet.id, &packet.body) else { continue };
                if let Err(e) = app.packet(&d.packet, now) {
                    lg_app::applog::err(&format!("{}: {}", packet.name(), e.0));
                }
            } else {
                continue;
            }
            if let Err(e) = app.save_changed(own) {
                lg_app::applog::err(&format!("cannot save into {}: {e}", own.display()));
            }
            let map = app.world.map.as_ref().map(|m| m.name.clone());
            if map != self.map {
                if let Some(m) = &map {
                    lg_app::applog::line("app", &format!("in {m}"));
                }
                self.map = map;
            }
        }
    }
}

/// Windows: the records folder held while the app runs (a file in it no other process may open). Waits a few seconds
/// for one closing (the app starts itself again on a restart or update); None while another one keeps it.
#[cfg(windows)]
fn hold_folder(own: &Path) -> Option<std::fs::File> {
    use std::os::windows::fs::OpenOptionsExt;
    let path = own.join("lootgobbler-rust.lock");
    for _ in 0..20 {
        match std::fs::OpenOptions::new().create(true).truncate(false).write(true).share_mode(0).open(&path) {
            Ok(f) => return Some(f),
            Err(_) => std::thread::sleep(std::time::Duration::from_millis(250)),
        }
    }
    None
}

/// Windows: the running app's window brought up (in front, restored when minimized).
#[cfg(windows)]
fn show_running() {
    use windows_sys::Win32::UI::WindowsAndMessaging::{FindWindowW, IsIconic, SW_RESTORE, SetForegroundWindow, ShowWindow};
    let title: Vec<u16> = "Loot Gobbler".encode_utf16().chain(Some(0)).collect();
    // SAFETY: plain Win32 calls with a window found by its title; the title outlives the call
    unsafe {
        let h = FindWindowW(std::ptr::null(), title.as_ptr());
        if !h.is_null() {
            // only a minimized one restored (a maximized one stays so)
            if IsIconic(h) != 0 {
                ShowWindow(h, SW_RESTORE);
            }
            SetForegroundWindow(h);
        }
    }
}

fn feeder() -> Feeder {
    Feeder { session: None, items: Vec::new(), map: None }
}

fn capture(app: &mut lg_app::App, own: &Path) -> ExitCode {
    let cap = match lg_capture::live::start("tcp port 2050") {
        Ok(c) => c,
        Err(e) => {
            lg_app::applog::err(&format!("{e}"));
            let pcap_missing = matches!(e, lg_capture::live::Error::NoLibrary(_));
            if cfg!(target_os = "linux") && (pcap_missing || matches!(e, lg_capture::live::Error::Permission(_))) {
                let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("./LootGobbler"));
                eprintln!("Run it as root, or set it up once:\n{}", capture_setup::Plan::now(pcap_missing).manual(&exe));
            }
            return ExitCode::FAILURE;
        }
    };
    let names: Vec<String> = cap.opened.iter().map(|i| i.name.clone()).collect();
    lg_app::applog::line("sniffer", &format!("capturing on {}; start or change servers in the game. Ctrl+C ends.", names.join(", ")));
    let mut f = feeder();
    while let Ok(frame) = cap.frames.recv() {
        f.frame(app, own, frame.link_type, (frame.micros / 1000) as i64, &frame.data);
    }
    ExitCode::SUCCESS
}

/// A recording, its times moved to now (the first frame now, the rest as far apart as they were).
fn play(app: &mut lg_app::App, own: &Path, file: &Path) -> ExitCode {
    let bytes = match std::fs::read(file) {
        Ok(b) => b,
        Err(e) => {
            lg_app::applog::err(&format!("cannot read {}: {e}", file.display()));
            return ExitCode::FAILURE;
        }
    };
    let mut r = match lg_capture::pcap::Reader::new(&bytes) {
        Ok(r) => r,
        Err(e) => {
            lg_app::applog::err(&format!("{}: {e}", file.display()));
            return ExitCode::FAILURE;
        }
    };
    let link = r.link_type;
    let (start, mut first) = (now_ms(), None);
    let mut f = feeder();
    while let Some(frame) = r.next_frame() {
        let frame = match frame {
            Ok(fr) => fr,
            Err(e) => {
                lg_app::applog::err(&format!("{}: {e}", file.display()));
                return ExitCode::FAILURE;
            }
        };
        let t = (frame.micros / 1000) as i64;
        let now = start + t - *first.get_or_insert(t);
        f.frame(app, own, link, now, frame.data);
    }
    ExitCode::SUCCESS
}
