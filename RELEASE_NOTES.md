v0.0.1: The first Rust build of Loot Gobbler

**A new Loot Gobbler:** 
- Rebuilt from the ground up in Rust: one small program, no Java needed, quicker to start and smoother to use
- Keeps its records in the Java app's formats; its first start offers to take over the Java app's records and settings, from the Java app's default folder or one you pick
- Finds the game's files by itself and reads the game data from them
- Windows (exe or portable zip) and Linux (rpm or portable tar.gz)

**Focus:** 
- Pick a game mode run as the focus: a gold line above the tabs, the run's box hanging from it, the open door to leave it again
- While focused, the Inventory shows the run's own finds and only its side of the forge resources, and Quests lists only the missions it can still do alone
- An item's history for a run shows only the copies on its character and its own finds

**Game modes:** 
- Each run keeps its own forge resources (synced with the server too), the items it can use, the missions it is solving alone and what it may take from the gift chest
- The Suggested Loadout of a game mode character uses only what its run obtained

**Inventory:** 
- Forge materials, dust and Forgefire of the seasonal and regular side, right-aligned in the top row with the game's own icons
- A tooltip per item, greyed out characters in the UPE column, and only dead characters that gathered at least five items
- An item opened from another tab is marked by its gold box glowing up three times, the first the fastest
- The white bag slides the column of whose finds open and shut; the pages line up with the collection

**Quests:** 
- The Profile's Quests page as in the Java app: the battle pass cards, the missions with their goals and arrows, the reward choices and the BXP

**Suggested Loadout:** 
- "Suggest enchants" switch, off by default
- Each item's share of the DPS or HP/s you prioritise
- Click an item or an enchantment to lock it; locked ones show in gold
- Item names bigger, the lines below them smaller
- The enchantments a suggestion replaces roll out smoothly while you hover the item, and back in after
- Second chance works with the Protective Matrix too

**Guild:** 
- A UPE comparison chart: points without the rarity factor, each item counted once
- Hovering a lost run shows why it was lost and when, and what was worth a look

**Amulet of Restoration:** 
- Its proc is detected from what the game sends and plays the sound again; Last Stand is recognised too

**Server:** 
- The admins' user list shows which app each member uses (Rust or Java) and its version

**Dungeons:** 
- Recognised when Loot Gobbler starts while you are already inside one
