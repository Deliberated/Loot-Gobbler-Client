//! The guild's drop log kept on this PC (`guild_drops.json`): every drop of the guild's members, fetched from the
//! server a page at a time after the last one it has, so the loot caravan can show all of a member's items and only
//! what is new comes over the line.

use crate::gson_out;
use crate::guild::{Caravan, CaravanItem, bag_name};
use crate::server::{long, text};
use lg_world::json::{self, Json};
use std::path::Path;

/// The most drops one answer brings.
pub const PAGE: usize = 120;
/// While more are waiting: the next page after this long (gentle on the server).
pub const PAGE_EVERY_MS: i64 = 10_000;
/// Caught up: asked again after this long.
pub const POLL_MS: i64 = 60_000;
pub const FILE: &str = "guild_drops.json";

/// One drop of the log: its number there (only goes up), whose it is, the item.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LogDrop {
    pub id: i64,
    pub user_id: i64,
    pub item: CaravanItem,
}

#[derive(Debug, Clone, Default)]
pub struct DropLog {
    /// Whose copy this is and of which log: "<server>|<account id>|<guild>|<server instance>".
    pub key: String,
    /// The number of the last drop held (0: none yet).
    pub after: i64,
    /// Whether all there was has come (until then the caravan keeps the server's own lists).
    pub caught_up: bool,
    pub drops: Vec<LogDrop>,
    /// To be written.
    pub dirty: bool,
    pub(crate) loading: bool,
    /// The `after` the call out asked with.
    pub(crate) asked: i64,
    pub(crate) next_at: i64,
}

impl DropLog {
    /// The copy saved in `dir`, or an empty one.
    pub fn import(dir: &Path) -> DropLog {
        let mut log = DropLog::default();
        let Ok(t) = std::fs::read_to_string(dir.join(FILE)) else { return log };
        let Ok(j) = json::parse(&t) else { return log };
        log.key = text(j.get("key"));
        log.after = long(j.get("after"));
        log.caught_up = matches!(j.get("caughtUp"), Some(Json::Bool(true)));
        if let Some(Json::Array(rows)) = j.get("drops") {
            for r in rows {
                let Json::Array(f) = r else { continue };
                let n = |i: usize| long(f.get(i));
                let s = |i: usize| text(f.get(i));
                log.drops.push(LogDrop {
                    id: n(0),
                    user_id: n(1),
                    item: CaravanItem { item_id: n(2) as i32, enchants: n(3) as i32, bag: s(4), time: s(5), r#where: s(6) },
                });
            }
        }
        log
    }

    /// The copy as saved: compact rows of `[id, user, item, enchants, bag, time, where]`.
    pub fn json(&self) -> String {
        let rows = self.drops.iter().map(|d| {
            let i = &d.item;
            format!(
                "[{},{},{},{},{},{},{}]",
                d.id,
                d.user_id,
                i.item_id,
                i.enchants,
                gson_out::string(&i.bag),
                gson_out::string(&i.time),
                gson_out::string(&i.r#where)
            )
        });
        gson_out::Obj::new()
            .str("key", &self.key)
            .int("after", self.after)
            .bool("caughtUp", self.caught_up)
            .raw("drops", Some(gson_out::array(rows)))
            .end()
    }

    /// The call for the next page.
    pub fn path(&self) -> String {
        format!("/guild/drops/log?after={}&limit={PAGE}", self.after)
    }

    /// An answer for `account` ("<server>|<account id>"): its drops added. A log of another guild, server or database
    /// than the copy's throws the copy away and starts over. Whether what the caravan shows changed.
    pub fn take(&mut self, account: &str, j: &Json, now: i64) -> bool {
        self.loading = false;
        let key = format!("{account}|{}|{}", text(j.get("guild")).to_lowercase(), text(j.get("instance")));
        if key != self.key {
            let fresh = self.asked == 0;
            *self = DropLog { key, dirty: true, next_at: now, ..Default::default() };
            if !fresh {
                // this answer went on from a number of another log: from the start again, right away
                return true;
            }
        }
        let mut added = false;
        if let Some(Json::Array(rows)) = j.get("drops") {
            for r in rows {
                let id = long(r.get("id"));
                if id <= self.after {
                    continue;
                }
                let ids = match r.get("enchantIds") {
                    Some(Json::Array(a)) => Some(a.len() as i32),
                    _ => None,
                };
                self.drops.push(LogDrop {
                    id,
                    user_id: long(r.get("userId")),
                    item: CaravanItem {
                        item_id: long(r.get("itemId")) as i32,
                        enchants: ids.unwrap_or_else(|| match r.get("enchants") {
                            Some(Json::Number(_)) => long(r.get("enchants")) as i32,
                            _ => -1,
                        }),
                        bag: bag_name(&text(r.get("bag"))),
                        time: text(r.get("time")),
                        r#where: text(r.get("where")),
                    },
                });
                self.after = id;
                added = true;
            }
        }
        let more = matches!(j.get("more"), Some(Json::Bool(true)));
        let was = self.caught_up;
        self.caught_up = !more;
        self.next_at = now + if more { PAGE_EVERY_MS } else { POLL_MS };
        if added || was != self.caught_up {
            self.dirty = true;
        }
        self.caught_up && (added || !was)
    }

    /// The call failed: again in a while.
    pub fn failed(&mut self, now: i64) {
        self.loading = false;
        self.next_at = now + POLL_MS;
    }

    /// A member's items, newest first.
    pub fn items(&self, user_id: i64) -> Vec<CaravanItem> {
        let mut out: Vec<&LogDrop> = self.drops.iter().filter(|d| d.user_id == user_id).collect();
        out.sort_by(|a, b| b.item.time.cmp(&a.item.time).then(b.id.cmp(&a.id)));
        out.into_iter().map(|d| d.item.clone()).collect()
    }

    /// The caravan's members with all their items from the copy (once it has caught up; until then the server's
    /// lists stay).
    pub fn fill(&self, caravan: &mut Caravan) {
        if !self.caught_up {
            return;
        }
        for m in &mut caravan.members {
            let items = self.items(m.id);
            // the server's list may already have a drop the copy has not yet
            if items.len() >= m.items.len() {
                m.items = items;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn answer(guild: &str, instance: &str, ids: &[i64], more: bool) -> Json {
        let drops: Vec<String> = ids
            .iter()
            .map(|i| format!(r#"{{"id":{i},"userId":7,"itemId":{},"time":"2026-01-01T00:00:{i:02}","where":"Nest","bag":"Boosted White","enchantIds":[1,2]}}"#, 100 + i))
            .collect();
        json::parse(&format!(r#"{{"guild":"{guild}","instance":"{instance}","drops":[{}],"more":{more}}}"#, drops.join(","))).expect("json")
    }

    #[test]
    fn takes_pages_and_starts_over_for_another_log() {
        let mut log = DropLog::default();
        assert!(!log.take("s|1", &answer("Gob", "a", &[1, 2], true), 0));
        assert_eq!((log.after, log.caught_up, log.next_at), (2, false, PAGE_EVERY_MS));
        log.asked = log.after;
        assert!(log.take("s|1", &answer("Gob", "a", &[3], false), 5));
        assert_eq!((log.after, log.caught_up, log.next_at, log.drops.len()), (3, true, 5 + POLL_MS, 3));
        assert_eq!(log.items(7)[0].item_id, 103);
        assert_eq!((log.items(7)[0].enchants, log.items(7)[0].bag.as_str()), (2, "White"));
        // saved and read back
        let dir = std::env::temp_dir().join(format!("lg-droplog-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("dir");
        std::fs::write(dir.join(FILE), log.json()).expect("write");
        let back = DropLog::import(&dir);
        assert_eq!((back.key.clone(), back.after, back.caught_up, back.drops.clone()), (log.key.clone(), 3, true, log.drops.clone()));
        let _ = std::fs::remove_dir_all(&dir);
        // the server's database started over: the copy too
        log.asked = log.after;
        assert!(log.take("s|1", &answer("Gob", "b", &[1], false), 9));
        assert_eq!((log.after, log.drops.len(), log.next_at), (0, 0, 9));
    }
}
