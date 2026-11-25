//! Which missing ST set pieces you could forge now (Inventory > ST Sets).
//!
//! The forge makes a set's piece from its set token (one or two, `forgeProperties.xml`), and most pieces also take
//! one ST item of a kind to dismantle (any set's ST weapon, ability, armor or ring: the item's `ST_WEAPON`... label).
//! The tokens are whatever copies you hold. An ST item only counts to dismantle when you hold it twice or more: the
//! copy with the most enchantments is kept, the second best is the one offered (one per item).
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
}

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
        Stock { copies }
    }

    pub fn from_copies(copies: HashMap<i32, Vec<Held>>) -> Stock {
        Stock { copies }
    }

    fn copies(&self, item: i32) -> Vec<Copy> {
        self.copies.get(&item).map_or_else(Vec::new, |c| c.iter().map(|h| Copy { item, held: h.clone() }).collect())
    }

    /// The spare ST items of a kind ("Armor"...): of each ST item held twice or more, its second best copy.
    pub fn spares(&self, data: &GameData, st_slot: &str) -> Vec<Copy> {
        let label = format!("ST_{}", st_slot.to_uppercase());
        let mut out: Vec<Copy> = Vec::new();
        for (item, held) in &self.copies {
            if held.len() < 2 || !data.items.item(*item).is_some_and(|i| i.is_st() && i.labels.contains(&label)) {
                continue;
            }
            let mut copies = self.copies(*item);
            // the best kept; on a tie the first as listed stays (characters before the vaults)
            copies.sort_by_key(|c| std::cmp::Reverse(c.enchants()));
            out.push(copies.swap_remove(1));
        }
        // the least enchanted first: the one to give up
        out.sort_by(|a, b| a.enchants().cmp(&b.enchants()).then(a.item.cmp(&b.item)));
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
