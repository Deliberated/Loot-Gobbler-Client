//! Forge resources: materials (four kinds), dust (five kinds) and Forgefire, as the game sends them with your
//! character's stats. They belong to your account, one set for the seasonal characters and one for the others, so
//! the last values seen are kept per side (`resources.json`), and what a game mode run's character gains while
//! playing is added up in that run's bank.
//!
//! The stats (decoded from the game, 2026-11-22): 71 materials "kind:amount,..." and 72 their maximum, 127 dust and
//! 147 its maximum, 120 Forgefire. Materials 1 silver, 2 gold, 3 red, 4 purple crystal; dust 4 purple. Dust 1 and 5
//! are green and red, which one is which is not known yet (both were full when seen); 2 and 3 were empty.
//! Changes of these stats (and the unnamed ones near them) are still logged, to learn the rest.

use lg_world::World;
use lg_world::json::{self, Json};
use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;

const MATERIALS: u8 = 71;
const MATERIALS_MAX: u8 = 72;
const FORGEFIRE: u8 = 120;
const DUST: u8 = 127;
const DUST_MAX: u8 = 147;

/// The stats only logged, to learn what they hold.
const LOGGED: [(u8, &str); 11] = [
    (148, "unknown 148"),
    (149, "unknown 149"),
    (150, "unknown 150"),
    (151, "unknown 151"),
    (152, "unknown 152"),
    (153, "unknown 153"),
    (154, "unknown 154"),
    (156, "unknown 156"),
    (157, "unknown 157"),
    (158, "unknown 158"),
    (159, "unknown 159"),
];

/// One side's resources (seasonal or not): per kind the amount and its maximum.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Side {
    pub materials: BTreeMap<i32, (i32, i32)>,
    pub dust: BTreeMap<i32, (i32, i32)>,
    pub forgefire: Option<i32>,
    /// When last seen (ms).
    pub seen_at: i64,
}

/// What a game mode run's character gained while playing: per kind, and Forgefire.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Bank {
    pub materials: BTreeMap<i32, i64>,
    pub dust: BTreeMap<i32, i64>,
    pub forgefire: i64,
}

impl Bank {
    pub fn is_empty(&self) -> bool {
        self.forgefire == 0 && self.materials.values().all(|v| *v == 0) && self.dust.values().all(|v| *v == 0)
    }
}

#[derive(Debug, Default)]
pub struct Resources {
    pub seasonal: Option<Side>,
    pub regular: Option<Side>,
    /// By character: the bank of its game mode run.
    pub banks: BTreeMap<i32, Bank>,
    /// The last values seen and by which character, per side (true: seasonal): gains count only when the same
    /// character played before and after them.
    last: HashMap<bool, (i32, Side)>,
    /// What was logged last, per character and stat.
    logged: HashMap<(i32, u8), String>,
    pub dirty: bool,
}

/// "1:615,2:380,3:0,4:520" -> kind -> amount.
fn pairs(s: &str) -> BTreeMap<i32, i32> {
    let mut out = BTreeMap::new();
    for part in s.split(',') {
        if let Some((k, v)) = part.split_once(':')
            && let (Ok(k), Ok(v)) = (k.trim().parse::<i32>(), v.trim().parse::<i32>())
        {
            out.insert(k, v);
        }
    }
    out
}

fn with_max(amounts: &str, max: Option<&str>) -> BTreeMap<i32, (i32, i32)> {
    let caps = max.map(pairs).unwrap_or_default();
    pairs(amounts).into_iter().map(|(k, v)| (k, (v, caps.get(&k).copied().unwrap_or(0)))).collect()
}

impl Resources {
    /// The side of a character (seasonal or not), its resources as last seen.
    pub fn side(&self, seasonal: bool) -> Option<&Side> {
        if seasonal { self.seasonal.as_ref() } else { self.regular.as_ref() }
    }

    /// Every tick: your character's resource stats; a change is kept for its side, and a gain made while a standing
    /// game mode run's character plays goes to that run's bank. `seasonal`: the character is seasonal.
    pub fn tick(&mut self, w: &World, seasonal: bool, run_standing: bool, now: i64) {
        let Some(me) = w.me() else { return };
        let char_id = me.char_id;
        for (stat, name) in LOGGED {
            let Some(s) = me.stat(stat) else { continue };
            let v = match s.string_stat_value.as_deref() {
                Some(t) => format!("\"{t}\" ({})", s.stat_value),
                None => s.stat_value.to_string(),
            };
            if self.logged.get(&(char_id, stat)) != Some(&v) {
                crate::applog::line("resources", &format!("stat {stat} ({name}) = {v}"));
                self.logged.insert((char_id, stat), v);
            }
        }
        let (Some(materials), Some(dust)) = (me.text(MATERIALS), me.text(DUST)) else { return };
        let side = Side {
            materials: with_max(materials, me.text(MATERIALS_MAX)),
            dust: with_max(dust, me.text(DUST_MAX)),
            forgefire: me.value(FORGEFIRE),
            seen_at: now,
        };
        let changed = self
            .side(seasonal)
            .is_none_or(|old| old.materials != side.materials || old.dust != side.dust || old.forgefire != side.forgefire);
        if !changed {
            return;
        }
        crate::applog::line(
            "resources",
            &format!(
                "{} resources: materials {materials}, dust {dust}, forgefire {}",
                if seasonal { "seasonal" } else { "regular" },
                side.forgefire.map_or("?".into(), |f| f.to_string())
            ),
        );
        if let Some((by, before)) = self.last.get(&seasonal)
            && *by == char_id
            && run_standing
        {
            let bank = self.banks.entry(char_id).or_default();
            for (k, (v, _)) in &side.materials {
                let was = before.materials.get(k).map_or(*v, |x| x.0);
                if v > &was {
                    *bank.materials.entry(*k).or_default() += i64::from(v - was);
                }
            }
            for (k, (v, _)) in &side.dust {
                let was = before.dust.get(k).map_or(*v, |x| x.0);
                if v > &was {
                    *bank.dust.entry(*k).or_default() += i64::from(v - was);
                }
            }
            if let (Some(v), Some(was)) = (side.forgefire, before.forgefire)
                && v > was
            {
                bank.forgefire += i64::from(v - was);
            }
        }
        self.last.insert(seasonal, (char_id, side.clone()));
        if seasonal {
            self.seasonal = Some(side);
        } else {
            self.regular = Some(side);
        }
        self.dirty = true;
    }

    /// What was kept (`resources.json`), if anything.
    pub fn load(dir: &std::path::Path) -> Resources {
        std::fs::read_to_string(dir.join("resources.json")).map_or_else(|_| Resources::default(), |t| Resources::from_text(&t))
    }

    /// The file's (or the server's document's) text read.
    fn from_text(text: &str) -> Resources {
        let mut r = Resources::default();
        let Ok(root) = json::parse(text) else { return r };
        let kinds = |v: Option<&Json>| -> BTreeMap<i32, (i32, i32)> {
            let mut out = BTreeMap::new();
            if let Some(Json::Object(o)) = v {
                for (k, x) in o {
                    if let (Ok(k), Some(Json::Array(a))) = (k.parse::<i32>(), Some(x))
                        && a.len() == 2
                    {
                        let n = |j: &Json| j.as_string().ok().and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
                        out.insert(k, (n(&a[0]), n(&a[1])));
                    }
                }
            }
            out
        };
        let side = |v: Option<&Json>| -> Option<Side> {
            let v = v.filter(|v| matches!(v, Json::Object(_)))?;
            Some(Side {
                materials: kinds(v.get("materials")),
                dust: kinds(v.get("dust")),
                forgefire: v.get("forgefire").and_then(|f| f.as_string().ok()).and_then(|s| s.parse().ok()),
                seen_at: v.get("seenAt").and_then(|f| f.as_string().ok()).and_then(|s| s.parse().ok()).unwrap_or(0),
            })
        };
        r.seasonal = side(root.get("seasonal"));
        r.regular = side(root.get("regular"));
        if let Some(Json::Object(banks)) = root.get("banks") {
            for (id, b) in banks {
                let Ok(id) = id.parse::<i32>() else { continue };
                let gains = |v: Option<&Json>| -> BTreeMap<i32, i64> {
                    let mut out = BTreeMap::new();
                    if let Some(Json::Object(o)) = v {
                        for (k, x) in o {
                            if let (Ok(k), Some(n)) = (k.parse::<i32>(), x.as_string().ok().and_then(|s| s.parse::<i64>().ok())) {
                                out.insert(k, n);
                            }
                        }
                    }
                    out
                };
                let forgefire = b.get("forgefire").and_then(|f| f.as_string().ok()).and_then(|s| s.parse().ok()).unwrap_or(0);
                r.banks.insert(id, Bank { materials: gains(b.get("materials")), dust: gains(b.get("dust")), forgefire });
            }
        }
        r
    }

    /// The server's (or another PC's) document merged in: each side as seen last, each run's bank the larger of
    /// both counts per kind (both PCs may have counted the same gains). True if anything changed.
    pub fn merge_sync(&mut self, remote: &Json) -> bool {
        let Json::Object(_) = remote else { return false };
        let other = Resources::from_text(&remote.write());
        let mut changed = false;
        for (mine, theirs) in [(&mut self.seasonal, other.seasonal), (&mut self.regular, other.regular)] {
            if let Some(t) = theirs
                && mine.as_ref().is_none_or(|m| t.seen_at > m.seen_at)
            {
                *mine = Some(t);
                changed = true;
            }
        }
        for (id, theirs) in other.banks {
            let mine = self.banks.entry(id).or_default();
            let before = mine.clone();
            for (m, t) in [(&mut mine.materials, theirs.materials), (&mut mine.dust, theirs.dust)] {
                for (k, v) in t {
                    let e = m.entry(k).or_insert(0);
                    *e = (*e).max(v);
                }
            }
            mine.forgefire = mine.forgefire.max(theirs.forgefire);
            changed |= *mine != before;
        }
        self.dirty |= changed;
        changed
    }

    /// The file's text.
    pub fn json(&self) -> String {
        let kinds = |m: &BTreeMap<i32, (i32, i32)>| -> String {
            let parts: Vec<String> = m.iter().map(|(k, (v, max))| format!("\"{k}\":[{v},{max}]")).collect();
            format!("{{{}}}", parts.join(","))
        };
        let side = |s: &Option<Side>| -> String {
            s.as_ref().map_or("null".into(), |s| {
                format!(
                    "{{\"materials\":{},\"dust\":{},\"forgefire\":{},\"seenAt\":{}}}",
                    kinds(&s.materials),
                    kinds(&s.dust),
                    s.forgefire.map_or("null".into(), |f| f.to_string()),
                    s.seen_at
                )
            })
        };
        let gains = |m: &BTreeMap<i32, i64>| -> String {
            let parts: Vec<String> = m.iter().map(|(k, v)| format!("\"{k}\":{v}")).collect();
            format!("{{{}}}", parts.join(","))
        };
        let mut banks = String::new();
        for (i, (id, b)) in self.banks.iter().enumerate() {
            let _ = write!(
                banks,
                "{}\"{id}\":{{\"materials\":{},\"dust\":{},\"forgefire\":{}}}",
                if i > 0 { "," } else { "" },
                gains(&b.materials),
                gains(&b.dust),
                b.forgefire
            );
        }
        format!("{{\"seasonal\":{},\"regular\":{},\"banks\":{{{banks}}}}}", side(&self.seasonal), side(&self.regular))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load_text(text: &str) -> Resources {
        let dir = std::env::temp_dir().join(format!("lg-resources-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("dir");
        std::fs::write(dir.join("resources.json"), text).expect("write");
        let r = Resources::load(&dir);
        let _ = std::fs::remove_dir_all(&dir);
        r
    }

    #[test]
    fn reads_the_games_lists_and_keeps_them() {
        let m = with_max("1:615,2:380,3:0,4:520", Some("1:2000,2:2000,3:2000,4:2000"));
        assert_eq!(m[&1], (615, 2000));
        assert_eq!(m[&4], (520, 2000));
        let r = Resources {
            seasonal: Some(Side { materials: m, dust: with_max("4:731", Some("4:2200")), forgefire: Some(2000), seen_at: 5 }),
            banks: [(519, Bank { dust: [(4, 120)].into_iter().collect(), forgefire: 30, ..Default::default() })].into_iter().collect(),
            ..Default::default()
        };
        let back = load_text(&r.json());
        assert_eq!(back.seasonal, r.seasonal);
        assert_eq!(back.regular, None);
        assert_eq!(back.banks, r.banks);
    }
}
