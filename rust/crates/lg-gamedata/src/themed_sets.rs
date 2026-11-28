//! Themed sets: groups of four untiered items made around one look and one class (as the RealmEye wiki lists them,
//! "Themed Sets"). The game itself does not group them (unlike the ST sets, `equipmentsets.xml`), so they are listed
//! here by the items' names, resolved against the game's data when it loads.

use crate::items::Items;
use crate::st_sets::StSet;

/// (name, class, what it is themed on, [weapon, ability, armor, ring] by their names, the four's stats together).
pub const THEMED: [(&str, &str, &str, [&str; 4], &str); 17] = [
    (
        "Coral Huntress",
        "Huntress",
        "Ocean Trench",
        ["Coral Bow", "Coral Venom Trap", "Coral Silk Armor", "Coral Ring"],
        "+75 MP, +14 DEF, +9 SPD, +17 DEX, +13 WIS",
    ),
    (
        "Crystal Necromancer",
        "Necromancer",
        "Forbidden Jungle",
        ["Staff of the Crystal Serpent", "Cracked Crystal Skull", "Robe of the Tlatoani", "Crystal Bone Ring"],
        "+50 HP, +90 MP, +4 DEX, +11 WIS",
    ),
    (
        "Purified Priest",
        "Priest",
        "Unknown",
        ["St. Abraham's Wand", "Tome of Purification", "Chasuble of Holy Light", "Ring of Divine Faith"],
        "+90 HP, +90 MP, +12 DEF, +27 VIT, +27 WIS",
    ),
    (
        "Ghostly Trickster",
        "Trickster",
        "Davy Jones’ Locker",
        ["Spirit Dagger", "Ghostly Prism", "Spectral Cloth Armor", "Captain's Ring"],
        "+50 HP, +9 ATT, +14 DEF, +9 DEX, +5 VIT, +5 WIS",
    ),
    (
        "Mad Scientist Sorcerer",
        "Sorcerer",
        "Dr. Terrible",
        ["Conducting Wand", "Scepter of Fulmination", "Robe of the Mad Scientist", "Experimental Ring"],
        "+60 HP, +60 MP, +2 ATT, +17 DEF, +4 DEX, +10 VIT, +11 WIS",
    ),
    (
        "Marble Paladin",
        "Paladin",
        "Marble Colossus",
        ["Sword of the Colossus", "Marble Seal", "Breastplate of New Life", "Magical Lodestone"],
        "+190 HP, +6 ATT, +25 DEF, +6 SPD, +6 DEX",
    ),
    (
        "Cultist Necromancer",
        "Necromancer",
        "Cultist Hideout",
        ["Staff of Unholy Sacrifice", "Skull of Corrupted Souls", "Ritual Robe", "Bloodshed Ring"],
        "+176 HP, +50 MP, +5 ATT, +18 DEF, +6 VIT, +28 WIS",
    ),
    (
        "Void Archer",
        "Archer",
        "The Void",
        ["Bow of the Void", "Quiver of Shadows", "Armor of Nil", "Sourcestone"],
        "+110 HP, +110 MP, +36 DEF, +1 SPD",
    ),
    (
        "Spirit Mystic",
        "Mystic",
        "Magic Woods",
        ["Spirit Staff", "Enchantment Orb", "Woodland Robe", "Fairy Ring"],
        "+60 HP, +12 DEF, +10 SPD, +10 DEX, +5 VIT, +5 WIS",
    ),
    (
        "Esben Necromancer",
        "Necromancer",
        "Esben the Neurotic",
        ["Staff of Esben", "Skullish Remains of Esben", "Esben's Shaman Attire", "Esben's Wedding Ring"],
        "+180 HP, +80 MP, +20 ATT, +13 DEF, -3 SPD",
    ),
    (
        "Chancellor Wizard",
        "Wizard",
        "Chancellor Dammah",
        ["Superior", "Genesis Spell", "Diplomatic Robe", "Chancellor's Cranium"],
        "+170 HP, +10 ATT, +17 DEF, +3 DEX",
    ),
    (
        "Treasurer Trickster",
        "Trickster",
        "Treasurer Gemsbok",
        ["Avarice", "Gambler's Fate", "Turncoat Cape", "Collector's Monocle"],
        "+140 HP, +4 ATT, +18 DEF, +12 DEX, +10 SPD",
    ),
    (
        "Archbishop Priest",
        "Priest",
        "Archbishop Leucoryx",
        ["Lumiaire", "Chaotic Scripture", "Vesture of Duality", "Divine Coronation"],
        "+170 HP, +95 MP, +5 ATT, +17 DEF, +5 SPD, +8 DEX, +6 VIT",
    ),
    (
        "Chief Ninja",
        "Ninja",
        "Chief Beisa",
        ["Enforcer", "Ballistic Star", "Centaur's Shielding", "Battalion Banner"],
        "+90 HP, +5 ATT, +13 DEF, +4 SPD, +11 DEX",
    ),
    (
        "Exalted Knight",
        "Knight",
        "Oryx the Mad God 3",
        ["Divinity", "Oryx's Escutcheon", "Gladiator Guard", "Exalted God's Horn"],
        "+140 HP, +5 ATT, +35 DEF, +14 SPD, +5 DEX",
    ),
    (
        "Esben Wizard",
        "Wizard",
        "Esben the Neurotic",
        ["Staff of Esben", "Esben's Vows", "Esben's Shaman Attire", "Esben's Wedding Ring"],
        "+140 HP, +80 MP, +16 ATT, +10% ATT equal to DEF, +18 DEF",
    ),
    (
        "Summer Solstice (Wizard)",
        "Wizard",
        "The Summer Solstice",
        ["Staff of the Rising Sun", "Thousand Suns Spell", "Robe of the Summer Solstice", "Ring of the Burning Sun"],
        "+60 HP, +95 MP, +4 ATT, +23 DEF, +12 WIS",
    ),
];

/// Each item's shiny copy (the item showing the same name, labelled shiny), by the plain item.
pub fn shiny_twins(items: &Items) -> std::collections::HashMap<i32, i32> {
    let mut plain: std::collections::HashMap<&str, i32> = std::collections::HashMap::new();
    for i in items.items.values().filter(|i| !i.is_shiny() && !i.name.is_empty()) {
        // the lowest id for a name shown by several
        let e = plain.entry(i.name.as_str()).or_insert(i.id);
        *e = (*e).min(i.id);
    }
    items.items.values().filter(|i| i.is_shiny()).filter_map(|s| plain.get(s.name.as_str()).map(|p| (*p, s.id))).collect()
}

/// The themed sets whose four items the data has, in the list's order; `class_id` gives a class's object type by its
/// name (the card's figure: the class as it starts out).
pub fn load(items: &Items, class_id: impl Fn(&str) -> Option<i32>) -> Vec<StSet> {
    // a name: the item showing it (an untiered item's shiny copy shows the same name: the plain one first)
    let by_name = |n: &str| {
        let mut found: Vec<&crate::items::Item> = items.items.values().filter(|i| i.name == n).collect();
        found.sort_by_key(|i| (i.is_shiny(), i.id));
        found.first().map(|i| i.id)
    };
    THEMED
        .iter()
        .filter_map(|(name, class, on, pieces, stats)| {
            let ids = [by_name(pieces[0])?, by_name(pieces[1])?, by_name(pieces[2])?, by_name(pieces[3])?];
            Some(StSet {
                name: format!("{name} (themed)"),
                title: name.to_string(),
                class_name: class.to_string(),
                skin: class_id(class).unwrap_or(0),
                pieces: ids,
                bonus: if on.is_empty() || *on == "Unknown" {
                    format!("Together: {stats}")
                } else {
                    format!("Themed on {on}.   Together: {stats}")
                },
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::items::Item;

    #[test]
    fn sets_from_names_the_plain_item_before_its_shiny_copy() {
        let mut items = Items::default();
        let named = |id: i32, name: &str, labels: &[&str]| {
            let mut i = Item::new(id);
            i.name = name.into();
            i.labels = labels.iter().map(|l| l.to_string()).collect();
            i
        };
        for (k, n) in ["Enforcer", "Ballistic Star", "Centaur's Shielding", "Battalion Banner"].iter().enumerate() {
            items.insert(named(100 + k as i32, n, &["UT"]));
        }
        // a shiny copy showing the same name, with a lower id
        items.insert(named(5, "Enforcer", &["UT", "SHINY"]));
        let sets = load(&items, |c| (c == "Ninja").then_some(806));
        let ninja = sets.iter().find(|s| s.title == "Chief Ninja").expect("the set");
        assert_eq!((ninja.pieces, ninja.skin, ninja.class_name.as_str()), ([100, 101, 102, 103], 806, "Ninja"));
        // the others: their items are not in this data
        assert_eq!(sets.len(), 1);
    }
}
