//! Pictures of the window without showing it, to compare with the Java app's (`oracle/UiOracle.java`): each tab
//! painted into `<out dir>/<Tab>.png`, rendered offscreen (wgpu; a software renderer will do).
//!
//!   ui-shot <out dir> <width> <height> [--data <the Java app's data folder>] [--hover x,y] [tab...]
//!
//! With `--data` the views show that folder's records (read only) and its `assets`.

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    // `ui-shot skins <data dir>`: every character's look, "skin <charId> <w>x<h> <fnv of the ARGB pixels>"
    if let [cmd, dir] = args.as_slice()
        && cmd == "skins"
    {
        let dir = std::path::Path::new(dir);
        let data = lg_gamedata::GameData::load(&dir.join("assets"));
        let mut sprites = lg_gamedata::sprites::Sprites::new(&dir.join("assets"));
        let chars = lg_app::characters::Characters::import(dir);
        let mut ids: Vec<_> = chars.all().map(|c| c.char_id).collect();
        ids.sort_unstable();
        for id in ids {
            let c = chars.get(id).expect("char");
            let sprite = if c.skin > 0 { c.skin } else { c.class_type };
            let im = lg_ui::icons::skin_image(&mut sprites, &data, sprite, c.tex1, c.tex2, 96);
            let mut h: u64 = 0xcbf29ce484222325;
            for p in im.rgba.as_chunks::<4>().0 {
                for b in [p[3], p[0], p[1], p[2]] {
                    h = (h ^ u64::from(b)).wrapping_mul(0x100000001b3);
                }
            }
            println!("skin {id} {}x{} {h:016x}", im.w, im.h);
        }
        return;
    }
    let [out, w, h, rest @ ..] = args.as_slice() else {
        eprintln!("usage: ui-shot <out dir> <width> <height> [--data <dir>] [tab...]");
        std::process::exit(2);
    };
    let (data, rest) = match rest {
        [flag, d, tabs @ ..] if flag == "--data" => (Some(std::path::PathBuf::from(d)), tabs),
        tabs => (None, tabs),
    };
    // `--hover x,y`: the pointer rests there (to see a tooltip)
    let (hover, tabs) = match rest {
        [flag, at, tabs @ ..] if flag == "--hover" => {
            let (x, y) = at.split_once(',').expect("--hover x,y");
            (Some(egui::pos2(x.parse().expect("x"), y.parse().expect("y"))), tabs)
        }
        tabs => (None, tabs),
    };
    // UI_SHOT_NOW=<ms>: the views' clock (pictures of the last 24 hours of an older sample)
    let now = std::env::var("UI_SHOT_NOW")
        .ok()
        .and_then(|v| v.trim().parse::<i64>().ok())
        .unwrap_or_else(|| std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as i64));
    let assets = data.as_ref().map(|d| d.join("assets")).unwrap_or_default();
    let world = lg_world::World::new(lg_gamedata::GameData::load(&assets));
    let mut app = lg_app::App::importing(world, now, data.as_deref());
    // the folder's settings, then any from UI_SHOT_PROPS ("key=value" lines, e.g. "focus=run:12")
    let props = lg_app::properties::Properties::in_memory(&format!(
        "{}\n{}",
        data.as_ref().and_then(|d| std::fs::read_to_string(d.join("realmShark.properties")).ok()).unwrap_or_default(),
        std::env::var("UI_SHOT_PROPS").unwrap_or_default()
    ));
    let (w, h): (f32, f32) = (w.parse().expect("width"), h.parse().expect("height"));
    let tabs: Vec<String> = if tabs.is_empty() { lg_ui::shell::TABS.iter().map(|t| t.to_string()).collect() } else { tabs.to_vec() };
    std::fs::create_dir_all(out).expect("out dir");
    for tab in tabs {
        // "card:<charId>[:<mode>]": the loadout card "Copy as image" makes, alone; with a mode, the character in a
        // marked run of it
        let card = tab.strip_prefix("card:").map(|t| t.split_once(':').map_or((t, None), |(i, m)| (i, Some(m))));
        if let Some((id, mode)) = card.and_then(|(i, m)| i.parse::<i32>().ok().map(|i| (i, m))) {
            if let Some(mode) = mode {
                let run = app.upe.runs.get(&id).or_else(|| app.upe.runs.values().next()).cloned();
                if let Some(mut run) = run {
                    run.char_id = id;
                    run.mode = mode.to_string();
                    run.marked = true;
                    run.broken = None;
                    app.upe.runs.insert(id, run);
                }
            }
            let mut win = lg_ui::window::Window::new(&assets, props.clone());
            win.palette = lg_ui::theme::palette(None);
            let ch = app.characters.get(id).expect("character");
            let rect = std::cell::Cell::new(egui::Rect::NOTHING);
            let mut themed = false;
            let mut harness = egui_kittest::Harness::builder()
                .with_size(egui::vec2(w, h))
                .with_pixels_per_point(1.0)
                .with_accessibility_check(false)
                .wgpu()
                .build_ui(|ui| {
                    if !themed {
                        lg_ui::theme::apply(ui.ctx(), &win.palette);
                        themed = true;
                    }
                    let rarity = win.characters.rarity(ui.ctx());
                    let ctx = ui.ctx().clone();
                    rect.set(lg_ui::card::paint(
                        ui.painter(),
                        &ctx,
                        &mut win.icons,
                        app.world.data(),
                        &rarity,
                        &win.palette,
                        ch,
                        lg_ui::card::mode_of(&app, ch.char_id),
                        egui::Pos2::ZERO,
                        lg_ui::card::SCALE,
                    ));
                });
            harness.run_steps(3);
            let img = harness.render().expect("render");
            drop(harness);
            let frame = egui::ColorImage::from_rgba_unmultiplied([img.width() as usize, img.height() as usize], img.as_raw());
            let card = lg_ui::card::crop(&frame, rect.get(), 1.0, lg_ui::card::SCALE);
            let out_img = image::RgbaImage::from_raw(
                card.size[0] as u32,
                card.size[1] as u32,
                card.pixels.iter().flat_map(|c| c.to_srgba_unmultiplied()).collect(),
            )
            .expect("image");
            out_img.save(std::path::Path::new(out).join(format!("card-{id}.png"))).expect("save");
            println!("card {id}");
            continue;
        }
        // a fresh window per picture, the Java default theme as the oracle shows it
        let mut win = lg_ui::window::Window::new(&assets, props.clone());
        win.palette = lg_ui::theme::palette(None);
        // "Server:online": connected to a made-up server, not logged in (the login form)
        app.server = Default::default();
        let dialog = match tab.as_str() {
            "Server:forgot" => Some(lg_ui::window::ServerAsk::OpenForgot("Goldwhisker".into())),
            "Server:change" => Some(lg_ui::window::ServerAsk::OpenChange),
            _ => None,
        };
        // "Guild:Member": logged in, a member's profile made from these records
        if tab == "Guild:Member" {
            app.server.url = "https://loot.example.com".into();
            app.server.state = lg_app::server::State::Online;
            app.server.user = Some(lg_app::server::User { id: 1, ign: "Goldwhisker".into(), ..Default::default() });
            let member = lg_app::guild::Member {
                id: 1,
                ign: "Goldwhisker".into(),
                guild_rank: 40,
                last_active: now - 3_600_000,
                items: app.collection.all().count() as i64,
                finds: 771,
                fame: 108_360,
                exalts: 144,
                characters: 20,
                maxed: 15,
                ..Default::default()
            };
            let profile = lg_app::guild::MemberProfile {
                guild: "Gilded Goblins".into(),
                member,
                characters: app.characters.all().cloned().collect(),
                exalts: app.exalts.progress.iter().map(|(k, v)| (k.to_string(), v.clone())).collect(),
                collection: app.collection.all().map(|(id, e)| (id.to_string(), i64::from(e.count))).collect(),
            };
            app.server.guild.member = Some((1, Some(Ok(profile))));
        }
        // "Guild:Quest Map": logged in, a map of every style (the current one half cleared, the others ended,
        // completed or barely begun)
        if tab == "Guild:Quest Map" {
            use lg_app::guild::{QuestMap, QuestMapClear, QuestMaps};
            app.server.url = "https://loot.example.com".into();
            app.server.state = lg_app::server::State::Online;
            app.server.user = Some(lg_app::server::User { id: 1, ign: "Goldwhisker".into(), ..Default::default() });
            let dungeons: Vec<String> = [
                "Snake Pit",
                "Spider Den",
                "Pirate Cave",
                "Forest Maze",
                "Tomb of the Ancients",
                "Cultist Hideout",
                "The Shatters",
                "Oryx's Sanctuary",
            ]
            .iter()
            .map(|d| d.to_string())
            .collect();
            let map = |id: i64, style: &str, variant: i32, cleared: usize, ended: bool| QuestMap {
                id,
                name: format!("The {style} map"),
                style: style.into(),
                variant,
                dungeons: dungeons.clone(),
                clears: (0..cleared)
                    .map(|i| QuestMapClear { user_id: 1, ign: "Goldwhisker".into(), at: now - 86_400_000 + i as i64 * 3_600_000 })
                    .collect(),
                starts_at: now - 2 * 86_400_000,
                ends_at: if ended { now - 3_600_000 } else { now + 5 * 86_400_000 },
                completed_at: (cleared == 8).then_some(now - 7_200_000),
            };
            let styles = ["hex", "overworld", "oryx", "constellation", "neon", "adventure", "hunt", "hellforge"];
            app.server.guild.quest_maps = Some(Ok(QuestMaps {
                guild: "Gilded Goblins".into(),
                now,
                current: Some(map(100, "parchment", 0, 3, false)),
                past: styles
                    .iter()
                    .enumerate()
                    .map(|(i, s)| map(i as i64 + 1, s, i as i32, [3, 8, 2, 5, 3, 4, 8, 1][i], i != 0 && i != 2))
                    .collect(),
            }));
        }
        // "Guild:UPE": logged in, this data's marked game mode runs as one member's, and a guildmate's run that took
        // half of them
        if tab == "Guild:UPE" {
            use lg_app::guild::{CaravanCharacter, UpeBoard, UpeFind, UpeMember, UpeRun};
            app.server.url = "https://loot.example.com".into();
            app.server.state = lg_app::server::State::Online;
            app.server.user = Some(lg_app::server::User { id: 1, ign: "Goldwhisker".into(), ..Default::default() });
            let runs: Vec<UpeRun> = app
                .upe
                .runs
                .values()
                .filter(|r| r.marked)
                .map(|r| {
                    let ch = app.characters.get(r.char_id);
                    UpeRun {
                        char_id: i64::from(r.char_id),
                        mode: r.mode.clone(),
                        marked_at: 0,
                        lost: r.broken.is_some(),
                        why: r.broken.clone().unwrap_or_default(),
                        lost_at: r.broken_at,
                        notes: r.notes.clone(),
                        alive: ch.is_some_and(|c| c.alive),
                        character: ch.map(|c| CaravanCharacter {
                            class_type: c.class_type,
                            class_name: c.class_name.clone(),
                            skin: c.skin,
                            tex1: c.tex1,
                            tex2: c.tex2,
                            fame: 0,
                        }),
                        gathered: r
                            .gathered
                            .iter()
                            .map(|g| UpeFind {
                                item_id: g.item,
                                enchants: g.enchants.as_ref().map_or(-1, |e| e.len() as i32),
                                r#where: g.r#where.clone(),
                                at: g.at,
                            })
                            .collect(),
                    }
                })
                .collect();
            let mut mate = runs.iter().max_by_key(|r| r.gathered.len()).cloned().unwrap_or_default();
            mate.gathered.truncate(mate.gathered.len() / 2);
            mate.lost = false;
            mate.alive = true;
            app.server.guild.upe = Some(Ok(UpeBoard {
                guild: "Gilded Goblins".into(),
                members: vec![
                    UpeMember { id: 1, ign: "Goldwhisker".into(), guild_rank: 40, runs },
                    UpeMember { id: 2, ign: "Valhirs".into(), guild_rank: 10, runs: vec![mate] },
                ],
            }));
        }
        // "Guild:Loot Caravan": logged in, this data's finds as yours, and guildmates who found a share of them (the
        // last one few enough to walk)
        if tab == "Guild:Loot Caravan" {
            use lg_app::guild::{Caravan, CaravanCharacter, CaravanItem, CaravanMember};
            app.server.url = "https://loot.example.com".into();
            app.server.state = lg_app::server::State::Online;
            app.server.user = Some(lg_app::server::User { id: 1, ign: "Goldwhisker".into(), ..Default::default() });
            let mut finds: Vec<(i64, CaravanItem)> = app
                .collection
                .all()
                .flat_map(|(id, e)| {
                    e.history.iter().flatten().map(move |f| {
                        (
                            f.at,
                            CaravanItem {
                                item_id: id,
                                enchants: f.enchants,
                                bag: lg_app::guild::bag_name(&f.bag),
                                time: f.time.clone(),
                                r#where: f.r#where.clone(),
                            },
                        )
                    })
                })
                .collect();
            finds.sort_by_key(|f| std::cmp::Reverse(f.0));
            let chars: Vec<CaravanCharacter> = {
                let mut seen = std::collections::HashSet::new();
                app.characters
                    .all()
                    .filter(|c| seen.insert(c.class_type))
                    .map(|c| CaravanCharacter {
                        class_type: c.class_type,
                        class_name: c.class_name.clone(),
                        skin: c.skin,
                        tex1: c.tex1,
                        tex2: c.tex2,
                        fame: c.fame,
                    })
                    .collect()
            };
            let member = |id: i64, ign: &str, rank: i32, every: usize, cap: usize, ch: usize| {
                let items: Vec<CaravanItem> = finds.iter().skip(id as usize - 1).step_by(every).take(cap).map(|f| f.1.clone()).collect();
                let mut by_bag: Vec<(String, i64)> = Vec::new();
                for i in &items {
                    match by_bag.iter_mut().find(|b| b.0 == i.bag) {
                        Some(b) => b.1 += 1,
                        None => by_bag.push((i.bag.clone(), 1)),
                    }
                }
                CaravanMember {
                    id,
                    ign: ign.into(),
                    guild_rank: rank,
                    by_bag,
                    total: items.len() as i64,
                    last_drop: items.first().map(|i| i.time.clone()),
                    items: items.into_iter().take(120).collect(),
                    tally: Vec::new(),
                    character: chars.get(ch % chars.len().max(1)).cloned(),
                }
            };
            app.server.guild.caravan = Some(Ok(Caravan {
                guild: "Gilded Goblins".into(),
                range: "all".into(),
                members: vec![
                    member(1, "Goldwhisker", 40, 1, 160, 0),
                    member(2, "Valhirs", 30, 2, 90, 1),
                    member(3, "Saltusk", 20, 3, 45, 2),
                    member(4, "Fendor", 10, 5, 16, 3),
                    member(5, "Orrbrix", 0, 9, 3, 4),
                ],
            }));
        }
        // "badge": inside a dungeon for 4:40 (the menu bar's dungeon box)
        if tab == "badge" {
            app.dungeon_visits.current = Some(lg_app::dungeon_visits::Visit::new("Snake Pit", now - 280_000));
        }
        // "Battles:mates:<start>": that visit with a guildmate the server has (as the local test server answers)
        let mates_visit = tab.strip_prefix("Battles:mates:").and_then(|t| t.parse::<i64>().ok());
        if let Some(start) = mates_visit {
            use lg_app::guild::{Looks, RunLook};
            app.server.url = "https://loot.example.com".into();
            app.server.state = lg_app::server::State::Online;
            app.server.user = Some(lg_app::server::User { id: 1, ign: "Goldwhisker".into(), ..Default::default() });
            // a visit without an amulet save gets one: two big hits late in it, the amulet firing after them
            if let Some(v) = app.dungeon_visits.visits.iter_mut().find(|v| v.start == start)
                && v.amulet_saves.is_empty()
                && v.duration_ms > 60_000
            {
                let at = (v.duration_ms * 7 / 8) as i32;
                for (t, d) in [(at - 900, 480), (at - 150, 290)] {
                    let i = v.hit_time.partition_point(|&x| x < t);
                    v.hit_time.insert(i, t);
                    v.hit_damage.insert(i, d);
                    // by the enemy that hit you before them
                    if i <= v.hit_enemy.len() {
                        let e = v.hit_enemy[..i].iter().rev().copied().find(|&e| e > 0).unwrap_or(0);
                        v.hit_enemy.insert(i, e);
                    }
                }
                v.amulet_saves.push(at);
            }
            // the guildmates: the two players ranked highest on the visit's last boss beside you, else Valhirs
            let visit = app.dungeon_visits.visits.iter().find(|v| v.start == start).cloned();
            let mut mates: Vec<(String, i32, i32, i32, i32)> = visit
                .as_ref()
                .and_then(|v| lg_app::dungeon_visits::run_of(v, &app.dps_history))
                .and_then(|r| r.main_boss())
                .map(|b| {
                    b.players.iter().filter(|p| !p.me).take(2).map(|p| (p.name.clone(), p.class_type, p.skin, p.tex1, p.tex2)).collect()
                })
                .unwrap_or_default();
            if mates.is_empty() {
                mates.push(("Valhirs".into(), 782, 0, 0, 0));
            }
            app.server.mates.names = mates.iter().map(|m| m.0.to_lowercase()).collect();
            if let Some(v) = visit {
                let span = v.duration_ms.max(60_000);
                let looks = mates
                    .iter()
                    .enumerate()
                    .map(|(i, (name, class, skin, t1, t2))| {
                        // hits spread over the visit, the second mate taking fewer, a quickslot used after the big ones
                        let n = if i == 0 { 9 } else { 5 };
                        let hit_at: Vec<i64> = (0..n).map(|k| span * (k as i64 * 2 + 1 + i as i64) / (n as i64 * 2 + 2)).collect();
                        let hit_damage: Vec<i32> =
                            (0..n).map(|k| [120, 260, 90, 340, 150, 210, 80, 400, 130][k % 9] - i as i32 * 30).collect();
                        let look = RunLook {
                            player: name.clone(),
                            class_type: *class,
                            skin: *skin,
                            tex1: *t1,
                            tex2: *t2,
                            source: "visit".into(),
                            max_hp: 980 - i as i32 * 120,
                            quick_used: Some(if i == 0 { vec![2594, 2594, 2793] } else { vec![2594] }),
                            quick_at: Some(if i == 0 {
                                vec![hit_at[3] + 500, hit_at[7] + 500, hit_at[8] + 900]
                            } else {
                                vec![hit_at[3] + 700]
                            }),
                            hit_at,
                            hit_damage,
                            ..Default::default()
                        };
                        (name.to_lowercase(), vec![look])
                    })
                    .collect();
                app.server.mates.looks.insert(format!("{}/{}", v.dungeon, v.start), Looks::Got(looks));
            }
        }
        // "Profile:mates": the overview with the players of your recent runs as guildmates
        if tab == "Profile:mates" {
            let names: std::collections::HashSet<String> = app
                .dps_history
                .runs
                .iter()
                .take(20)
                .flat_map(|r| r.bosses.iter().flat_map(|b| b.players.iter()))
                .filter(|p| !p.me)
                .take(3)
                .map(|p| p.name.to_lowercase())
                .collect();
            app.server.mates.names = names;
        }
        let admin = tab == "Server:admin";
        let shown = if mates_visit.is_some() {
            "Battles".to_string()
        } else if tab == "Server:online" || dialog.is_some() || admin {
            app.server.url = "https://loot.example.com".into();
            app.server.state = lg_app::server::State::Online;
            app.server.status = Some(lg_app::server::Status {
                version: "1.4.0".into(),
                api_version: lg_app::server::API_VERSION,
                admin_claimed: true,
                registration: "approval".into(),
            });
            if admin {
                // logged in as the root admin, with made-up lists
                use lg_app::server::{IssuedToken, OpenToken, User};
                let user = |id: i64, ign: &str, role: &str, root: bool, status: &str| User {
                    id,
                    ign: ign.into(),
                    status: status.into(),
                    role: role.into(),
                    root,
                    created_at: 1_795_000_000_000 + id * 3_600_000,
                    last_login_at: (id != 4).then_some(1_795_200_000_000 + id * 60_000),
                    client: match id {
                        4 => String::new(),
                        _ if id % 2 == 1 => "Rust v0.0.1".into(),
                        _ => "Java v0.9.2".into(),
                    },
                };
                app.server.user = Some(user(1, "Goldwhisker", "admin", true, "active"));
                let m = &mut app.server.moderation;
                m.pending = vec![user(7, "Newbie", "user", false, "pending")];
                m.members = vec![
                    user(1, "Goldwhisker", "admin", true, "active"),
                    user(2, "Helper", "admin", false, "active"),
                    user(3, "Watcher", "moderator", false, "active"),
                    user(4, "Quiet", "user", false, "active"),
                ];
                m.tokens = vec![OpenToken {
                    id: 1,
                    note: "For Ana".into(),
                    created_by: "Goldwhisker".into(),
                    created_at: 0,
                    expires_at: Some(1_795_300_000_000),
                }];
                m.last_issued = Some(IssuedToken {
                    id: 1,
                    token: "LGM-ABCD-EFGH-IJKL-MNOP".into(),
                    note: "For Ana".into(),
                    expires_at: 1_795_300_000_000,
                });
                app.sync.phase = lg_app::sync::Phase::UpToDate;
            }
            "Server".to_string()
        } else {
            tab.clone()
        };
        match tab.strip_prefix("Battles:run:").and_then(|t| t.parse::<i64>().ok()) {
            // a run under DPS, its latest boss open
            Some(start) => {
                win.open("Battles:dps");
                win.battles.dps.show_run(&app.dps_history, Some(start));
            }
            None => match tab.strip_prefix("Settings:dialog:").and_then(|t| t.parse::<usize>().ok()) {
                // a ping editor open over Settings
                Some(k) => {
                    win.open("Settings");
                    win.ask(lg_ui::window::Ask::OpenDialog(k));
                    win.take_asks();
                }
                None => {
                    win.open(&shown);
                    // "welcome": the first start's greeting, with the offer of the Java app's records
                    if tab == "welcome" {
                        win.open("Profile");
                        win.welcome = true;
                        win.java_offer =
                            Some(lg_ui::welcome::JavaOffer { path: r"C:\Users\Someone\AppData\Roaming\LootGobbler".into(), take: true });
                    }
                    // "about": Info > About over the Profile
                    // "Inventory:st:forge": ST Sets with a set clicked whose missing piece you can forge (else one the
                    // forge makes from tokens), its class alone in the grid
                    if tab == "Inventory:st:forge" {
                        win.open("Inventory:st");
                        let data = app.world.data();
                        let stock = lg_app::st_forge::Stock::of(&app, 0);
                        let has = |id: i32| app.collection.entry(id).is_some_and(|e| e.held() > 0);
                        let plans: Vec<(&lg_gamedata::st_sets::StSet, Vec<lg_app::st_forge::Piece>)> =
                            data.st_sets.iter().map(|s| (s, stock.plan(data, s, &has))).collect();
                        let pick = plans
                            .iter()
                            .find(|(_, p)| p.iter().any(|c| c.craftable))
                            .or_else(|| plans.iter().find(|(_, p)| p.iter().any(|c| !c.tokens.is_empty())))
                            .or_else(|| plans.iter().find(|(_, p)| !p.is_empty()));
                        if let Some((s, _)) = pick {
                            win.inventory.st_selected = Some(s.name.clone());
                            win.inventory.st_class = Some(s.class_name.clone());
                        }
                    }
                    // "Inventory:st:set:<title>[:<piece id>]": ST Sets with that set's bar open (that piece alone)
                    if let Some(rest) = tab.strip_prefix("Inventory:st:set:") {
                        win.open("Inventory:st");
                        let (title, piece) = match rest.rsplit_once(':') {
                            Some((t, p)) if p.parse::<i32>().is_ok() => (t, p.parse::<i32>().ok()),
                            _ => (rest, None),
                        };
                        if let Some(s) = app.world.data().st_sets.iter().find(|s| s.title == title) {
                            win.inventory.st_selected = Some(s.name.clone());
                            win.inventory.st_piece = piece;
                        }
                    }
                    // "Inventory:keys:<item id>": the Keys page with that key clicked
                    if let Some(k) = tab.strip_prefix("Inventory:keys:").and_then(|k| k.parse::<i32>().ok()) {
                        win.open("Inventory:keys");
                        win.inventory.key_selected = Some(k);
                    }
                    if tab == "about" {
                        win.open("Profile");
                        let mut asks = Vec::new();
                        win.info.open("About", &mut asks);
                    }
                    if let Some(start) = mates_visit {
                        win.battles.selected = start;
                        win.battles.reveal = true;
                    }
                    if let Some(d) = dialog.clone() {
                        win.ask(lg_ui::window::Ask::Server(d));
                        win.take_asks();
                    }
                }
            },
        }
        let mut harness = egui_kittest::Harness::builder()
            .with_size(egui::vec2(w, h))
            .with_pixels_per_point(1.0)
            .with_accessibility_check(false)
            .wgpu()
            .build_ui(|ui| {
                // the whole window, without the margin the harness puts around it
                let mut all = ui.new_child(egui::UiBuilder::new().max_rect(ui.ctx().content_rect()));
                win.show(&mut all, &app, now);
            });
        harness.run_steps(3);
        if let Some(at) = hover {
            harness.input_mut().events.push(egui::Event::PointerMoved(at));
            harness.run_steps(2);
            // past the tooltip delay (the harness's clock moves a step at a time)
            for _ in 0..60 {
                harness.step();
            }
            println!("cursor at ({}, {}): {:?}", at.x, at.y, harness.output().platform_output.cursor_icon);
        }
        // UI_SHOT_DRAG="x,y": press there, move 40 px right a step at a time, and tell whether the window was asked to
        // start moving (the title bar's drag)
        if let Some((dx, dy)) = std::env::var("UI_SHOT_DRAG").ok().and_then(|v| {
            let (a, b) = v.split_once(',')?;
            Some((a.trim().parse::<f32>().ok()?, b.trim().parse::<f32>().ok()?))
        }) {
            let at = egui::pos2(dx, dy);
            let started = |h: &egui_kittest::Harness<'_>| {
                h.output().viewport_output.values().any(|v| v.commands.iter().any(|c| matches!(c, egui::ViewportCommand::StartDrag)))
            };
            harness.input_mut().events.push(egui::Event::PointerMoved(at));
            harness.step();
            harness.input_mut().events.push(egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: Default::default(),
            });
            harness.step();
            let mut moved = started(&harness);
            for k in 1..=8 {
                harness.input_mut().events.push(egui::Event::PointerMoved(at + egui::vec2(5.0 * k as f32, 0.0)));
                harness.step();
                moved |= started(&harness);
            }
            harness.input_mut().events.push(egui::Event::PointerButton {
                pos: at + egui::vec2(40.0, 0.0),
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Default::default(),
            });
            harness.run_steps(3);
            println!("drag at ({dx}, {dy}): window drag {}", if moved { "STARTED" } else { "not started" });
            drop(harness);
            for a in win.take_asks() {
                if let lg_ui::window::Ask::Set(k, v) = a {
                    println!("  set {k} = {v}");
                }
            }
            continue;
        }
        let img = harness.render().expect("render");
        let path = std::path::Path::new(out).join(format!("{}.png", tab.replace(' ', "_").replace(':', "-")));
        img.save(&path).expect("save");
        println!("tab {tab}");
    }
}
