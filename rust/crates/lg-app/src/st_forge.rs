//! Which missing ST set pieces you could forge now (Inventory > ST Sets).
//!
//! The forge makes a set's piece from its set token (one or two, `forgeProperties.xml`), and most pieces also take
//! one ST item of a kind to dismantle (any set's ST weapon, ability, armor or ring: the item's `ST_WEAPON`... label).
//! The tokens are whatever copies you hold. An ST item only counts to dismantle beyond the copies kept: as a
//! collector the one with the most enchantments, with a backup also the second best (unless that backup is let go);
//! the best of the rest on each side is the one offered.
//! Tokens and the item dismantled have to be on the same side, seasonal or not; a place whose side is not known
//! counts for both.

use crate::loadout::Held;
use lg_gamedata::GameData;
use lg_gamedata::st_sets::StSet;
use std::collections::HashMap;

/// A copy of an item somewhere, with the item.
#[derive(Debug, Clone)]
pub struct Copy {
    pub item: i32,
    pub held: Held,
}

impl Copy {
    /// Its enchantments (-1: not known).
    pub fn enchants(&self) -> i32 {
        self.held.enchant_ids.as_ref().map_or(-1, |e| e.len() as i32)
    }
}

/// What forging one missing piece takes, and what of it you hold.
#[derive(Debug, Clone)]
pub struct Piece {
    pub piece: i32,
    pub token: i32,
    /// Tokens the recipe takes.
    pub amount: i32,
    /// Every copy of the token you hold.
    pub tokens: Vec<Copy>,
    /// The kind of ST item it takes to dismantle ("Weapon", "Ability", "Armor", "Ring"), if any.
    pub st_slot: Option<String>,
    /// The spare ST items of that kind (see the module), least enchanted first.
    pub spares: Vec<Copy>,
    /// Tokens and a spare (if one is needed) on one side: it can be forged now.
    pub craftable: bool,
}

/// What the planner works from: every copy of the set tokens and of every ST item, by item.
pub struct Stock {
    copies: HashMap<i32, Vec<Held>>,
    /// Copies of each ST item kept (not offered to dismantle): 1 as a collector, 2 with a backup.
    keep: usize,
    /// Items whose backup was let go: only their best kept.
    freed: std::collections::HashSet<i32>,
    /// The rarities (by enchantments, 0 to 4) whose copies may be dismantled for a craft.
    rarities: [bool; 5],
}

/// The setting listing the rarities (enchantments 0 to 4, comma separated) considered for forging and crafting; not
/// set: all five.
pub const RARITIES_KEY: &str = "stForgeRarities";

/// The rarities considered, as the setting says.
pub fn rarities(value: Option<&str>) -> [bool; 5] {
    match value {
        None => [true; 5],
        Some(v) => {
            let mut out = [false; 5];
            for n in v.split(',').filter_map(|x| x.trim().parse::<usize>().ok()).filter(|n| *n < 5) {
                out[n] = true;
            }
            out
        }
    }
}

/// The setting's value for these rarities.
pub fn rarities_value(r: [bool; 5]) -> String {
    (0..5).filter(|n| r[*n]).map(|n| n.to_string()).collect::<Vec<_>>().join(",")
}

/// How the forge suggestions treat your ST items (the ST Sets page's toggle).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    /// No suggestions.
    Off,
    /// The best copy of each kept, the rest can be dismantled.
    #[default]
    Collector,
    /// The best copy and a backup (the second best) kept.
    Backup,
}

impl Mode {
    pub const KEY: &str = "stForgeMode";
    /// The setting's value.
    pub fn of(value: Option<&str>) -> Mode {
        match value {
            Some("off") => Mode::Off,
            Some("backup") => Mode::Backup,
            _ => Mode::Collector,
        }
    }
    pub fn value(self) -> &'static str {
        match self {
            Mode::Off => "off",
            Mode::Collector => "collector",
            Mode::Backup => "backup",
        }
    }
    pub fn next(self) -> Mode {
        match self {
            Mode::Off => Mode::Collector,
            Mode::Collector => Mode::Backup,
            Mode::Backup => Mode::Off,
        }
    }
}

/// The setting listing the items whose backup was let go (ids, comma separated).
pub const FREED_KEY: &str = "stBackupFreed";

impl Stock {
    /// Every copy of the set tokens and the ST items on your living characters and in your vaults; with `only_char`
    /// (a game mode run's character) only that character's.
    pub fn of(app: &crate::App, only_char: i32) -> Stock {
        let data = app.world.data();
        let tokens: std::collections::HashSet<i32> = data.set_tokens.values().map(|t| t.token).collect();
        let living: HashMap<i32, bool> = app.characters.living().iter().map(|l| (l.char_id, l.seasonal)).collect();
        let mut copies = app.holdings.copies_of(&living, |id| tokens.contains(&id) || data.items.item(id).is_some_and(|i| i.is_st()));
        if only_char != 0 {
            for c in copies.values_mut() {
                c.retain(|h| h.char_id == only_char);
            }
            copies.retain(|_, c| !c.is_empty());
        }
        Stock { copies, keep: 1, freed: Default::default(), rarities: [true; 5] }
    }

    /// The copies kept of each ST item as the mode says (a backup too), but for the items whose backup was let go.
    pub fn keeping(mut self, mode: Mode, freed: std::collections::HashSet<i32>) -> Stock {
        self.keep = if mode == Mode::Backup { 2 } else { 1 };
        self.freed = freed;
        self
    }

    /// The stock less copies set aside (crafts planned): for each, the first copy of its item in the same place
    /// with the same enchantments.
    pub fn without(&self, taken: &[(i32, Held)]) -> Stock {
        let mut copies = self.copies.clone();
        for (item, h) in taken {
            if let Some(c) = copies.get_mut(item)
                && let Some(i) = c
                    .iter()
                    .position(|x| x.char_id == h.char_id && x.vault == h.vault && x.slot == h.slot && x.enchant_ids == h.enchant_ids)
            {
                c.remove(i);
            }
        }
        copies.retain(|_, c| !c.is_empty());
        Stock { copies, keep: self.keep, freed: self.freed.clone(), rarities: self.rarities }
    }

    /// Only copies of these rarities offered to dismantle.
    pub fn with_rarities(mut self, rarities: [bool; 5]) -> Stock {
        self.rarities = rarities;
        self
    }

    /// Whether a spare ST item of a kind (on a side: Some(true) seasonal, Some(false) regular) would be there but for
    /// the rarities left out.
    pub fn spare_left_out(&self, data: &GameData, st_slot: &str, side: Option<bool>) -> bool {
        let on = |c: &Copy| c.held.seasonal.is_none() || c.held.seasonal == side;
        let all = Stock { copies: self.copies.clone(), keep: self.keep, freed: self.freed.clone(), rarities: [true; 5] };
        !self.spares(data, st_slot).iter().any(on) && all.spares(data, st_slot).iter().any(on)
    }

    /// How many copies of `item` are kept.
    pub fn kept(&self, item: i32) -> usize {
        if self.freed.contains(&item) { 1 } else { self.keep }
    }

    /// Every copy of an item, the most enchanted first (on a tie as listed: characters before the vaults).
    pub fn ranked(&self, item: i32) -> Vec<Copy> {
        let mut c = self.copies(item);
        c.sort_by_key(|c| std::cmp::Reverse(c.enchants()));
        c
    }

    /// What a character (not a game mode run) can reach: what it carries, and the vault of its side (seasonal or
    /// regular), from which it can take things.
    pub fn reachable(app: &crate::App, char_id: i32, seasonal: bool) -> Stock {
        let mut s = Stock::of(app, 0);
        for c in s.copies.values_mut() {
            c.retain(|h| h.char_id == char_id || (h.char_id == 0 && h.seasonal == Some(seasonal)));
        }
        s.copies.retain(|_, c| !c.is_empty());
        s
    }

    pub fn from_copies(copies: HashMap<i32, Vec<Held>>) -> Stock {
        Stock { copies, keep: 1, freed: Default::default(), rarities: [true; 5] }
    }

    fn copies(&self, item: i32) -> Vec<Copy> {
        self.copies.get(&item).map_or_else(Vec::new, |c| c.iter().map(|h| Copy { item, held: h.clone() }).collect())
    }

    /// The spare ST items of a kind ("Armor"...): of each ST item held beyond the copies kept, the best of the rest on
    /// each side (seasonal, regular; one when they are on one side); the rarest first.
    pub fn spares(&self, data: &GameData, st_slot: &str) -> Vec<Copy> {
        let label = format!("ST_{}", st_slot.to_uppercase());
        let mut out: Vec<Copy> = Vec::new();
        for (item, held) in &self.copies {
            let keep = self.kept(*item);
            if held.len() <= keep || !data.items.item(*item).is_some_and(|i| i.is_st() && i.labels.contains(&label)) {
                continue;
            }
            // the best kept; on a tie the first as listed stays (characters before the vaults)
            let rest: Vec<Copy> =
                self.ranked(*item).into_iter().skip(keep).filter(|c| self.rarities[c.enchants().clamp(0, 4) as usize]).collect();
            let mut taken = 0;
            for side in [Some(true), Some(false)] {
                if let Some((i, c)) =
                    rest.iter().enumerate().find(|(i, c)| taken & (1 << i) == 0 && (c.held.seasonal.is_none() || c.held.seasonal == side))
                {
                    taken |= 1 << i;
                    out.push(c.clone());
                }
            }
        }
        // the rarest first: what the craft is best made with (of the rarities considered)
        out.sort_by(|a, b| b.enchants().cmp(&a.enchants()).then(a.item.cmp(&b.item)));
        out
    }

    /// For a set: each missing piece the forge makes from a token, what it takes and what of it you hold.
    pub fn plan(&self, data: &GameData, set: &StSet, has: &impl Fn(i32) -> bool) -> Vec<Piece> {
        let mut out = Vec::new();
        for &piece in &set.pieces {
            if has(piece) {
                continue;
            }
            let Some(cost) = data.set_tokens.get(&piece) else { continue };
            let tokens = self.copies(cost.token);
            let spares = cost.st_slot.as_deref().map_or_else(Vec::new, |s| self.spares(data, s));
            let craftable = [Some(false), Some(true)].iter().any(|side| {
                let on = |h: &Held| h.seasonal.is_none() || h.seasonal == *side;
                tokens.iter().filter(|c| on(&c.held)).count() as i32 >= cost.amount
                    && (cost.st_slot.is_none() || spares.iter().any(|c| on(&c.held)))
            });
            out.push(Piece { piece, token: cost.token, amount: cost.amount, tokens, st_slot: cost.st_slot.clone(), spares, craftable });
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn held(char_id: i32, enchants: usize, seasonal: bool) -> Held {
        Held {
            char_id,
            vault: (char_id == 0).then(|| "vault".to_string()),
            slot: "Vault chest".into(),
            enchant_ids: Some(vec![1; enchants]),
            seasonal: Some(seasonal),
        }
    }

    fn item(id: i32, name: &str, slot_type: i32, labels: &[&str]) -> lg_gamedata::items::Item {
        lg_gamedata::items::Item {
            name: name.into(),
            slot_type,
            labels: labels.iter().map(|l| l.to_string()).collect(),
            ..lg_gamedata::items::Item::new(id)
        }
    }

    fn data() -> GameData {
        let mut d = GameData::default();
        d.items.insert(item(0x100, "Spellbound Set Token", 10, &[]));
        d.items.insert(item(0x201, "Piece A", 2, &["ST", "ST_WEAPON"]));
        d.items.insert(item(0x202, "Piece B", 6, &["ST", "ST_ARMOR"]));
        d.items.insert(item(0x300, "Other Armor", 14, &["ST", "ST_ARMOR"]));
        let forge = r#"<ForgeProperties type="0x201" id="w"><RequiredItem quantity="2">0x100</RequiredItem><RequireDismantleWithLabel amount="1">ST_WEAPON</RequireDismantleWithLabel></ForgeProperties>
<ForgeProperties type="0x202" id="a"><RequiredItem quantity="1">0x100</RequiredItem><RequireDismantleWithLabel amount="1">ST_ARMOR</RequireDismantleWithLabel></ForgeProperties>"#;
        d.set_tokens = lg_gamedata::st_sets::set_tokens(forge, &d.items);
        d
    }

    fn set() -> StSet {
        StSet {
            name: "s".into(),
            title: "S".into(),
            class_name: "Rogue".into(),
            skin: 0,
            pieces: [0x201, 0x202, 0x203, 0x204],
            bonus: String::new(),
        }
    }

    #[test]
    fn the_second_best_copy_is_the_spare() {
        let d = data();
        let stock = Stock::from_copies(HashMap::from([(0x300, vec![held(1, 1, false), held(0, 3, false), held(2, 2, false)])]));
        let spares = stock.spares(&d, "Armor");
        assert_eq!(spares.len(), 1, "one spare per item");
        assert_eq!(spares[0].enchants(), 2, "the best (3) is kept, the second best (2) offered");
        assert!(stock.spares(&d, "Weapon").is_empty());
    }

    #[test]
    fn a_backup_is_kept_unless_let_go() {
        let d = data();
        let copies = HashMap::from([(0x300, vec![held(1, 1, false), held(0, 3, false), held(2, 2, false)])]);
        let backup = Stock::from_copies(copies.clone()).keeping(Mode::Backup, Default::default());
        let spares = backup.spares(&d, "Armor");
        assert_eq!(spares.len(), 1);
        assert_eq!(spares[0].enchants(), 1, "the best (3) and the backup (2) kept, the third offered");
        let freed = Stock::from_copies(copies).keeping(Mode::Backup, [0x300].into());
        assert_eq!(freed.spares(&d, "Armor")[0].enchants(), 2, "the backup let go: offered again");
    }

    #[test]
    fn a_spare_on_each_side() {
        let d = data();
        let stock = Stock::from_copies(HashMap::from([(0x300, vec![held(0, 4, false), held(1, 3, true), held(2, 1, false)])]));
        let spares = stock.spares(&d, "Armor");
        assert_eq!(spares.len(), 2, "one seasonal, one regular");
    }

    #[test]
    fn copies_set_aside_are_left_out() {
        let d = data();
        let stock = Stock::from_copies(HashMap::from([(0x100, vec![held(0, 0, false), held(1, 0, false)])]));
        let less = stock.without(&[(0x100, held(0, 0, false))]);
        let plan = less.plan(&d, &set(), &|_| false);
        assert_eq!(plan[0].tokens.len(), 1, "one of the two tokens set aside");
    }

    #[test]
    fn only_the_rarities_chosen_are_offered() {
        let d = data();
        let copies = HashMap::from([(0x300, vec![held(1, 4, false), held(0, 2, false), held(2, 0, false)])]);
        let all = Stock::from_copies(copies.clone());
        assert_eq!(all.spares(&d, "Armor")[0].enchants(), 2);
        let no_rare = Stock::from_copies(copies).with_rarities([true, true, false, true, true]);
        assert_eq!(no_rare.spares(&d, "Armor")[0].enchants(), 0, "the copy with 2 enchantments left out");
        assert_eq!(rarities(Some("0,3")), [true, false, false, true, false]);
        assert_eq!(rarities_value([true, false, false, true, false]), "0,3");
    }

    #[test]
    fn the_rarest_spare_comes_first() {
        let d = data();
        let mut d = d;
        d.items.insert(item(0x301, "Second Armor", 14, &["ST", "ST_ARMOR"]));
        let stock = Stock::from_copies(HashMap::from([
            (0x300, vec![held(1, 4, false), held(0, 1, false)]),
            (0x301, vec![held(2, 4, false), held(0, 3, false)]),
        ]));
        assert_eq!(stock.spares(&d, "Armor")[0].enchants(), 3, "the rare spare before the uncommon one");
    }

    #[test]
    fn a_single_copy_is_no_spare() {
        let d = data();
        let stock = Stock::from_copies(HashMap::from([(0x300, vec![held(1, 4, false)]), (0x100, vec![held(0, 0, false)])]));
        let plan = stock.plan(&d, &set(), &|_| false);
        let armor = plan.iter().find(|p| p.piece == 0x202).unwrap();
        assert_eq!(armor.tokens.len(), 1);
        assert!(armor.spares.is_empty());
        assert!(!armor.craftable);
    }

    #[test]
    fn tokens_and_a_spare_make_it_craftable() {
        let d = data();
        let stock =
            Stock::from_copies(HashMap::from([(0x300, vec![held(1, 4, false), held(0, 0, false)]), (0x100, vec![held(0, 0, false)])]));
        let plan = stock.plan(&d, &set(), &|id| id == 0x203 || id == 0x204);
        let armor = plan.iter().find(|p| p.piece == 0x202).unwrap();
        assert!(armor.craftable, "1 token and a spare ST armor");
        let weapon = plan.iter().find(|p| p.piece == 0x201).unwrap();
        assert!(!weapon.craftable, "2 tokens needed, and no spare ST weapon");
        assert_eq!(plan.len(), 2, "pieces held are left out, as are pieces without a recipe");
    }

    #[test]
    fn tokens_and_spare_on_one_side() {
        let d = data();
        // the token in the seasonal vault, the spare on the regular side
        let stock =
            Stock::from_copies(HashMap::from([(0x300, vec![held(1, 4, false), held(0, 0, false)]), (0x100, vec![held(0, 0, true)])]));
        let plan = stock.plan(&d, &set(), &|_| false);
        assert!(!plan.iter().find(|p| p.piece == 0x202).unwrap().craftable);
    }
}
