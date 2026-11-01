# Loot Gobbler

Your right hand for your RotMG adventures: a loot log with bag sounds, a lifetime UT/ST collection,
DPS logs per boss, character and exaltation tracking, chat pings and Discord loot alerts.

Loot Gobbler only reads the game's network traffic. It never sends, blocks or changes game packets.
It is a fork of [Tomato](https://github.com/X-com/RealmShark) built on the RealmShark engine.

## Download

Get the latest version from the [Releases page](../../releases/latest):

| File | What it is |
|---|---|
| **LootGobbler.exe** | Setup. Installs Loot Gobbler with a Start menu and desktop shortcut. No admin rights needed. |
| **LootGobbler-portable.zip** | No install. Unzip anywhere and run `LootGobbler\LootGobbler.exe`. Your data stays in that folder. |

Java is included, so you don't need to install it.

## Before the first start: install Npcap

Loot Gobbler needs **Npcap** to see the game's traffic.

1. Download it from [npcap.com](https://npcap.com/#download).
2. In the installer, tick **"Install Npcap in WinPcap API-compatible Mode"**.
3. Finish the install. You only need to do this once.

## Getting started

1. Start Loot Gobbler.
2. Click **Start Sniffer** in the status bar.
3. Start RotMG, or change server if it's already running.

On the first start it reads the item and sprite data from your RotMG Exalt install, so the game must be installed
on the same PC.

## Your data

Settings, your collection, characters, DPS history and Discord webhooks are saved in the Loot Gobbler folder:

- Setup version: the folder you installed to, `%APPDATA%\LootGobbler` by default
- Portable version: the folder you unzipped

To update, install the new setup over the old one, or unzip the new portable version over the old folder.
Your data is kept either way. Keep your `realmShark.properties` private: it contains your Discord webhook URLs, and
anyone who has one can post to that channel.

## Network connections

Besides reading game traffic, Loot Gobbler connects to:

- `realmofthemadgod.com` (Deca's official servers) to load your character list, using your own login token from your traffic
- `api.realmshark.cc` for Crucible data and a chat spam-filter list
- your Discord webhooks, if you add any

## Good to know

- Windows only, and one game client at a time.
- Game updates can break packet reading until Loot Gobbler is updated.
- Passive sniffers are not officially approved by Deca. Use at your own risk.

## License

MIT, see [LICENSE](LICENSE). Includes RealmShark/Tomato (MIT, see
[THIRD_PARTY_LICENSE-RealmShark.md](THIRD_PARTY_LICENSE-RealmShark.md)) and the Jersey fonts (SIL Open Font License).
