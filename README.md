<div align="center">

<img src="docs/img/loot-gobbler.png" alt="The Loot Gobbler hugging its sack of treasure" width="420">

# Loot Gobbler

### A tool, designed and built around what _you_ need
[Loot Gobbler Page](https://deliberated.github.io/Loot-Gobbler-Client/)
</div>

The Loot Gobbler is a sniffing tool that can act as your right hand for your RotMG adventures. It helps you track your UPEs, your characters, their inventories, and your dungeon and exaltation progress, and it suggests the ideal loadout and enchantments. All that while being written in Rust, so it won't bother your CPU!

## Features
If even one of these questions resonates with you, the Loot Gobbler will make you happy!
- Are you a **collector** who got an item and wondered whether you already have it?
- Do you often play **UPEs** and would love a third eye tracking your items?
- Have you waited for a tool that lets you **fine-tune your loadout items and the enchants** on them?
- Are you a Discord girl/boy who likes to show off your character or loot progress?
- Can I interest you in some **overlay features**, showing which buff is active or recharging, how well you are performing in Moonlight Village, or perhaps **when your matrix/ami procs**?
- Would you like to see your **dungeon history**, including how your **guildmates performed**, who dodged more bullets and who used their quickslots more often?
- How about **tracking your exaltation progress**?
- Did I already mention syncing with your guildmates to **check the progress within your guild**, allowing for internal challenges?

See the full set of features with screenshots on the [Loot Gobbler page](https://deliberated.github.io/Loot-Gobbler-Client/).


## Approach
The project was designed to give our guild a better game experience, no matter whether you are a new player or a veteran. Big disclaimer: this project is vibecoded, but heavily tested by our guildies and designed with time, taste, and love!

It is a fork of **Tomato**, built on the **RealmShark** sniffing engine (both MIT, see `THIRD_PARTY_LICENSE-RealmShark.md`), and rewritten in Rust. Many of the original features have been revisited and redesigned.

## Comparison to Existing Tools
There are great RotMG tools out there, and the Loot Gobbler does not try to replace them. It goes in its own direction: connecting the dots and showing you the big picture of **your** game. Where other tools do something really well, the Loot Gobbler brings in a similar style and features, so you have less to switch between.

The two closest tools are RealmHound and Baghound. Like the Loot Gobbler, both are desktop apps that passively read the game's traffic (RealmShark lives on in all three).

| | Loot Gobbler | [RealmHound](https://github.com/pillarzz/RealmHound) | [Baghound](https://baghound.app/) |
|---|---|---|---|
| Price | Free | Free | $6.6 per month |
| Open source | Yes (MIT) | Yes (MIT) | No |
| Language | Client in Rust, server in Go| Rust | Java
| Platforms | Windows, Linux, (soon macOS) | Windows, macOS, Linux | Windows |
| UPE / PPE runs | **Tracked per run:** its items, forge resources, missions, gift chest, and when and why a run was lost | — | — |
| Your UT/ST collection | Every item, filled in as you find and store it, with its full history | Rare drops by dungeon | Top-tier drops, and what you still own |
| Where each item sits | Per character, vault and gift chest | Search across all storage, vault views | — |
| Loadout and enchants | **A suggested loadout** from the items you own, enchants included | Theoretical DPS calculator | — |
| DPS and dungeon history | Per boss, with your guildmates' damage, dodges and quickslots side by side | Per boss | Per boss |
| Exaltations | Per class and stat, with recent progress | Per class and stat | — |
| Quests and missions | Season missions and battle pass | Tinkerer quests, season missions, objective tracker | — |
| Overlay | Buffs, Moonlight Village, matrix and ami procs | Live combat and loot | — |
| Fallen characters | Graveyard with kill history and full inventory view | — | Graveyard with killer and losses |
| Discord | Loot cards | — | Loot cards |
| Maintenance | **Actively developed**, shaped by our guild's feedback and open to everyone's issues and pull requests | **Actively developed**, with community contributions (macOS and Linux support came from contributors) | No code changes since May 2026, one developer, no outside contributions |
| Sharing | **Your guild's own server:** guild quests, leaderboards, the loot caravan and member profiles | — | Public web profiles |
| Syncing between PCs | Free, through your guild's server | — | Pro Feature |

See the full feature comparison for more details.

## Requirements
**Windows**
- Windows 10 or 11, 64-bit
- [Npcap](https://npcap.com), which lets the Loot Gobbler see the game's traffic. The default install options are fine.
- RotMG Exalt installed on the same PC

**Linux**
- A 64-bit (x86_64) distribution
- RotMG Exalt installed through Steam (Proton)
- `libpcap`, to read the network traffic, and the permission to capture it

The **rpm** takes care of both: it installs libpcap along with it and gives the Loot Gobbler the permission.

With the **tar.gz**, the Loot Gobbler sets itself up the first time you start the sniffer:
1. It tells you what is missing and asks you to click **Grant permission**.
2. Your system asks for your password.
3. It installs what is missing (libpcap, and `setcap` for the permission), gives itself the permission and starts again with the sniffer running.

After an update the permission has to be given again, so it asks once more. This works on Debian, Ubuntu, Fedora, openSUSE and Arch, and on any distribution with a password prompt (polkit) when nothing needs installing. Otherwise it shows you the commands to run yourself, for example on Ubuntu:

```sh
sudo apt-get install -y libpcap0.8t64 libcap2-bin   # libpcap, and setcap for the permission
sudo setcap cap_net_raw,cap_net_admin=eip ./LootGobbler
```

Then start the Loot Gobbler again.

A [Loot Gobbler server](https://github.com/Deliberated/Loot-Gobbler-Server) is optional. Without one, everything except the guild features and the syncing between PCs works on its own.


## Download
Get the latest version from the [Releases page](https://github.com/Deliberated/Loot-Gobbler-Client/releases/latest):

| File | For |
|---|---|
| `LootGobbler.exe` | Windows: the program itself, ready to start |
| `LootGobbler-portable.zip` | Windows: the same program in its own folder |
| `LootGobbler-x86_64.rpm` | Linux (Fedora, openSUSE and others): installs to `/opt` with the rights to capture already set |
| `LootGobbler-linux-x86_64.tar.gz` | Linux, any distribution: unpack and start |

The Loot Gobbler keeps its records next to the program, so give it a folder of its own (the portable zip does that for you). It checks for new versions on its own and tells you when one is out.

**Windows says "Windows protected your PC"?** That is Microsoft Defender SmartScreen: it warns about programs that are not signed with a paid certificate and that few people have run yet, which is true for every new release of a small open-source project like this one. Click **More info** if needed, then **Run anyway**. The program's code is all here to read, and each release is built by GitHub from it.

**Have you used the previous Java-based Loot Gobbler?** On its first start, the new one offers to take over your records and settings from the Java app's folder. If you installed the Java app at its default place, leave the suggested path as it is.


## Building from Source
You need a recent stable [Rust toolchain](https://rustup.rs) (1.88 or newer).

```sh
git clone https://github.com/Deliberated/Loot-Gobbler-Client.git
cd Loot-Gobbler-Client/rust
cargo build --release -p lg-gobbler
```

The program lands in `target/release/` (`loot-gobbler.exe` on Windows, `loot-gobbler` on Linux). Useful options when starting it:

```
loot-gobbler [--data <folder>] [--java <Java app folder>] [--path <resources.assets>] [--offline]
```

- `--data`: keep the records in this folder instead of next to the program
- `--java`: read the records of a Java Loot Gobbler from this folder
- `--path`: the game's `resources.assets`, if it is not found by itself
- `--offline`: no update checks

The Java version's sources (`src/`, `build.gradle`) are still in this repository for reference; they are no longer developed.

## Game Assets
The Loot Gobbler does not ship any of the game's art or data. On its first start it finds your RotMG Exalt install and reads the item, dungeon and sprite data from the game's own `resources.assets`:

- **Windows:** `%LOCALAPPDATA%\RealmOfTheMadGod\Production\RotMG Exalt_Data\` (or the same path under your Documents folder)
- **Linux:** the game in any of your Steam libraries, run through Proton

What it reads is kept in an `assets` folder next to its records. It reads the game again only when the game was updated, so after a game patch the first start takes a moment longer. If your game sits somewhere else, point the Loot Gobbler to it with `--path`.

## Legal Disclaimer
The Loot Gobbler is a fan-made project. It is not made, endorsed or supported by DECA Live Operations, and Realm of the Mad God and all of its art and data belong to their owners.

The Loot Gobbler only listens to the game's traffic on your own PC. It never sends, blocks or changes anything the game sends or receives. Even so, passive tools like this are not officially approved by DECA, so **you use it at your own risk**.

It only talks to:
- **GitHub**, to check for new versions
- **realmofthemadgod.com**, to load your character list with the login token from your own game traffic. The token goes only to the game's own servers.
- **The Loot Gobbler server you set up**, if you set one up

## License
The Loot Gobbler is open source under the [MIT License](LICENSE).

## Acknowledgements
- **[RealmShark](https://github.com/X-com/RealmShark)** and **Tomato** by X-com: the sniffing engine and the app the Loot Gobbler started from (MIT, see [`THIRD_PARTY_LICENSE-RealmShark.md`](THIRD_PARTY_LICENSE-RealmShark.md)). Its bag and chat sounds come from Tomato.
- **[egui](https://github.com/emilk/egui)**, which draws the whole window
- **[Jersey 10 and Jersey 15](https://github.com/scfried/soft-type-jersey)** by the Soft Type Project Authors, the pixel fonts (SIL Open Font License, see [`rust/assets/fonts/`](rust/assets/fonts/))
- **[Npcap](https://npcap.com)** and **libpcap**, which capture the traffic
- Our guildies, who tested every version and told us what was wrong with it
