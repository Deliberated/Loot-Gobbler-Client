v0.0.1: The first Rust build of Loot Gobbler

**A new Loot Gobbler:** 
- Rebuilt from the ground up in Rust: one small program, no Java needed, quicker to start and light on your CPU
- Only listens to what the game receives on your PC; it never sends, blocks or changes anything
- Keeps its records in the Java app's formats; its first start offers to take over the Java app's records and settings, from its default folder or one you pick
- Finds the game's files by itself and reads the game data from them
- Windows: an installer (LootGobbler.exe, no admin rights needed, installs into %APPDATA%\LootGobbler-Rust beside the Java app) or the portable zip; Linux: rpm or portable tar.gz; tells you when a new version is out
- Linux: when the sniffer cannot capture yet, one click asks for your password, installs what is missing (libpcap, setcap), gives the permission and starts Loot Gobbler again
- Its own page with every feature and pictures: https://deliberated.github.io/Loot-Gobbler-Client/

**Profile:** 
- An overview of you: your most played character, recent drops and runs, and your exaltations of the last days
- The fame graph of the last 24 hours: every dungeon's portal above the line, the guildmates who fought with you above it, hours without fame cut out; click a portal to open that dungeon under Battles
- Quests: the battle pass cards, the season missions with their goals, progress and reward choices, and the BXP

**Inventory:** 
- Your UT/ST collection, filled in by itself as you find and store items: what you own lit up, with how many copies
- Pick an item to see every copy you hold, where each one sits (backpack, vault, gift chest, which character, seasonal or not) and its enchants, the drops of it you found and the copies used up
- Search by name or enchant, filter shiny, normal or reusable, for your whole account or one game mode run
- Forge materials, dust and Forgefire of the seasonal and regular side, with the game's own icons
- ST Sets: every set with the pieces you own, the ones almost complete, and your progress per class
- Log: every bag that dropped for you, filtered by bag colour

**Characters:** 
- Every character with its gear drawn as in the game, its maxed stats, tags and the fame it gained; fallen ones in the Graveyard with their kills and full inventory
- Copy as image: a character's badge with its gear, enchants, backpack, quickslot items and the plate of its game mode
- Suggested Loadout: the best gear from what you own for the DPS, HP/s, HP or stats you prioritise, each item's share shown; scope local, vault or account; enchant suggestions; lock an item or an enchantment to keep it; second chance and the Protective Matrix counted
- Classes: your exaltations per class and stat, the recent progress (last 3 days) in blue with its total
- Pets: every pet with its abilities and saddlebags

**Game modes:** 
- UPE, Pure UPE, STPE, Shiny UPE, Divine PE and PPE, each with a guild and a no-pet version
- The rules checked as you play (vault items, trades, shop, rewards); a lost run says when and why
- Each run keeps its own items, forge resources, missions it is solving alone and what it may take from the gift chest, synced with the server too
- Focus: put a run in focus and the whole app follows it, with a gold line above the tabs and the run hanging from it; the Inventory shows only its finds and forge resources, Quests only the missions it can still do alone, an item's history only its copies, the Suggested Loadout only what it obtained

**Battles:** 
- Dungeons: every dungeon you enter, grouped by day with the day's loot, exaltations and fame; each with its boss ranks and DPS, the damage you dealt and took over time, big hits, quickslots, your death and when your amulet saved you
- Your guildmates in the same run side by side: critical hits, biggest slap, shots dodged, quickslots, and a podium
- DPS: each boss's damage table with your rank and share, and the history of all your runs
- The box in the menu bar while you are in a dungeon: its portal, the time inside, your rank on the boss and the difficulty; a click opens it
- Recognised when Loot Gobbler starts while you are already inside a dungeon

**Overlays:** 
- Small windows above the game, each switched on by itself, with its own opacity, dragged where you want them
- Dungeon (portal, rank, amulet and matrix recharging, Moonlight Village's lantern and phases), Mana, DPS (your place with the players around you) and Buffs (item cooldowns)

**Amulet of Restoration:** 
- Its proc is detected from what the game sends, Last Stand too; a sound when it saves you and another when it is charged again

**Guild (with a Loot Gobbler server):** 
- The loot caravan: every guildie's wagon piled with their finds, the highest score leading, with each member's points, bags and best finds
- The weekly quest map the guild clears together, leaderboards and recent drops
- The UPE comparison: points without the rarity factor, each item counted once; hovering a lost run shows why and when it was lost
- Member profiles with everyone's characters, collection and exaltations

**Server:** 
- Connect to your guild's own server, register and log in; your records sync between your PCs for free
- Admins: approve members, moderator tokens, and see which app and version each member uses

**Social:** 
- The chat log with its channels

**Settings:** 
- Sounds for PMs, party and guild messages, trades and loot bags; a startup sound
- Item sounds for the amulet, Coral Juice Refinery, Miniature Alien Generator and Protective Matrix, each with your own file and volume
- Ping lists for chat, items, enchants, entities and dungeons; a loot filter; six themes; logging

**Help:** 
- The Newcomer's Guide: a quick tour through the app on the first start
- Info > Logs to look at and export the log files
