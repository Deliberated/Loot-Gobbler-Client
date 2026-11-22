//! Forge resources (dust of each kind, seasonal and not, and Forgefire): for now only written to the app log as the
//! game sends them, to learn how the game encodes them before the Inventory shows them. The stats are your
//! character's with these numbers (names from the game's stat list): 71 material, 72 material cap, 120 forge fire,
//! 127 dust, 147 dust amount, and the unnamed 148-154 and 156-159.

use lg_world::World;
use std::collections::HashMap;

const WATCHED: [(u8, &str); 16] = [
    (71, "material"),
    (72, "material cap"),
    (120, "forge fire"),
    (127, "dust"),
    (147, "dust amount"),
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

#[derive(Debug, Default)]
pub struct Resources {
    /// Per stat its value as last logged (number or text), per character.
    seen: HashMap<(i32, u8), String>,
}

impl Resources {
    /// Every tick: a watched stat of yours that changed, logged with its raw value.
    pub fn tick(&mut self, w: &World) {
        let Some(me) = w.me() else { return };
        for (stat, name) in WATCHED {
            let Some(s) = me.stat(stat) else { continue };
            let v = match s.string_stat_value.as_deref() {
                Some(t) => format!("\"{t}\" ({})", s.stat_value),
                None => s.stat_value.to_string(),
            };
            if self.seen.get(&(me.char_id, stat)) != Some(&v) {
                crate::applog::line("resources", &format!("stat {stat} ({name}) = {v}"));
                self.seen.insert((me.char_id, stat), v);
            }
        }
    }
}
