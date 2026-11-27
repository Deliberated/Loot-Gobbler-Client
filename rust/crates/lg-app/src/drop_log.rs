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
pub const PAGE_EVERY_MS: i64 = 1_000;
/// Caught up: asked again after this long.
pub const POLL_MS: i64 = 60_000;
pub const FILE: &str = "guild_drops.json";
/// The last caravan the server sent, kept so the next start shows it (balloons and all) right away.
pub const CARAVAN_FILE: &str = "guild_caravan.json";

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
                    item: CaravanItem { item_id: n(2) as i32, enchants: n(3) as i32, bag: s(4), time: s(5), r#where: s(6), at: n(7) },
                });
            }
        }
        log
    }

    /// The copy as saved: compact rows of `[id, user, item, enchants, bag, time, where, at]` (`at`: the exact time,
    /// 0 if not known; copies from before it have rows of seven).
    pub fn json(&self) -> String {
        let rows = self.drops.iter().map(|d| {
            let i = &d.item;
            format!(
                "[{},{},{},{},{},{},{},{}]",
                d.id,
                d.user_id,
                i.item_id,
                i.enchants,
                gson_out::string(&i.bag),
                gson_out::string(&i.time),
                gson_out::string(&i.r#where),
                i.at
            )
        });
        gson_out::Obj::new()
            .str("key", &self.key)
            .int("after", self.after)
            .bool("caughtUp", self.caught_up)
            .raw("drops", Some(gson_out::array(rows)))
            .end()
    }

    /// Whose copy it is ("<server>|<account id>"), from its key.
    pub fn account(&self) -> String {
        self.key.splitn(3, '|').take(2).collect::<Vec<_>>().join("|")
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
                        at: long(r.get("at")),
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
        added || was != self.caught_up
    }

    /// Asked again right away (a drop of yours just went up).
    pub fn soon(&mut self) {
        self.next_at = 0;
    }

    /// The call failed: again in a while.
    pub fn failed(&mut self, now: i64) {
        self.loading = false;
        self.next_at = now + POLL_MS;
    }

    /// A member's items found since `start` (epoch ms, 0: all), newest first.
    pub fn items(&self, user_id: i64, start: i64) -> Vec<CaravanItem> {
        let mut out: Vec<&LogDrop> = self.drops.iter().filter(|d| d.user_id == user_id && d.item.since(start)).collect();
        out.sort_by(|a, b| b.item.time.cmp(&a.item.time).then(b.id.cmp(&a.id)));
        out.into_iter().map(|d| d.item.clone()).collect()
    }

    /// The caravan's members (as the server sent them) with their items of the season from the copy: all of them
    /// once it has caught up; while it is still catching up (oldest first), the server's newest ones with the copy's
    /// older ones before them, so what came already shows (also after a restart).
    pub fn fill(&self, caravan: &mut Caravan) {
        let start = caravan.season.start;
        for m in &mut caravan.members {
            let items = self.items(m.id, start);
            if self.caught_up {
                // all of the copy's, and the server's newest it has not fetched yet (just uploaded): both
                let same = |a: &CaravanItem, b: &CaravanItem| a.item_id == b.item_id && a.time == b.time && a.r#where == b.r#where;
                let newer: Vec<CaravanItem> = m.items.iter().filter(|s| !items.iter().any(|i| same(i, s))).cloned().collect();
                m.items = items;
                if !newer.is_empty() {
                    m.items.extend(newer);
                    m.items.sort_by(|a, b| b.time.cmp(&a.time));
                }
                continue;
            }
            // the server sends at most its 120 newest: the copy's drops older than the oldest of them go behind
            if m.items.len() < m.total as usize
                && let Some(oldest) = m.items.last().map(|i| i.time.clone())
            {
                m.items.extend(items.into_iter().filter(|i| i.time < oldest));
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
    fn catching_up_puts_older_drops_behind_the_servers_newest() {
        use crate::guild::CaravanMember;
        let mut log = DropLog::default();
        // the first page: drops 1..3 of player 7 (oldest first), more waiting
        log.take("s|1", &answer("Gob", "a", &[1, 2, 3], true), 0);
        assert!(!log.caught_up);
        // the server's newest of 7's (it found 10): drop 3 and a newer one; the copy's older ones go behind them
        let newest = |id: i32, t: &str| CaravanItem { item_id: id, time: t.into(), ..Default::default() };
        let mut c = Caravan {
            members: vec![CaravanMember {
                id: 7,
                total: 10,
                items: vec![newest(999, "2026-01-02T00:00:00"), newest(103, "2026-01-01T00:00:03")],
                ..Default::default()
            }],
            ..Default::default()
        };
        log.fill(&mut c);
        let ids: Vec<i32> = c.members[0].items.iter().map(|i| i.item_id).collect();
        assert_eq!(ids, vec![999, 103, 102, 101]);
    }

    /// Caught up, the copy's items with the server's newest the copy has not fetched yet (they were just uploaded):
    /// none of either lost.
    #[test]
    fn the_servers_newest_stay_until_the_copy_has_them() {
        use crate::guild::CaravanMember;
        let mut log = DropLog::default();
        log.take("s|1", &answer("Gob", "a", &[1, 2, 3], false), 0);
        assert!(log.caught_up);
        let newest = CaravanItem { item_id: 999, time: "2026-01-02T00:00:00".into(), r#where: "Nest".into(), ..Default::default() };
        let mut c =
            Caravan { members: vec![CaravanMember { id: 7, total: 4, items: vec![newest], ..Default::default() }], ..Default::default() };
        log.fill(&mut c);
        assert_eq!(c.members[0].items.iter().map(|i| i.item_id).collect::<Vec<_>>(), vec![999, 103, 102, 101]);
    }

    #[test]
    fn the_season_starts_the_caravan_over() {
        use crate::guild::{CaravanMember, CaravanSeason};
        let mut log = DropLog::default();
        // drops 1..3 at 00:00:01..03; drop 3 also with its exact time, 2 s after the season began
        let mut j = answer("Gob", "a", &[1, 2, 3], false);
        let start = 1_798_761_600_000; // 2027-01-01T00:00:00Z
        if let Json::Object(o) = &mut j
            && let Some((_, Json::Array(drops))) = o.iter_mut().find(|(k, _)| k == "drops")
            && let Some(Json::Object(d)) = drops.last_mut()
        {
            d.push(("at".into(), Json::Number((start + 2_000).to_string())));
        }
        log.take("s|1", &j, 0);
        let member = || CaravanMember { id: 7, total: 3, ..Default::default() };
        // the season began at 00:00:02 (by the times): drops 2 and 3
        let mut c = Caravan {
            members: vec![member()],
            season: CaravanSeason { start: 1_767_225_602_000, ..Default::default() },
            ..Default::default()
        };
        log.fill(&mut c);
        assert_eq!(c.members[0].items.iter().map(|i| i.item_id).collect::<Vec<_>>(), vec![103, 102]);
        // a season starting in 2027: only drop 3, by its exact time
        let mut c = Caravan { members: vec![member()], season: CaravanSeason { start, ..Default::default() }, ..Default::default() };
        log.fill(&mut c);
        assert_eq!(c.members[0].items.iter().map(|i| i.item_id).collect::<Vec<_>>(), vec![103]);
        // the exact time is kept in the copy
        let dir = std::env::temp_dir().join(format!("lg-droplog-season-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("dir");
        std::fs::write(dir.join(FILE), log.json()).expect("write");
        assert_eq!(DropLog::import(&dir).drops[2].item.at, start + 2_000);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn takes_pages_and_starts_over_for_another_log() {
        let mut log = DropLog::default();
        assert!(log.take("s|1", &answer("Gob", "a", &[1, 2], true), 0));
        assert_eq!((log.after, log.caught_up, log.next_at), (2, false, PAGE_EVERY_MS));
        log.asked = log.after;
        assert!(log.take("s|1", &answer("Gob", "a", &[3], false), 5));
        assert_eq!((log.after, log.caught_up, log.next_at, log.drops.len()), (3, true, 5 + POLL_MS, 3));
        assert_eq!(log.items(7, 0)[0].item_id, 103);
        assert_eq!((log.items(7, 0)[0].enchants, log.items(7, 0)[0].bag.as_str()), (2, "White"));
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
