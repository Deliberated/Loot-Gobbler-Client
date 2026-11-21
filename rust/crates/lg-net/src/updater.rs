//! Update check and in-app update from the GitHub releases of Loot-Gobbler-Client (Java `Updater`).
//!
//! Checks shortly after start and then every hour. Updating (Windows) downloads LootGobbler-portable.zip from the
//! latest release, checks it against GitHub's SHA-256, unpacks it to update/new next to the program, makes sure the
//! new program is the version the release says, puts it in place of the running one (which Windows lets be renamed,
//! not overwritten) and starts it; the app then closes. The data files next to the program are not touched.
//! Elsewhere (Linux) the release page is opened, as the Java app does.

use lg_world::json::{self, Json};
use std::cmp::Ordering;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const API: &str = "https://api.github.com/repos/Deliberated/Loot-Gobbler-Client/releases/latest";
const RELEASES_PAGE: &str = "https://github.com/Deliberated/Loot-Gobbler-Client/releases/latest";
const ZIP_NAME: &str = "LootGobbler-portable.zip";
const FIRST_CHECK: Duration = Duration::from_secs(2);
const INTERVAL: Duration = Duration::from_secs(60 * 60);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Idle,
    Checking,
    UpToDate,
    Available,
    Downloading,
    Restarting,
    Failed,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Release {
    pub version: String,
    pub page_url: String,
    pub zip_url: Option<String>,
    pub size: u64,
    pub sha256: Option<String>,
}

/// A published version's notes.
#[derive(Debug, Clone, PartialEq)]
pub struct Notes {
    pub version: String,
    pub published: String,
    pub body: String,
    pub page_url: String,
}

/// Where the releases are asked for: LG_UPDATE_API overrides it for testing (Java: `lootgobbler.updateApi`).
fn api() -> String {
    std::env::var("LG_UPDATE_API").unwrap_or_else(|_| API.to_string())
}

/// Java `compare`: versions like "v0.4.0" and "0.10.1", number by number (a part that is no number counts 0).
pub fn compare(a: &str, b: &str) -> Ordering {
    let parts = |s: &str| -> Vec<i64> {
        let s = s.strip_prefix(['v', 'V']).unwrap_or(s);
        s.split(['.', '-', '+']).map(|p| p.parse::<i64>().unwrap_or(0)).collect()
    };
    let (x, y) = (parts(a), parts(b));
    for i in 0..x.len().max(y.len()) {
        let c = x.get(i).copied().unwrap_or(0).cmp(&y.get(i).copied().unwrap_or(0));
        if c != Ordering::Equal {
            return c;
        }
    }
    Ordering::Equal
}

fn text(o: &Json, key: &str) -> String {
    match o.get(key) {
        Some(Json::String(s)) => s.clone(),
        Some(Json::Number(n)) => n.clone(),
        _ => String::new(),
    }
}

/// Java `fetchLatest`'s reading of GitHub's answer: the tag, the page, and the zip (with its SHA-256 if GitHub has it).
pub fn parse_latest(body: &str) -> Result<Release, String> {
    let j = json::parse(body)?;
    let tag = text(&j, "tag_name");
    if tag.is_empty() {
        return Err("GitHub's answer has no version".into());
    }
    let page = match text(&j, "html_url") {
        p if p.is_empty() => RELEASES_PAGE.to_string(),
        p => p,
    };
    if let Some(Json::Array(assets)) = j.get("assets") {
        for a in assets {
            if text(a, "name") != ZIP_NAME {
                continue;
            }
            let sha = text(a, "digest").strip_prefix("sha256:").map(str::to_string);
            let size = text(a, "size").parse().unwrap_or(0);
            return Ok(Release { version: tag, page_url: page, zip_url: Some(text(a, "browser_download_url")), size, sha256: sha });
        }
    }
    // a release without the zip: only the page can be offered
    Ok(Release { version: tag, page_url: page, zip_url: None, size: 0, sha256: None })
}

/// Java `releaseNotes`' reading: every published version after `from` up to and including `to`, newest first.
pub fn parse_notes(body: &str, from: &str, to: &str) -> Result<Vec<Notes>, String> {
    let Json::Array(list) = json::parse(body)? else { return Err("GitHub's answer is no list".into()) };
    let mut out: Vec<Notes> = list
        .iter()
        .filter(|r| !matches!(r.get("draft"), Some(Json::Bool(true))))
        .map(|r| Notes {
            version: text(r, "tag_name"),
            published: text(r, "published_at"),
            body: text(r, "body"),
            page_url: text(r, "html_url"),
        })
        .filter(|n| compare(&n.version, from) == Ordering::Greater && compare(&n.version, to) != Ordering::Greater)
        .collect();
    out.sort_by(|a, b| compare(&b.version, &a.version));
    Ok(out)
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new().timeout_connect(Duration::from_secs(15)).timeout(Duration::from_secs(20)).build()
}

/// Java `describe`: what went wrong, in a few words.
fn describe(e: &ureq::Error) -> String {
    match e {
        ureq::Error::Status(code, _) => format!("GitHub answered {code}"),
        ureq::Error::Transport(_) => "no connection to GitHub".into(),
    }
}

fn get(url: &str, version: &str) -> Result<Option<String>, String> {
    match agent().get(url).set("Accept", "application/vnd.github+json").set("User-Agent", &format!("LootGobbler/{version}")).call() {
        Ok(r) => r.into_string().map(Some).map_err(|e| e.to_string()),
        Err(ureq::Error::Status(404, _)) => Ok(None), // no release published yet
        Err(e) => Err(describe(&e)),
    }
}

/// The release notes of every published version after `from` up to `to` (Java `releaseNotes`).
pub fn release_notes(from: &str, to: &str, version: &str) -> Result<Vec<Notes>, String> {
    let list = format!("{}?per_page=50", api().trim_end_matches("/latest"));
    match get(&list, version)? {
        Some(body) => parse_notes(&body, from, to),
        None => Ok(Vec::new()),
    }
}

/// The releases page (Java `releasesPage`).
pub fn releases_page() -> String {
    RELEASES_PAGE.trim_end_matches("/latest").to_string()
}

#[derive(Debug)]
struct Inner {
    state: State,
    latest: Option<Release>,
    progress: f64,
    message: String,
    started: bool,
    /// A manual check whose answer the About dialog waits for.
    answer: Option<String>,
}

/// The updater: its checks and the install run on threads of their own; the window reads its state each frame.
#[derive(Clone)]
pub struct Updater {
    version: String,
    inner: Arc<Mutex<Inner>>,
}

impl Updater {
    pub fn new(version: &str) -> Updater {
        Updater {
            version: version.to_string(),
            inner: Arc::new(Mutex::new(Inner {
                state: State::Idle,
                latest: None,
                progress: 0.0,
                message: String::new(),
                started: false,
                answer: None,
            })),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn state(&self) -> State {
        self.lock().state
    }

    pub fn progress(&self) -> f64 {
        self.lock().progress
    }

    /// What went wrong for Failed, otherwise a short description of the state.
    pub fn message(&self) -> String {
        self.lock().message.clone()
    }

    pub fn latest(&self) -> Option<Release> {
        self.lock().latest.clone()
    }

    /// The answer to a check asked from Info > About, once it is there (taken).
    pub fn take_answer(&self) -> Option<String> {
        self.lock().answer.take()
    }

    fn set(&self, s: State, text: &str) {
        let mut i = self.lock();
        if s != i.state || text != i.message {
            lg_app::applog::line("update", &format!("{s:?}: {text}"));
        }
        i.state = s;
        i.message = text.to_string();
        if s != State::Downloading {
            i.progress = 0.0;
        }
    }

    /// The automatic checks: right after start (the button shows CHECKING... until then), then every hour.
    pub fn start(&self) {
        {
            let mut i = self.lock();
            if i.started {
                return;
            }
            i.started = true;
        }
        self.set(State::Checking, "Checking for updates");
        let me = self.clone();
        std::thread::Builder::new()
            .name("update-check".into())
            .spawn(move || {
                std::thread::sleep(FIRST_CHECK);
                loop {
                    me.check(false);
                    std::thread::sleep(INTERVAL);
                }
            })
            .ok();
    }

    /// The status bar's button: updates when one is available, otherwise checks now. Gives a page to open, if any.
    pub fn clicked(&self) -> Option<String> {
        match self.state() {
            State::Available => self.install(),
            State::Idle | State::UpToDate | State::Failed => {
                let me = self.clone();
                std::thread::spawn(move || me.check(true));
                None
            }
            _ => None, // busy
        }
    }

    /// Info > About's "Check for updates": checks now; the answer comes with `take_answer`.
    pub fn check_and_answer(&self) {
        let me = self.clone();
        std::thread::spawn(move || {
            me.check(true);
            let text = match me.state() {
                State::Available => format!(
                    "Loot Gobbler {} is out (you have {}).\n\nClick UPDATE AVAILABLE at the bottom right to update and restart.",
                    me.latest().map_or(String::new(), |r| r.version),
                    me.version
                ),
                State::Failed => format!("Could not check for updates: {}", me.message()),
                _ => format!("You have the newest version, Loot Gobbler {}.", me.version),
            };
            me.lock().answer = Some(text);
        });
    }

    /// Asks GitHub for the latest release. An automatic check that fails keeps the button as it was, except for
    /// the first one (then it falls back to Idle, "check updates").
    fn check(&self, manual: bool) {
        let before = self.state();
        if matches!(before, State::Downloading | State::Restarting) {
            return;
        }
        if manual {
            self.set(State::Checking, "Checking for updates");
        }
        let found = get(&api(), &self.version).and_then(|b| b.map(|b| parse_latest(&b)).transpose());
        match found {
            Ok(Some(r)) if compare(&r.version, &self.version) == Ordering::Greater => {
                let text = format!("Loot Gobbler {} is out. Click to update and restart.", r.version);
                self.lock().latest = Some(r);
                self.set(State::Available, &text);
            }
            Ok(_) => self.set(State::UpToDate, &format!("You have the newest version ({}). Click to check again.", self.version)),
            Err(e) if manual => self.set(State::Failed, &e),
            Err(e) if before == State::Checking => self.set(State::Idle, &format!("Could not check for updates: {e}")),
            Err(_) => {}
        }
    }

    /// Downloads and unpacks the new version and starts it (Windows); elsewhere, or for a release without the zip,
    /// gives the release page to open instead.
    fn install(&self) -> Option<String> {
        let r = self.latest()?;
        if !cfg!(windows) || r.zip_url.is_none() {
            return Some(r.page_url);
        }
        self.set(State::Downloading, &format!("Downloading Loot Gobbler {}", r.version));
        let me = self.clone();
        std::thread::spawn(move || {
            if let Err(e) = me.download_and_swap(&r) {
                me.set(State::Failed, &format!("Update failed: {e}"));
                me.lock().latest = Some(r);
            }
        });
        None
    }

    fn download_and_swap(&self, r: &Release) -> Result<(), String> {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let app_dir = exe.parent().ok_or("no program folder")?.to_path_buf();
        let update_dir = app_dir.join("update");
        let _ = std::fs::remove_dir_all(&update_dir);
        std::fs::create_dir_all(&update_dir).map_err(|e| writable(&e))?;
        let zip = update_dir.join(ZIP_NAME);
        self.download(r, &zip)?;
        let new_dir = update_dir.join("new");
        unzip(&zip, &new_dir)?;
        let name = exe.file_name().ok_or("no program name")?;
        let new_exe = [new_dir.join(name), new_dir.join("LootGobbler.exe")].into_iter().find(|p| p.is_file());
        let new_exe = new_exe.ok_or("the download does not contain Loot Gobbler")?;
        // a release built before the version was raised would install the old version again, and the app would
        // offer the same update after every restart
        if let Ok(out) = std::process::Command::new(&new_exe).arg("--version").output() {
            let inside = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !inside.is_empty() && compare(&inside, &r.version) != Ordering::Equal {
                return Err(format!("release {} contains version {inside} - it has to be rebuilt with the right version", r.version));
            }
        }
        self.set(State::Restarting, &format!("Restarting with Loot Gobbler {}", r.version));
        // the running program can be renamed, not overwritten: it goes aside (removed on the next start)
        let old = exe.with_extension("exe.old");
        let _ = std::fs::remove_file(&old);
        std::fs::rename(&exe, &old).map_err(|e| writable(&e))?;
        let copied = copy_tree(&new_dir, &app_dir);
        let target = app_dir.join(new_exe.file_name().ok_or("no program name")?);
        if let Err(e) = copied.and_then(|_| std::process::Command::new(&target).spawn().map(|_| ()).map_err(|e| e.to_string())) {
            // put the old one back
            let _ = std::fs::rename(&old, &exe);
            return Err(e);
        }
        Ok(())
    }

    fn download(&self, r: &Release, target: &Path) -> Result<(), String> {
        let url = r.zip_url.as_deref().ok_or("no download")?;
        let res = ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(15))
            .timeout(Duration::from_secs(600))
            .build()
            .get(url)
            .set("User-Agent", &format!("LootGobbler/{}", self.version))
            .call()
            .map_err(|e| match e {
                ureq::Error::Status(code, _) => format!("download answered {code}"),
                ureq::Error::Transport(_) => "no connection to GitHub".into(),
            })?;
        let total = if r.size > 0 { r.size } else { res.header("Content-Length").and_then(|v| v.parse().ok()).unwrap_or(0) };
        let mut input = res.into_reader();
        let mut out = std::fs::File::create(target).map_err(|e| writable(&e))?;
        let mut sha = Sha256::new();
        let mut buf = vec![0u8; 64 * 1024];
        let (mut done, mut last) = (0u64, 0u64);
        loop {
            let n = input.read(&mut buf).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            out.write_all(&buf[..n]).map_err(|e| writable(&e))?;
            sha.update(&buf[..n]);
            done += n as u64;
            if total > 0 && done - last > total / 200 {
                last = done;
                self.lock().progress = (done as f64 / total as f64).min(1.0);
            }
        }
        self.lock().progress = 1.0;
        if let Some(want) = &r.sha256
            && !sha.hex().eq_ignore_ascii_case(want)
        {
            return Err("the download is damaged (checksum does not match)".into());
        }
        Ok(())
    }
}

fn writable(e: &std::io::Error) -> String {
    if e.kind() == std::io::ErrorKind::PermissionDenied { "Loot Gobbler's folder is not writable".into() } else { e.to_string() }
}

/// Removes what an update left: the program put aside and the downloaded files (on start).
pub fn clean_up() {
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        let _ = std::fs::remove_file(exe.with_extension("exe.old"));
        let _ = std::fs::remove_dir_all(dir.join("update"));
    }
}

fn copy_tree(from: &Path, to: &Path) -> Result<(), String> {
    for e in std::fs::read_dir(from).map_err(|e| e.to_string())? {
        let e = e.map_err(|e| e.to_string())?;
        let target = to.join(e.file_name());
        if e.path().is_dir() {
            std::fs::create_dir_all(&target).map_err(|e| writable(&e))?;
            copy_tree(&e.path(), &target)?;
        } else {
            std::fs::copy(e.path(), &target).map_err(|e| writable(&e))?;
        }
    }
    Ok(())
}

/// Java `unzip`: unpacks the zip into `dir`, dropping its top folder (LootGobbler/...); stored and deflated entries.
pub fn unzip(zip: &Path, dir: &Path) -> Result<(), String> {
    let data = std::fs::read(zip).map_err(|e| e.to_string())?;
    let bad = || "the download is not a zip".to_string();
    let u16_at = |i: usize| data.get(i..i + 2).map(|b| u16::from_le_bytes([b[0], b[1]]) as usize).ok_or_else(bad);
    let u32_at = |i: usize| data.get(i..i + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize).ok_or_else(bad);
    // the end of the central directory, searched from the end (a comment may follow it)
    let eocd = (0..data.len().saturating_sub(21)).rev().find(|&i| data[i..].starts_with(&[0x50, 0x4b, 0x05, 0x06])).ok_or_else(bad)?;
    let entries = u16_at(eocd + 10)?;
    let mut at = u32_at(eocd + 16)?;
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    for _ in 0..entries {
        if u32_at(at)? != 0x0201_4b50 {
            return Err(bad());
        }
        let method = u16_at(at + 10)?;
        let packed = u32_at(at + 20)?;
        let (name_len, extra_len, comment_len) = (u16_at(at + 28)?, u16_at(at + 30)?, u16_at(at + 32)?);
        let local = u32_at(at + 42)?;
        let name = String::from_utf8_lossy(data.get(at + 46..at + 46 + name_len).ok_or_else(bad)?).replace('\\', "/");
        at += 46 + name_len + extra_len + comment_len;
        let Some(slash) = name.find('/') else { continue };
        let rest = &name[slash + 1..];
        if rest.is_empty() {
            continue; // the top folder itself
        }
        if rest.split('/').any(|p| p == "..") {
            return Err(format!("bad path in the download: {name}"));
        }
        let out: PathBuf = dir.join(rest);
        if name.ends_with('/') {
            std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
            continue;
        }
        let start = local + 30 + u16_at(local + 26)? + u16_at(local + 28)?;
        let raw = data.get(start..start + packed).ok_or_else(bad)?;
        let bytes = match method {
            0 => raw.to_vec(),
            8 => {
                let mut v = Vec::new();
                flate2::read::DeflateDecoder::new(raw).read_to_end(&mut v).map_err(|_| bad())?;
                v
            }
            m => return Err(format!("the download uses an unknown packing ({m})")),
        };
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(&out, bytes).map_err(|e| writable(&e))?;
    }
    Ok(())
}

/// SHA-256 (FIPS 180-4), for the download's checksum.
pub struct Sha256 {
    h: [u32; 8],
    buf: Vec<u8>,
    len: u64,
}

const K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be,
    0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa,
    0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85,
    0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3,
    0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f,
    0x682e6ff3, 0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

impl Default for Sha256 {
    fn default() -> Self {
        Self::new()
    }
}

impl Sha256 {
    pub fn new() -> Sha256 {
        Sha256 {
            h: [0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19],
            buf: Vec::new(),
            len: 0,
        }
    }

    fn block(&mut self, b: &[u8]) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([b[4 * i], b[4 * i + 1], b[4 * i + 2], b[4 * i + 3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
        }
        let [mut a, mut b2, mut c, mut d, mut e, mut f, mut g, mut h] = self.h;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let t1 = h.wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b2) ^ (a & c) ^ (b2 & c);
            let t2 = s0.wrapping_add(maj);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b2;
            b2 = a;
            a = t1.wrapping_add(t2);
        }
        for (x, v) in self.h.iter_mut().zip([a, b2, c, d, e, f, g, h]) {
            *x = x.wrapping_add(v);
        }
    }

    pub fn update(&mut self, data: &[u8]) {
        self.len += data.len() as u64;
        self.buf.extend_from_slice(data);
        let full = self.buf.len() / 64 * 64;
        let blocks: Vec<u8> = self.buf.drain(..full).collect();
        for b in blocks.chunks(64) {
            self.block(b);
        }
    }

    /// The digest as lower-case hex.
    pub fn hex(mut self) -> String {
        let bits = self.len * 8;
        let mut pad = vec![0x80u8];
        while (self.buf.len() + pad.len()) % 64 != 56 {
            pad.push(0);
        }
        pad.extend_from_slice(&bits.to_be_bytes());
        let len = self.len;
        self.update(&pad);
        self.len = len;
        self.h.iter().map(|x| format!("{x:08x}")).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_compare_as_java() {
        assert_eq!(compare("v0.10.1", "0.9.9"), Ordering::Greater);
        assert_eq!(compare("v1.0", "1.0.0"), Ordering::Equal);
        assert_eq!(compare("0.0.1", "v0.0.2"), Ordering::Less);
        assert_eq!(compare("v0.9.1-beta", "0.9.1"), Ordering::Equal);
    }

    #[test]
    fn sha256_known_digests() {
        let h = |s: &[u8]| {
            let mut x = Sha256::new();
            x.update(s);
            x.hex()
        };
        assert_eq!(h(b""), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
        assert_eq!(h(b"abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
        let long: Vec<u8> = (0..1000u32).map(|i| (i % 251) as u8).collect();
        let mut x = Sha256::new();
        for c in long.chunks(37) {
            x.update(c);
        }
        assert_eq!(x.hex(), h(&long));
    }

    #[test]
    fn github_answers_are_read() {
        let latest = r#"{"tag_name":"v0.10.0","html_url":"https://x/r/v0.10.0","assets":[
            {"name":"LootGobbler.exe","browser_download_url":"https://x/a.exe","size":5},
            {"name":"LootGobbler-portable.zip","browser_download_url":"https://x/p.zip","size":1234,"digest":"sha256:abcd"}]}"#;
        let r = parse_latest(latest).unwrap();
        assert_eq!(r.version, "v0.10.0");
        assert_eq!(r.zip_url.as_deref(), Some("https://x/p.zip"));
        assert_eq!((r.size, r.sha256.as_deref()), (1234, Some("abcd")));
        let list = r#"[{"tag_name":"v0.3.0","body":"c"},{"tag_name":"v0.2.0","body":"b","draft":true},
            {"tag_name":"v0.1.0","body":"a"},{"tag_name":"v0.4.0","body":"d"}]"#;
        let n = parse_notes(list, "0.1.0", "0.3.0").unwrap();
        assert_eq!(n.iter().map(|x| x.version.as_str()).collect::<Vec<_>>(), vec!["v0.3.0"]);
        let n = parse_notes(list, "0.0.0", "9.9.9").unwrap();
        assert_eq!(n.iter().map(|x| x.version.as_str()).collect::<Vec<_>>(), vec!["v0.4.0", "v0.3.0", "v0.1.0"]);
    }

    #[test]
    fn unzips_dropping_the_top_folder() {
        // a zip made by hand: LootGobbler/a.txt (stored) and LootGobbler/sub/b.txt (deflated)
        fn entry(name: &str, data: &[u8], deflate: bool) -> (Vec<u8>, Vec<u8>, u32) {
            let packed = if deflate {
                let mut e = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
                e.write_all(data).unwrap();
                e.finish().unwrap()
            } else {
                data.to_vec()
            };
            (name.as_bytes().to_vec(), packed, data.len() as u32)
        }
        let files = [entry("LootGobbler/a.txt", b"hello", false), entry("LootGobbler/sub/b.txt", &b"world ".repeat(50), true)];
        let mut zip = Vec::new();
        let mut central = Vec::new();
        for (i, (name, packed, size)) in files.iter().enumerate() {
            let method: u16 = if i == 1 { 8 } else { 0 };
            let offset = zip.len() as u32;
            zip.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
            zip.extend_from_slice(&[20, 0, 0, 0]);
            zip.extend_from_slice(&method.to_le_bytes());
            zip.extend_from_slice(&[0; 8]);
            zip.extend_from_slice(&(packed.len() as u32).to_le_bytes());
            zip.extend_from_slice(&size.to_le_bytes());
            zip.extend_from_slice(&(name.len() as u16).to_le_bytes());
            zip.extend_from_slice(&0u16.to_le_bytes());
            zip.extend_from_slice(name);
            zip.extend_from_slice(packed);
            central.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
            central.extend_from_slice(&[20, 0, 20, 0, 0, 0]);
            central.extend_from_slice(&method.to_le_bytes());
            central.extend_from_slice(&[0; 8]);
            central.extend_from_slice(&(packed.len() as u32).to_le_bytes());
            central.extend_from_slice(&size.to_le_bytes());
            central.extend_from_slice(&(name.len() as u16).to_le_bytes());
            central.extend_from_slice(&[0; 12]);
            central.extend_from_slice(&offset.to_le_bytes());
            central.extend_from_slice(name);
        }
        let cd_at = zip.len() as u32;
        zip.extend_from_slice(&central);
        zip.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
        zip.extend_from_slice(&[0, 0, 0, 0]);
        zip.extend_from_slice(&2u16.to_le_bytes());
        zip.extend_from_slice(&2u16.to_le_bytes());
        zip.extend_from_slice(&(central.len() as u32).to_le_bytes());
        zip.extend_from_slice(&cd_at.to_le_bytes());
        zip.extend_from_slice(&[0, 0]);
        let dir = std::env::temp_dir().join(format!("lg-unzip-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let z = dir.join("t.zip");
        std::fs::write(&z, &zip).unwrap();
        unzip(&z, &dir.join("out")).unwrap();
        assert_eq!(std::fs::read(dir.join("out/a.txt")).unwrap(), b"hello");
        assert_eq!(std::fs::read(dir.join("out/sub/b.txt")).unwrap(), b"world ".repeat(50));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
