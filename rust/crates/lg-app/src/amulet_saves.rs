//! The times your Amulet of Restoration saved you (Java: `AmuletSaves`). The game data says when it fires: on a hit
//! of more than 2 damage taken below 20% of your life it makes you Invulnerable for 2 seconds and heals 120, at most
//! once every 30 minutes. So a save is the game putting Invulnerable on you while you wear it, if you were hit just
//! before, your life was at 20% or below, the amulet's last save is 30 minutes ago or more, and the game shows its
//! "+120" heal on you within 1.5 seconds of it. Game notifications about you and system texts that mention the amulet
//! count too. Nothing is guessed: a save is only what the game itself sent (your life and max life, the Invulnerable
//! condition on you, its "+120" on you), and each of these is written to the app log while you wear the amulet, so
//! a save that was or was not counted can be checked afterwards. Each save is also marked on the dungeon visit; the
//! window plays its sound, and another when the amulet is charged again.

use crate::dungeon_visits::DungeonVisits;
use lg_world::json::{self, Json};
use lg_world::{World, stats};
use std::collections::VecDeque;
use std::path::Path;

const LOW_LIFE_PERCENT: i64 = 20;
const HEAL: i32 = 120;
const MIN_HIT: i32 = 2;
pub const RECHARGE_MS: i64 = 1_800_000;
const LOOK_BACK_MS: i64 = 1500;
/// "Played lately": the last two weeks.
const RECENT_MS: i64 = 14 * 24 * 3_600_000;
/// The "+120" comes with the Invulnerable, before or after it.
const HEAL_WINDOW_MS: i64 = 1500;
/// What the first version counted: any Invulnerable while wearing it, buffs included.
const OLD_RULE: &str = ": Invulnerable (";
const INVULNERABLE: i32 = 0x100_0000;
const HEALING: i32 = 0x2_0000;
const BERSERK: i32 = 0x8_0000;
const ARMORED: i32 = 0x200_0000;
/// The amulet's other proc, Last Stand: on shooting below 20% life, Healing, Berserk and Armored together for 4 s
/// (every 30 s). It gives no Invulnerable, so it is never taken for a save; it is logged on its own.
const LAST_STAND: i32 = HEALING | BERSERK | ARMORED;
/// What the trace keeps: the last moments before a proc, and the moments after it that are logged as they come.
const TRACE_MS: i64 = 3000;
const TRACE_AFTER_MS: i64 = 2000;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Save {
    pub t: i64,
    pub char_id: i32,
    pub amulet: String,
    pub dungeon: String,
    pub message: String,
    /// Turned out not to be a save; kept so that syncing does not bring it back.
    pub removed: bool,
}

#[derive(Debug, Default)]
pub struct AmuletSaves {
    /// Java would write its file now (its `save()`): the app writes it.
    pub dirty: bool,

    /// Oldest first.
    pub saves: Vec<Save>,
    last_heal_at: i64,
    /// The amount of that heal number (120, or less when something cuts your healing).
    last_heal: i32,
    /// An Invulnerable that passed the other checks, waiting for the heal.
    pending_at: i64,
    pending: Option<Pending>,
    was_invulnerable: bool,
    /// Your life was at 20% or below (logged once per dip).
    was_low: bool,
    /// Healing, Berserk and Armored were all on you (Last Stand).
    was_last_stand: bool,
    /// While you wear the amulet, what the game sent about you (life, conditions, hits, heal numbers) over the last
    /// moments: written to the log when a proc may have happened, so it can be checked afterwards.
    trace: VecDeque<(i64, String)>,
    /// The trace is logged up to here; and as it comes until `trace_live_until`.
    trace_logged: i64,
    trace_live_until: i64,
    /// What the trace saw last: life, max life, conditions, the newest hit.
    traced: (i32, i32, i32, i64),
    /// Your life over the last moments: time, hp, max hp.
    life: VecDeque<(i64, i32, i32)>,
}

/// An Invulnerable waiting for its heal: the amulet's name and your life at that moment.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Pending {
    amulet: String,
    hp: i32,
    max: i32,
}

/// Java's `Math.round`: half up.
fn java_round(x: f64) -> i64 {
    let f = x.floor();
    if x - f >= 0.5 { f as i64 + 1 } else { f as i64 }
}

/// The game's heal number on you, `{"k":"s.plus_symbol","t":{"amount":"120",}}`: its amount if it could be the
/// amulet's (120, or less when something cuts your healing, e.g. +60 under a healing debuff in a dungeon).
fn amulet_heal(s: &str) -> Option<i32> {
    if !s.contains("plus_symbol") {
        return None;
    }
    let flat = s.replace(' ', "");
    let rest = &flat[flat.find("\"amount\":\"")? + 10..];
    let amount: i32 = rest[..rest.find('"')?].parse().ok()?;
    (1..=HEAL).contains(&amount).then_some(amount)
}

fn mentions_amulet(s: &str) -> bool {
    s.to_lowercase().contains("restoration")
}

impl AmuletSaves {
    /// What the Java app kept (`amulet_saves.json`), read only; saves the first version counted from Invulnerable
    /// alone are marked removed, as Java does on starting.
    pub fn import(dir: &Path) -> AmuletSaves {
        let mut a = AmuletSaves::default();
        if let Ok(text) = std::fs::read_to_string(dir.join("amulet_saves.json"))
            && let Some(saves) = parse_saves(&text)
        {
            a.saves = saves;
        }
        for s in &mut a.saves {
            if !s.removed && s.message.contains(OLD_RULE) {
                s.removed = true;
                a.dirty = true;
            }
        }
        a
    }

    /// Java `notification`: the amulet's heal on you (which a waiting Invulnerable needed), or a notification about
    /// you that mentions it.
    pub fn notification(&mut self, w: &World, visits: &mut DungeonVisits, n: &lg_protocol::packets::Notification, now: i64) {
        let msg = n.message.as_deref().unwrap_or("");
        let mine = w.me().is_some_and(|p| n.object_id == p.id || n.sender_object_id == p.id);
        if mine && msg.contains("plus_symbol") && !self.trace.is_empty() {
            let flat = msg.replace(' ', "");
            let amount = flat.find("\"amount\":\"").and_then(|i| flat[i + 10..].split('"').next().map(str::to_string));
            self.note(now, format!("heal number +{}", amount.unwrap_or_else(|| "?".into())));
        }
        if mine && let Some(amount) = amulet_heal(msg) {
            crate::applog::line("amulet", &format!("The game shows +{amount} on you"));
            self.last_heal_at = now;
            self.last_heal = amount;
            if let Some(p) = self.take_pending(now) {
                // your life before the heal: what the game showed then, less the heal it shows now
                let lowest = self.lowest_percent(now, p.hp, p.max, amount);
                if lowest <= LOW_LIFE_PERCENT {
                    crate::applog::line("amulet", &format!("Its +{amount} came: counted (life {lowest}% before it)"));
                    self.record(w, visits, &format!("{}: hit at {lowest}% life, Invulnerable, +{amount}", p.amulet), now);
                } else {
                    crate::applog::line("amulet", &format!("A +{amount} came, but your life was not at 20% or below ({lowest}%): not counted"));
                }
            }
        }
        if mine && mentions_amulet(msg) {
            self.record(w, visits, msg, now);
        }
    }

    /// Java `text`: a text the game sent (no player name) that mentions the amulet.
    pub fn text(&mut self, w: &World, visits: &mut DungeonVisits, t: &lg_protocol::packets::Text, now: i64) {
        let system = t.name.chars().all(char::is_whitespace) || t.name.starts_with('*');
        if system && mentions_amulet(&t.text) {
            self.record(w, visits, &t.text, now);
        }
    }

    /// Java `tick`, every tick: the game putting Invulnerable on you while you wear the amulet.
    pub fn tick(&mut self, w: &World, visits: &mut DungeonVisits, now: i64) {
        let Some(me) = w.me() else { return };
        let amulet = me.value(stats::INVENTORY_0 + 3).and_then(|r| w.data().amulets.of(r));
        let invulnerable = me.value(stats::CONDITION).is_some_and(|c| c & INVULNERABLE != 0);
        let (hp, max) = (me.value(stats::HP).unwrap_or(0), me.value(stats::MAX_HP).unwrap_or(0));
        let worn = amulet.filter(|a| a.invulnerable());
        let cond = me.value(stats::CONDITION).unwrap_or(0);
        if worn.is_some() {
            // the trace: your life and the procs' conditions as they change, and each hit on you
            let newest = me.damage_list.iter().map(|d| d.time).max().unwrap_or(0);
            let (h0, m0, c0, hit0) = self.traced;
            let hits: Vec<String> = me
                .damage_list
                .iter()
                .filter(|d| d.time > hit0)
                .map(|d| format!("hit {} by {}", d.damage, d.owner_name.as_deref().unwrap_or("?")))
                .collect();
            for h in hits {
                self.note(now, h);
            }
            if (hp, max, cond & (LAST_STAND | INVULNERABLE)) != (h0, m0, c0) {
                self.note(now, format!("life {hp}/{max}, {}", Self::condition_names(cond)));
            }
            self.traced = (hp, max, cond & (LAST_STAND | INVULNERABLE), newest.max(hit0));
        } else {
            self.trace.clear();
        }
        // Last Stand: Healing, Berserk and Armored put on you together, your life low
        let last_stand = cond & LAST_STAND == LAST_STAND;
        if let Some(a) = worn
            && last_stand
            && !self.was_last_stand
        {
            self.log_trace(now, &format!("{}: Last Stand (Healing, Berserk, Armored) at life {hp}/{max}", a.name));
        }
        self.was_last_stand = last_stand;
        // your life at 20% or below while wearing it: when the game should fire it (logged once per dip)
        let low = max > 0 && hp > 0 && i64::from(hp) * 100 <= LOW_LIFE_PERCENT * i64::from(max);
        if let Some(a) = worn
            && low
            && !self.was_low
        {
            let charge = match self.charging_until(w, now) {
                Some(until) => format!("still charging for {} s", (until - now) / 1000),
                None => "charged".into(),
            };
            self.log_trace(now, &format!("Life {hp}/{max} ({}%) wearing the {} ({charge})", i64::from(hp) * 100 / i64::from(max), a.name));
        }
        self.was_low = low;
        if let Some(a) = worn
            && invulnerable
            && !self.was_invulnerable
            && max > 0
        {
            self.log_trace(now, &format!("Invulnerable on you at life {hp}/{max}"));
            // the heal number may have come just before (then your life is checked now) or still come
            let heal = (now - self.last_heal_at <= HEAL_WINDOW_MS).then_some(self.last_heal);
            match (self.not_the_amulet(w, now, hp, max, heal), heal) {
                (Some(why), _) => crate::applog::line("amulet", &format!("Invulnerable on you at life {hp}/{max}, not counted: {why}")),
                (None, Some(amount)) => {
                    let lowest = self.lowest_percent(now, hp, max, amount);
                    crate::applog::line("amulet", &format!("Invulnerable on you at life {hp}/{max} with its +{amount}: counted"));
                    self.record(w, visits, &format!("{}: hit at {lowest}% life, Invulnerable, +{amount}", a.name), now);
                }
                (None, None) => {
                    crate::applog::line("amulet", &format!("Invulnerable on you at life {hp}/{max}: waiting for its heal number"));
                    self.pending_at = now;
                    self.pending = Some(Pending { amulet: a.name.clone(), hp, max });
                }
            }
        }
        self.was_invulnerable = invulnerable;
        if self.pending_at > 0 && now - self.pending_at > HEAL_WINDOW_MS {
            crate::applog::line("amulet", "No heal number within 1.5 s of the Invulnerable: not counted (e.g. going to the Nexus)");
            self.pending_at = 0;
            self.pending = None;
        }
        if max > 0 {
            self.life.push_back((now, hp, max));
        }
        while self.life.front().is_some_and(|l| now - l.0 > LOOK_BACK_MS) {
            self.life.pop_front();
        }
    }

    /// You died: logged with your last life and whether an Invulnerable was still waiting for its heal number (not
    /// counted: a save needs the game's own heal number).
    pub fn died(&mut self, w: &World, now: i64) {
        let Some(me) = w.me() else { return };
        if me.value(stats::INVENTORY_0 + 3).and_then(|r| w.data().amulets.of(r)).is_none_or(|a| !a.invulnerable()) {
            return;
        }
        let (hp, max) = (me.value(stats::HP).unwrap_or(0), me.value(stats::MAX_HP).unwrap_or(0));
        let waiting = if self.pending_at > 0 && now - self.pending_at <= HEAL_WINDOW_MS {
            "an Invulnerable was waiting for its heal number, which never came: not counted"
        } else {
            "the game showed no amulet Invulnerable before it"
        };
        self.log_trace(now, &format!("You died wearing the amulet, last life {hp}/{max}: {waiting}"));
    }

    /// When your character's amulet is charged again after its last save, if it is still charging.
    fn charging_until(&self, w: &World, now: i64) -> Option<i64> {
        let me = w.me()?;
        let last = self.saves.iter().rev().find(|s| !s.removed && s.char_id == me.char_id)?;
        (now - last.t < RECHARGE_MS).then_some(last.t + RECHARGE_MS)
    }

    /// Java `notTheAmulet`: why this Invulnerable is not the amulet firing (None: it is). It is not within the
    /// cooldown of your character's last save, you were hit just before, and (once its heal number is known) your
    /// life was low.
    fn not_the_amulet(&self, w: &World, now: i64, hp: i32, max: i32, heal: Option<i32>) -> Option<String> {
        let me = w.me()?;
        if let Some(until) = self.charging_until(w, now) {
            return Some(format!("the amulet is charging for {} s more", (until - now) / 1000));
        }
        if !me.damage_list.iter().any(|d| d.damage > MIN_HIT && now - d.time <= LOOK_BACK_MS) {
            return Some("no hit on you just before".into());
        }
        let lowest = self.lowest_percent(now, hp, max, heal?);
        (lowest > LOW_LIFE_PERCENT).then(|| format!("your life was not at 20% or below (lowest {lowest}%)"))
    }

    /// Your lowest life in the last moments, in %: what the game showed, and your life now less the heal it showed.
    fn lowest_percent(&self, now: i64, hp: i32, max: i32, heal: i32) -> i64 {
        let mut low = f64::from((hp - heal).max(0)) / f64::from(max);
        for &(t, h, m) in &self.life {
            if now - t <= LOOK_BACK_MS {
                low = low.min(f64::from(h) / f64::from(m));
            }
        }
        java_round(low * 100.0)
    }

    /// One thing the game sent about you, into the trace (logged at once just after a proc).
    fn note(&mut self, now: i64, what: String) {
        if now <= self.trace_live_until {
            crate::applog::line("amulet", &format!("  then: {what}"));
            self.trace_logged = now;
        }
        self.trace.push_back((now, what));
        while self.trace.front().is_some_and(|t| now - t.0 > TRACE_MS) {
            self.trace.pop_front();
        }
    }

    /// Writes the trace up to now to the log (what came before `why`), and what comes in the next moments.
    fn log_trace(&mut self, now: i64, why: &str) {
        crate::applog::line("amulet", why);
        for (t, what) in &self.trace {
            if *t > self.trace_logged {
                crate::applog::line("amulet", &format!("  {} ms before: {what}", now - t));
            }
        }
        self.trace_logged = now;
        self.trace_live_until = now + TRACE_AFTER_MS;
    }

    /// The conditions the amulet's procs give, as names.
    fn condition_names(c: i32) -> String {
        let names: Vec<&str> = [(INVULNERABLE, "Invulnerable"), (HEALING, "Healing"), (BERSERK, "Berserk"), (ARMORED, "Armored")]
            .iter()
            .filter(|(bit, _)| c & bit != 0)
            .map(|(_, n)| *n)
            .collect();
        if names.is_empty() { "-".into() } else { names.join(", ") }
    }

    fn take_pending(&mut self, t: i64) -> Option<Pending> {
        if self.pending_at == 0 || (t - self.pending_at).abs() > HEAL_WINDOW_MS {
            return None;
        }
        self.pending_at = 0;
        self.pending.take()
    }

    /// Java `record`: one save per few seconds (the same can come as a notification and a text).
    fn record(&mut self, w: &World, visits: &mut DungeonVisits, message: &str, now: i64) {
        let mut s = Save { t: now, message: message.to_string(), ..Default::default() };
        if let Some(me) = w.me() {
            s.char_id = me.char_id;
            if let Some(a) = me.value(stats::INVENTORY_0 + 3).and_then(|r| w.data().amulets.of(r)) {
                s.amulet = a.name.clone();
            }
        }
        s.dungeon = match (&visits.current, &w.map) {
            (Some(v), _) => v.dungeon.clone(),
            (None, Some(m)) => m.name.clone(),
            (None, None) => String::new(),
        };
        for old in self.saves.iter().rev() {
            if now - old.t >= 3000 {
                break;
            }
            if !old.removed {
                return;
            }
        }
        self.saves.push(s);
        self.dirty = true;
        visits.amulet_saved(now);
    }

    /// Java `mergeSync`: another PC's saves taken in; one counted by the first version of the rule, or removed there,
    /// is removed here too (and unmarked in its visit). True if anything changed.
    pub fn merge_sync(&mut self, remote: &Json, visits: &mut DungeonVisits, now: i64) -> Result<bool, crate::java::Thrown> {
        if *remote == Json::Null {
            return Ok(false);
        }
        let theirs = parse_saves(&remote.write()).ok_or(crate::java::Thrown)?;
        let mut changed = false;
        for mut s in theirs {
            match self.saves.iter_mut().find(|m| m.t == s.t) {
                None => {
                    if s.message.contains(OLD_RULE) && !s.removed {
                        s.removed = true; // counted by the first version from another client
                        visits.amulet_save_removed(s.t, now);
                    }
                    self.saves.push(s);
                    changed = true;
                }
                Some(mine) => {
                    if s.removed && !mine.removed {
                        mine.removed = true;
                        visits.amulet_save_removed(s.t, now);
                        changed = true;
                    }
                }
            }
        }
        if changed {
            self.saves.sort_by_key(|s| s.t);
            self.dirty = true;
        }
        Ok(changed)
    }

    /// Java `removedAt`: a save near `t` was found not to be one.
    pub fn removed_at(&self, t: i64) -> bool {
        self.saves.iter().any(|s| s.removed && (s.t - t).abs() <= 3000)
    }

    /// Java `count`: every save kept.
    pub fn count(&self) -> usize {
        self.saves.iter().filter(|s| !s.removed).count()
    }

    /// Java `countSince`.
    pub fn count_since(&self, since: i64) -> usize {
        self.saves.iter().filter(|s| !s.removed && s.t >= since).count()
    }

    /// Java `wornRecently`: a living character played in the last two weeks wears an amulet that saves you.
    pub fn worn_recently(chars: &crate::characters::Characters, data: &lg_gamedata::GameData, now: i64) -> bool {
        chars.all().any(|c| {
            c.alive
                && c.last_played >= now - RECENT_MS
                && c.equipment.as_ref().is_some_and(|e| e.len() > 3 && data.amulets.of(e[3]).is_some())
        })
    }

    /// Java `relevant`: worn lately, or a save in the last two weeks.
    pub fn relevant(&self, chars: &crate::characters::Characters, data: &lg_gamedata::GameData, now: i64) -> bool {
        AmuletSaves::worn_recently(chars, data, now) || self.count_since(now - RECENT_MS) > 0
    }

    pub fn dump(&self) -> String {
        let q = |s: &str| lg_protocol::dump::to_string(&s.to_string());
        let saves: Vec<String> = self
            .saves
            .iter()
            .map(|s| format!("{}/{}/{}/{}/{}/{}", s.t, s.char_id, q(&s.amulet), q(&s.dungeon), q(&s.message), s.removed))
            .collect();
        let life: Vec<String> = self.life.iter().map(|(t, h, m)| format!("{t}/{h}/{m}")).collect();
        format!(
            "amulet saves=[{}] heal={} pending={}/{} was={} life=[{}]\n",
            saves.join(","),
            self.last_heal_at,
            self.pending_at,
            self.pending.as_ref().map_or("null".into(), |p| q(&format!("{} {}/{}", p.amulet, p.hp, p.max))),
            self.was_invulnerable,
            life.join(",")
        )
    }
}

/// Gson reading `ArrayList<Save>`: missing fields keep their defaults.
fn parse_saves(text: &str) -> Option<Vec<Save>> {
    let Json::Array(a) = json::parse(text).ok()? else { return None };
    let mut out = Vec::new();
    for s in a {
        let text = |k: &str| match s.get(k) {
            None => Some(String::new()),
            Some(Json::Null) => Some(String::new()),
            Some(v) => v.as_string().ok(),
        };
        out.push(Save {
            t: match s.get("t") {
                None => 0,
                Some(v) => v.as_string().ok()?.parse().ok()?,
            },
            char_id: s.get("charId").map_or(Some(0), |v| v.as_int().ok())?,
            amulet: text("amulet")?,
            dungeon: text("dungeon")?,
            message: text("message")?,
            removed: matches!(s.get("removed"), Some(Json::Bool(true))),
        });
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_waiting_save_is_taken_once_within_the_window() {
        let p = Pending { amulet: "A".into(), hp: 228, max: 860 };
        let mut a = AmuletSaves { pending_at: 1000, pending: Some(p.clone()), ..Default::default() };
        assert_eq!(a.take_pending(1000 + HEAL_WINDOW_MS + 1), None); // too late
        assert_eq!(a.take_pending(1200), Some(p));
        assert_eq!(a.take_pending(1300), None); // taken
    }

    #[test]
    fn the_trace_keeps_the_last_moments() {
        let mut a = AmuletSaves::default();
        a.note(1000, "life 400/860, -".into());
        a.note(3500, "hit 95 by Factory Control Core".into());
        a.note(4100, "life 168/860, Invulnerable".into());
        let kept: Vec<i64> = a.trace.iter().map(|t| t.0).collect();
        assert_eq!(kept, vec![3500, 4100]); // the first is more than 3 s before the last
        assert_eq!(AmuletSaves::condition_names(INVULNERABLE | ARMORED | 0x20), "Invulnerable, Armored");
    }

    #[test]
    fn life_before_the_heal_it_showed() {
        // the screenshot of 21 Nov 2026: 228/860 after a +60, so 168/860 (19.5%) when it fired
        let a = AmuletSaves::default();
        assert_eq!(a.lowest_percent(0, 228, 860, 60), 20);
        assert_eq!(a.lowest_percent(0, 228, 860, 120), 13);
    }

    #[test]
    fn rounds_as_java() {
        assert_eq!([java_round(19.5), java_round(20.49), java_round(0.0), java_round(-2.5)], [20, 20, 0, -2]);
    }

    #[test]
    fn heals_and_mentions() {
        assert_eq!(amulet_heal(r#"{"k":"s.plus_symbol","t":{"amount": "120",}}"#), Some(120));
        // cut by a healing debuff (seen in Advanced Kogbold Steamworks, 21 Nov 2026)
        assert_eq!(amulet_heal(r#"{"k":"s.plus_symbol","t":{"amount":"60",}}"#), Some(60));
        assert_eq!(amulet_heal(r#"{"k":"s.plus_symbol","t":{"amount":"150",}}"#), None); // more than it heals
        assert_eq!(amulet_heal(r#"{"k":"s.minus_symbol","t":{"amount":"60",}}"#), None);
        assert!(mentions_amulet("Your Amulet of RESTORATION shines"));
    }

    #[test]
    fn imports_and_drops_old_rule_saves() {
        let s = parse_saves(r#"[{"t":5,"charId":42,"amulet":"A","dungeon":"D","message":"A: Invulnerable (x)","removed":false},{"t":6}]"#)
            .unwrap();
        assert_eq!(s.len(), 2);
        assert_eq!((s[1].t, s[1].char_id, s[1].message.as_str()), (6, 0, ""));
    }
}
