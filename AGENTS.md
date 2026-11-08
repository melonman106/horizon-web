# Horizon: a map for agents

Minecraft: Java Edition 26.3 rewritten in Rust (wgpu). Vanilla is the specification wherever it has the
feature. Read [CONTRIBUTING.md](CONTRIBUTING.md) once; this page tells you where things are.
Feature → files → harness for the most-changed areas: [docs/architecture.md](docs/architecture.md).

## Rules that bite

- **Never commit Mojang's files**: game jars, assets, decompiled source, prepared resources (`.local/`).
  `tools/check_public_tree.py` rejects them; `git config core.hooksPath .githooks` runs it on commit.
- **Keep your runs away from real player data.** Any game or launcher you start gets fresh `mktemp -d`
  folders in `HORIZON_HOME` (logs, crash reports, telemetry consent, recordings; not saves), `HORIZON_LAUNCHER_HOME`, `XDG_DATA_HOME` and
  `XDG_CONFIG_HOME`, plus `HORIZON_LAUNCHER_NO_SELF_UPDATE=1` and `HORIZON_NO_AUDIO=1`. Saves, `options.txt` and
  caches go in the game folder, by default the working directory: start the game from a scratch folder with
  `--game-dir` pointing at it, or they land in the repo root, hidden by `.gitignore`. For the
  launcher, `HORIZON_LAUNCHER_TRASH`, `HORIZON_LAUNCHER_CLIPBOARD` and `HORIZON_LAUNCHER_DESKTOP` pointing into the scratch home. Never write to
  `~/.local/share/{applications,horizon,horizon-launcher}` or `~/.local/bin`.
- **Mark what an agent posts.** A PR body, issue or review written by an agent says so in its first line,
  naming the person it works for and the tool, e.g. `> 🤖 Opened by <login>'s AI agent (<tool, model>), not by
  <login> personally.` (issues say "Filed by", reviews "Review by"; every comment starts with `🤖 Agent:`).
  Write only as the person you work for.
- **Attribution.** Commits end with a `Co-Authored-By:` line for the agent that wrote them, and PR bodies with
  the tool's own "Generated with" line, if it adds one.
- Everything players run is Rust. Python/Java only for dev tools and harness drivers in `tools/`.
- Horizon-only features follow [docs/design-system.md](docs/design-system.md) and are off by default where
  they would change the vanilla look.
- Small PRs (< ~500 lines).
- Check for existing work before you start (open PRs and issues in the area); don't duplicate it.
- **No hardcoded user-facing strings.** Keep UI translation-ready from the start. In the launcher (`launcher/`), wrap visible text with `tr("...")` or `trf("... {arg} ...", &[("arg", &val)])` from `crate::i18n` instead of raw string literals or `format!`. In the game, use translation keys (`Text::tr`, `lang::get`), never hardcoded English strings.
- Upstream's process (labels, `/claim` on issues, batched releases) lives in [CONTRIBUTING.md](CONTRIBUTING.md)
  and [docs/release-flow.md](docs/release-flow.md); it applies to PRs sent to horizon-rust/horizon, not to this fork.
- **Controllers reach what registers.** Draw controls with a registering helper (`ui::button`, `settings_shell::foot_button`); `graphics_style::button` and `screen_draw::button` only paint, and registering one part of a screen ends its crosshair, so register all of it.

## Crates

Game workspace (root `Cargo.toml`); `launcher/` and `crates/horizon-telemetry-server` are separate workspaces.

| Crate | Owns |
|---|---|
| `horizon-client` | Gameplay library (`src/lib.rs`: `game.rs` + `game/`, `entry.rs`) and thin executable (`src/main.rs`). Also the dedicated server (`--dedicated`). Re-exports `horizon-client-lib` for existing mod paths. |
| `horizon-client-lib` | Reusable client systems, `horizon-ctl`, probes and their tests. Its manifest points to the existing sources under `crates/horizon-client/`; the library root is `src/client_lib.rs`. It never depends on gameplay. |
| `horizon-items` | Blocks and items below gameplay: `blocks.rs` (block facts), `items.rs`, `item_data.rs` (components), `enchant.rs`, `cooldowns.rs`, `bundle.rs`, `item_nbt.rs`. Re-exported as `horizon_client::items`... |
| `horizon-command` | Command parsing (Brigadier): the vanilla tree, argument types, selectors, suggestions, the tree's network form. Re-exported as `horizon_client::command` |
| `horizon-client-core` | The client lib's bottom layer: `text.rs`, `lang.rs`, `snbt.rs`, `nbt_path.rs`, `component.rs`, `scoreboard.rs`, `bossbar.rs`, `gamerules.rs`, `block_entity_nbt.rs`, registry/particle/sound tables. Re-exported under the old paths (`horizon_client::text`...) |
| `horizon-client-input` | Key bindings and key names, toggle keys, mouse options, controller settings and glyphs, the clipboard, window options (`keybinds.rs`, `gamepad.rs`, `mouse.rs`, `window.rs`...). Re-exported under the old paths |
| `horizon-client-render` | The client's world data, rendering and screens (chunks, meshing, light, streaming, `gfx`, HUD, menus, settings), modules that horizon-client re-exports under the same paths. Nothing in it reaches gameplay |
| `horizon-world` | Vanilla world data with no gameplay loop: NBT, regions, chunks, `datafix/`, `worldgen/`, `level/` (server-side block behaviour, ticks, redstone), entity physics, items, effects, food, damage |
| `horizon-core` | Simulation primitives: Java `Random`s, `jmath` (HotSpot-exact `Math`), positions, noise, density, paletted storage, `version.rs` (the pinned version) |
| `horizon-protocol` | Network protocol 777: codecs, framing, encryption, `packets/{handshake,status,login,configuration,play}.rs`, `entity_data.rs` |
| `horizon-assets` | Resource decoding: block-state and item models, baking, atlases |
| `horizon-prepare` | Builds `.local/horizon-client/26.3` from the player's own Minecraft install (runs the Java exporters) |
| `horizon-server` | `horizon-server`: `server.toml`, CLI, supervisor of the headless game ([docs/hosting.md](docs/hosting.md)) |
| `horizon-server-files` | `server.properties`, eula, ops/whitelist/bans, usercache |
| `horizon-anticheat` | The dedicated server's anticheat, off by default: `[anticheat]` settings, violation levels, the checks' rules; the hub runs them (`lan/anticheat.rs`) ([docs/anticheat.md](docs/anticheat.md)) |
| `horizon-cosmetics` | Cosmetic items Horizon clients draw on players (off by default): the item format, bones and animations ([docs/cosmetics.md](docs/cosmetics.md)) |
| `horizon-friends`, `horizon-relay` | Play with Friends: invite codes, encrypted tunnel, the relay ([docs/play-with-friends.md](docs/play-with-friends.md)) |
| `horizon-mods`, `cargo-horizon-mod` | Content mods (`.hmod`: manifest, content fence, no code), the registry index and `.hpack`, and `cargo horizon-mod` (`new`, `build`, `run`, `check`, `registry`) ([docs/mods.md](docs/mods.md)). Code mods are native, built by the launcher ([docs/native-mods.md](docs/native-mods.md)) |
| `horizon-port-pack` | Porting Java modpacks ([docs/modpacks.md](docs/modpacks.md)) |
| `horizon-optifine` | OptiFine-format pack files below every feature that reads them: `.properties`, the name/range/block matchers, paths and `optifine/` folders, and custom entity models (`cem/`: `.jem`/`.jpm` parsing and baking) |
| `horizon-polytone` | Polytone-format pack files (`assets/*/polytone/`), read clean-room from the public wiki: targets, biome and dimension modifiers and their environment attribute layers (#11486) |
| `horizon-shader-api` | Horizon's own WGSL shader packs ([docs/api/shaders.md](docs/api/shaders.md)): `horizon-shaders.toml`, the `.hshader` package; shared by the game, the launcher and the registry |
| `horizon-shaders` | Shader packs (the `shaders/` folder format), rebuilt clean-room from a written spec of the format; no wgpu |
| `horizon-theme` | Design tokens and fonts for the launcher, the game's own menus and the website |
| `horizon-telemetry`, `horizon-telemetry-server`, `horizon-bench` | Opt-in data collection, its ingest server, the benchmark report format |
| `horizon-dualsense`, `horizon-threads`, `horizon-release-sign`, `horizon-bots`, `xtask` | DualSense output, thread priorities, signed release checksums, load-test bots, repo tasks |
| `launcher/` | GPUI launcher: `main.rs` (UI), `backend.rs` (installs, updates), `cli.rs`, `mcp.rs`, `*_page.rs` |

Other top-level folders: `tools/` (harnesses and dev scripts), `tests/` (Python unit tests: `python3 -m
unittest discover -s tests`), `mods/horizon/src/test/java/dev/horizonlab/parity/` (Java oracles),
`docs/`, `.github/workflows/`. The website (playhorizon.dev) is a separate, private repository upstream: its build
reads `README.md`, `launcher/links.json`, `docs/api/`, `roadmap/roadmap.json` and `crates/horizon-theme/tokens.css` from main.

## Write tests with signal

- Test observable behavior, not words in docs, comments or source. Keep checks for machine-consumed contracts.
- Use independent expected values or a vanilla oracle; do not derive the answer from the code under test.
- Test outcomes, not only calls to stubs. Use mocks to isolate behavior, not as the behavior itself.
- Skip tests of trivial constants, derives and getters, or behavior already covered by another test.
- Avoid pinning implementation details; a harmless refactor should not break a behavior test.
- Make slow tests faster without removing cases or weakening their coverage.
- Never delete parity harnesses or tests that pin vanilla behaviour: they are the spec.

## Build and test fast

```sh
python3 tools/pr_ready.py                  # the self-check: fmt, clippy -D warnings, touched crates' tests, harnesses; paste its block in the PR
python3 tools/pr_ready.py --quick          # same without harnesses
cargo test -p horizon-client -p horizon-client-lib --lib --bins --tests   # skips the examples (pr_ready does this); `--doc` separately
cargo test -p horizon-client-lib --lib mod_content::tests:: -- --ignored   # the slow content-root tests, #[ignore]d; pr_ready runs them when mod content, assets, blocks or items change
cargo build --profile fast -p horizon-client        # optimized, quick to link: use it to play or run lan_e2e
python3 tools/parity.py --changed origin/main       # the harnesses your branch can affect (CI's parity job)
python3 tools/parity.py --changed-files paths.txt --list   # which harnesses cover these files, without running them
python3 tools/parity.py mob_ai sculk       # named harnesses
python3 tools/lan_e2e.py --check chest     # two real games over LAN (server-thread host)
python3 tools/agent_control_e2e.py         # horizon-ctl / MCP end to end
python3 tools/lang_coverage.py --summary   # the keys each language still misses, and its English leftovers
```

- Harnesses and the game need the vanilla data in `.local/` (`minecraft/`, `horizon-client/26.3`, `datagen/`,
  `tools/`): see [docs/parity-workflow.md](docs/parity-workflow.md). Without it, pr_ready lists the
  harnesses for CI.
- A harness = `tools/test_<name>_parity.py` + a Java oracle (`mods/horizon/src/test/java/dev/horizonlab/parity/Vanilla*Oracle.java`)
  + a Rust probe: `crates/<crate>/examples/<name>_{probe,fixture}.rs` (mostly horizon-world and horizon-client),
  or the game binary with a fixture env var (`game/dispense_fixture.rs`, `game/sculk_fixture.rs`,
  `game/command_harness.rs`, `game/net_harness.rs`). Copy a nearby harness for a new one.
- Developing a native mod? Run `horizon-launcher settings native-profile dev` once. `dev` builds the game crate
  unoptimized: a mod edit rebuilds about 6× faster than with `performance`, while game logic runs up to about twice as
  slow. Use it while developing, not for playing; `performance` stays the default for players and releases
  ([docs/native-mods.md](docs/native-mods.md)).
- Probes build into `.local/parity-target/`; harnesses find binaries via `lab.cargo_target()`, never `target/`.
- Harness selection uses Cargo dep-info and reachable-code narrowing (`tools/parity_reach.py`).
  `horizon-client-lib` builds the control tool and client probes without gameplay; their dep-info
  excludes `game.rs` and `game/`, so gameplay edits do not rebuild or select those probes.
  Source paths remain in `crates/horizon-client/`. Measure actual harness selection on refreshed
  records with `tools/parity.py --changed-files paths.txt --list`; historical counts are not a guarantee.
- Rust integration tests: `crates/horizon-client/tests/` (`lan_bot.rs`, `generation_saves.rs`, `streaming_meshes.rs`).
- Linux needs `libasound2-dev libudev-dev`; `--no-default-features` drops audio, controllers, `discord`, `friends` and `heartbeat` (the server build).
- Run and drive the game (starts the `horizon-client` beside `horizon-ctl`; other arguments go to the game):
  `target/fast/horizon-ctl --socket $PWD/ctl.sock launch --world W --creative --offscreen --wait --resources $PWD/.local/horizon-client/26.3`,
  then `horizon-ctl --socket … state|keys|turn|command|screenshot|quit`
  ([docs/horizon-ctl-commands.md](docs/horizon-ctl-commands.md)).

## Vanilla reference

- Start parity work from the port ledger, not grep: `python3 tools/port_ledger.py issue N` (or `where
  Class.method`, `what file:line`), and `check --base origin/main` before the PR. How:
  [.claude/skills/vanilla-parity/SKILL.md](.claude/skills/vanilla-parity/SKILL.md), [docs/port-ledger.md](docs/port-ledger.md).
- Decompiled sources: `.local/minecraft/26.3/sources/{client,server}/` (`python3 tools/lab.py decompile`).
- Bytecode queries (callers through lambdas, every overload):
  `python3 tools/rea_vanilla.py method net.minecraft.world.level.block.DispenserBlock dispenseFrom`
  ([docs/decompilation-status.md](docs/decompilation-status.md)).
- A harness runs the oracle against the real 26.3 jars and the Rust probe on the same inputs, then compares
  their JSON. Oracle runs are cached by `tools/parity_shim.py`; `--fresh` reruns them.
- Mirror vanilla's `Math` calls with `horizon_core::jmath`, and vanilla's RNG with `horizon_core::random`:
  random draws must happen in vanilla's order.

## horizon-client: where things live

`L:` = lib (`crates/horizon-client/src/`, or `crates/horizon-client-render/src/` for the modules that crate holds, under
the same `horizon_client::<module>` paths), `G:` = gameplay (`crates/horizon-client/src/game/`), also in the lib in normal builds.
Gameplay keeps its `horizon_client::<module>` imports through a self-crate alias. The executable
contains only the allocator, mod API marker and a call to `entry::main`. For unit tests, the lib
excludes `entry` and `game`, and the executable test target owns `game` so its process-wide test
state stays separate from the library tests. This compiles gameplay twice for executable tests.

| Subsystem | Files |
|---|---|
| Game loop, tick, frame | `game.rs`: `fn run`, `fn tick`, `fn tick_world`, `fn render`; G: `window_handler.rs` (`impl ApplicationHandler for Game`: `window_event`, `about_to_wait`); L: `clock.rs`, `spans.rs`; `horizon-client-input/src/window.rs` |
| Rendering | L: `gfx.rs` (`Renderer::new`, `Renderer::render`), `gfx/` (cull, occlusion, translucency, upload, timing, lod, weather, stars, end_sky), `*.wgsl`, `particles.rs`, `block_entity_render.rs`, `item_render.rs`, `fog.rs`, `clouds.rs`, `camera.rs`, `first_person.rs`; G: `render_tick.rs`, `item_draw.rs`, `screen_backgrounds.rs` (panorama, menu blur, border wall) |
| Chunks, streaming, light | L: `streaming.rs` (worker pool: generate, light, mesh), `world.rs`, `chunk.rs`, `chunk_map.rs`, `light.rs`, `mesher.rs` + `mesher/`, `vanillagen.rs` (binds horizon-world's generator), `lod.rs` + `lod/`, `server_terrain.rs` (a server's kept chunks), `bluemap.rs` + `bluemap_detect.rs` (far terrain from its web map), `ticket_levels.rs`; G: `world_loading.rs`, `world_entry.rs`, `postprocess.rs`; game.rs `fn update_world` |
| World generation | `crates/horizon-world/src/worldgen/` (`generator*.rs`, `feature_graph.rs`, `structure/`, `carver/`) |
| Block behaviour | `crates/horizon-world/src/level/` (`mod.rs`, `tick.rs`, `hooks.rs`, `redstone.rs`, `place.rs`, `interact.rs`); game.rs `fn tick_blocks`, `fn process_level_events` |
| Mobs | L: `mobs.rs` (types), `mobs/` (`ai.rs`, `goals.rs`, `brain.rs`, `*_brain.rs`, `nav.rs`, `path.rs`, `species.rs`, `spawning.rs`, `render.rs`, `anim.rs`); G: `creatures.rs` (ticking, spawning, mob events), one file per mob family (`dragon.rs`, `wither.rs`, `villagers.rs`, `piglins.rs`, `golems.rs`...), `natural_spawning.rs` |
| Other entities | L: `entities.rs`, `projectiles.rs`, `arrows.rs`; G: `boats.rs`, `minecarts.rs`, `mounts.rs`, `armor_stands.rs`, `paintings.rs`, `frames.rs`, `fireworks.rs`, `tnt.rs`, `explosion.rs` |
| Player actions | game.rs (`fn use_hand`, `fn tick_mining`, `fn break_block`); G: `item_actions.rs` (`fn use_item`), `melee.rs` (`fn attack`), `hand.rs`, `combat.rs`, `consume.rs`, `using_item.rs` (an item in use, tick by tick), `potions.rs`, `shield.rs`, `crossbow.rs`, `trident.rs`, `fishing.rs`; L: `motion.rs` + `motion/` (movement), `collisions.rs`; crate `horizon-items`: `items.rs`, `item_data.rs`, `enchant.rs`, `blocks.rs` |
| Inventory and containers | L: `menu.rs` (`AbstractContainerMenu.clicked`), `container_screen.rs`, `containers.rs`, `creative.rs`, `crafting.rs`, `recipe_book*.rs`, `stations.rs`, `merchant.rs`; game.rs `fn menu_ops`, `fn click_inventory`; G: `stations.rs`, `trading.rs`, `recipe_book.rs`, `machines.rs`, `inventory_input.rs` (keys and clicks over a container), `inventory_slots.rs` (slot layout and drawing) |
| Commands | parse: crate `horizon-command` (`args.rs`, `selector.rs`, `tree_*.rs`, `vanilla.rs`); run: G: `commands.rs` (dispatch) + `commands/*.rs` (`admin`, `execute`, `items`, `players`, `world_cmds`...); L: `scoreboard_cmd.rs`, `data_cmd.rs`; `horizon-client-core`: `nbt_path.rs`, `scoreboard.rs`; G: `functions.rs`, `command_blocks.rs` |
| UI screens | `Screen` enum in game.rs; L: `title.rs` + `title/`, `options_ui.rs`, `multiplayer.rs`, `pack_screen.rs`, `settings_shell.rs` (Horizon's settings screens on it: `extras_ui.rs`, `graphics_ui.rs`, `controller_ui.rs`), `screen_draw.rs`, `ui.rs` (widgets), `hud.rs`, `hud_layout.rs`, `hud_overlays.rs`, `hud_kit/` (Horizon overlays' `Room` and `Plate`, the collapsed page strip `tabs` that Shaders' screens use, [docs/ui-kit.md](docs/ui-kit.md)), `pad_glyph.rs` (controller glyphs, by what the button does), `chat.rs`, `tab_list.rs`; G: `startup_menu.rs`, `loading_screen.rs` (the loading screen, the pre-game `App`), `screens.rs` (pause, death), `options.rs`, `lan_screen.rs`, `chat.rs`, `sign_edit.rs`, `books.rs`, `creative_screen.rs` |
| Multiplayer client | L: `net.rs` (network thread); G: `remote.rs` (`fn handle_packet`, `fn tick_remote`), `remote_*.rs` (mob data, objects, projectiles, vehicles, stations) |
| LAN host / server side | L: `lan/` (`conn.rs` handshake + login threads, `hub.rs` + `hub/` play state, `chunks.rs`, `entity.rs`, `tracking.rs`, `guest.rs`, `playerdata.rs`); G: `lan.rs` (`LanHost`), `lan_actions.rs` (`fn as_guest`, `fn tick_guests`), `lan_*.rs` per feature, `lan_entities.rs` (tracked entities) |
| Integrated server | G: `integrated.rs` (the server thread), `levels.rs` (one level per dimension, `fn tick_server`), `server_travel.rs` |
| Dedicated server | G: `dedicated.rs`, `dedicated/status.rs`; crates `horizon-server-files`, `horizon-server`, `horizon-anticheat` (L: `lan/anticheat.rs`) |
| Save and load | L: `save.rs` (level.dat, regions, playerdata), `entity_nbt.rs`, `options_file.rs`; `horizon-client-core/src/block_entity_nbt.rs`; G: `saving.rs`, `persist.rs` (entities), `world_session.rs` (opening and closing a world); `crates/horizon-world/src/{region.rs,nbt.rs,datafix/}` |
| Input | game.rs `fn window_event`; G: `play_keys.rs` (`fn key_pressed`), `play_input.rs` (`fn play_mouse`, toggled keys, the hotbar wheel); crate `horizon-client-input` (`keybinds.rs`, `keymapping.rs`, `key_names.rs`, `mouse.rs`, `input.rs`, `interaction_input.rs`, `gamepad.rs`, `clipboard.rs`); L: `osk.rs`; G: `pad.rs`, `pad_menu.rs`, `pad_craft.rs`, `pointer_capture.rs` (mouse look, pointer grab) |
| Audio | L: `audio.rs` (mixer), `sound_engine.rs`, `sound_events.rs`, `music.rs`, `ambience.rs`; G: `music.rs`, `subtitle_hud.rs` |
| Resource packs | L: `packs.rs` + `packs/`, `pack_screen.rs`, `atlas_sources.rs`, `sprites.rs`, `mipmap.rs`, `model_library.rs`, `font.rs`, `mobs/cem.rs` + `mobs/cem_anim.rs` (OptiFine/EMF entity models and their animations); `horizon-client-core/src/lang.rs`; crates `horizon-assets`, `horizon-optifine`; G: `packs.rs` |
| Agent control | L: `control.rs` (socket), `bin/horizon-ctl/`; G: `control.rs`, `agent.rs`, `events.rs`, `ui_control.rs` ([docs/agent-control.md](docs/agent-control.md)) |
| Mods | crate `horizon-mods`; G: `mods.rs`; L: `mod_content.rs`, `mod_list.rs` |
| Horizon extras, settings | L: `extras.rs`, `graphics.rs`, `settings.rs`, `minimap.rs`; G: `extras.rs`, `tweaks.rs`, `options.rs` |

## How `Game` is organised

- `struct Game` (game.rs) holds everything. Its methods are spread over many files: game.rs keeps
  the core (tick, render, input, inventory, world open/close) and each `game/<feature>.rs` adds an
  `impl Game<'_>` block for one feature, with its own state types. Fields stay private to the gameplay module and its descendants. Gameplay tests run in the gameplay library, separate from reusable-library tests.
- Find a feature's code: `grep -rn "fn tick_<thing>\|fn <verb>_<thing>" crates/horizon-client/src/game*`;
  the file is usually `game/<thing>s.rs`. Who calls it: grep the name in game.rs's `fn tick` / `fn tick_world`.
- Fields that say which mode runs: `remote` (this window is a client of a server: multiplayer, LAN guest, or
  single player on its server thread), `lan` (`LanHost`: this world is open to the network), `integrated`
  (the window's own server thread), `serving` (this `Game` *is* that server thread), `options.dedicated`.
  `fn tick` returns `tick_remote()` when `remote` is set; a server ticks its levels (`fn tick_world`).
- A server's players run through the shared gameplay code: `as_guest` swaps the guest's
  state (position, inventory, mode, mining, item use) into `Game` for one call (`game/lan_actions.rs`), and
  `levels.rs` swaps whole dimensions in the same way. Changes reach clients through the block log, level
  events and the entity tracker (`lan/tracking.rs`, `game/lan_entities.rs`).
- Shared read-only data: `Resources` (`res`: blocks, items, recipes, mob types, biomes) and `PackResources`
  (`pack`), both in game.rs, their methods in `game/resource_tables.rs`.

## Single player runs on a server thread

`game/integrated.rs` runs a second `Game` (`serving`) on a "Server thread" with one level per dimension;
the window joins it as a client through memory (`remote`), exactly as a LAN guest or a dedicated server's
player does (#1514). A window's `Game` never ticks a world itself: the old in-frame path is gone
(`--in-frame` and `--no-server-thread` only print a notice; `HORIZON_SERVER_THREAD` is ignored).
Open `[server-thread]` issues list the gaps left.

Single player, LAN guests and dedicated servers all run the server code, so a fix there serves all three.
Test the server thread when you touch player-facing gameplay:

```sh
cargo run --profile fast -p horizon-client -- --resources .local/horizon-client/26.3 --world T               # server thread
python3 tools/lan_e2e.py --check <name>                  # host on its server thread, guest is a server client
python3 tools/parity.py dedicated_server server_entities server_player_damage
```

## Docs worth knowing

[docs/parity-workflow.md](docs/parity-workflow.md) (runner, gate, shards, scopes) ·
[docs/native-client.md](docs/native-client.md) (options, controls, horizon-ctl) ·
[docs/server-levels.md](docs/server-levels.md) (levels per dimension) ·
[docs/rust-parity.md](docs/rust-parity.md) (what is ported) · [docs/known-issues.md](docs/known-issues.md) ·
[docs/release-flow.md](docs/release-flow.md) · [docs/README.md](docs/README.md) (index).

# Horizon in the browser (the `web` port)

The `web` branch ports the client to the browser: `horizon-client` compiled for `wasm32-unknown-unknown`,
drawing through WebGPU, with threads as Web Workers. It boots to the title screen, opens single-player worlds
and joins offline-mode Horizon and Eaglercraft servers over a WebSocket, with sound and controllers. Options and
saves are kept in the browser's OPFS; a service worker keeps the large files. Design, status and what's missing:
[web/README.md](web/README.md). Everything above still applies; this section adds what the port needs.
(`ROADMAP.md` is the whole project's roadmap, not the port's.)

## Port rules that bite

- **Native behaviour must not change.** Every web difference sits behind
  `#[cfg(target_arch = "wasm32")]` (or `cfg!`), or in `horizon-os`, which is std's own on native. Upstream
  merges stay small: don't reformat or restructure code beyond what the port needs.
- **Go through `horizon_os`, not std**, in the client's crates (std's versions panic or fail in the browser):

  | Instead of | Use |
  |---|---|
  | `std::fs::*` | `horizon_os::fs::*` |
  | `path.exists()`, `.is_dir()`, `.is_file()`, `.metadata()`, `.canonicalize()` on a `Path` | `path.fs_exists()`... (`use horizon_os::fs::PathExt;`) |
  | `std::time::{Instant, SystemTime, UNIX_EPOCH}` | `horizon_os::time::*`; `horizon_os::time::to_std` for chrono |
  | `std::thread::*` | `horizon_os::thread::*` |
  | `rx.recv_timeout(t)`, `condvar.wait_timeout_while(g, t, f)`, `condvar.wait_while(g, f)` | `horizon_os::sync::recv_timeout(&rx, t)`, `horizon_os::sync::wait_timeout_while(&mutex, &condvar, g, t, f)`, `horizon_os::sync::wait_while(&mutex, &condvar, g, f)` (std's read std's clock, which panics on the web; plain `wait_timeout`, `park_timeout` and `sleep` are fine on workers) |
  | `std::process::id()` | `horizon_os::process::id()` |
  | `std::env::{args, current_dir, current_exe, temp_dir}` | `horizon_os::env::*` (the page's `?args=` on the web) |

  `Metadata`/`DirEntry`/`FileType` keep std's own `.is_dir()`, `.metadata()`... (they aren't paths).
  `horizon-ctl` (`src/bin/horizon-ctl/`) is native-only and keeps std.
- **The page's thread never blocks.** No `pollster::block_on` on something that waits for the browser
  (use `horizon_os::block_on`, which only takes a ready future on the web), no `Condvar` waits,
  `recv()` on an empty channel or `thread::sleep` loops: browsers trap a wait on the page's thread. Workers
  may block (`horizon_os::workers::in_worker()` tells them apart).
- **A lock the page shares with workers goes through `horizon_os::sync::lock` / `write`**, which spin on the
  page. std's `Mutex` waits once contended, the browser traps it (`Atomics.wait cannot be called in this
  context`), winit swallows the error and the frame loop stops with no message. A world that freezes silently:
  find the trap with CDP `Debugger.setPauseOnExceptions` (`state: "all"`, armed after startup; skip winit's
  `AbortError`s). A reply the page needs from a running worker: `horizon_os::sync::recv_reply`.
- **Threads are Web Workers** on shared memory: `horizon_os::thread::spawn` and `Builder::spawn` start one
  ([crates/horizon-os/src/workers.rs](crates/horizon-os/src/workers.rs)). A worker starts only once the
  page's thread returns to the browser, so the page must never wait for one it just spawned (`join` on the
  page spins; scoped threads run in sequence on the page, `std::thread::scope` panics there). There is no
  build without threads: `horizon-os` refuses to compile for wasm without `+atomics`, so the page must be
  cross-origin isolated (`web/serve.py` sends the headers; on a static host `web/sw.js` adds them). Keep the
  fallbacks for a spawn the browser refuses, as the job pool's `pump` (`horizon_threads::jobs`) does.
- **Shaders must pass browsers' WGSL rules**, which naga doesn't enforce: derivative uniformity (sample
  textures in uniform control flow, or start the module with `diagnostic(off, derivative_uniformity);`
  as `hud.wgsl` and `entity_shader()` do) and parentheses when mixing `<<`/`>>` with `|`/`&`/`^`.
- **On the web, `run_app` never returns.** Errors that native reports after the event loop are logged in
  `App::exiting` instead.
- Never commit `web/dist/` (it holds the prepared resources, Mojang's files). It is gitignored.

## Build and check

```sh
web/build.sh                       # the browser build into web/dist (release: LTO + wasm-opt; PROFILE=fast to iterate)
python3 web/serve.py               # http://localhost:8080, cross-origin isolated
# the browser build's check (the flags web/build.sh sets)
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS='--cfg getrandom_backend="wasm_js" -C target-feature=+atomics,+bulk-memory,+mutable-globals' \
  cargo +nightly-2026-10-14 -Z build-std=std,panic_abort clippy --target wasm32-unknown-unknown -p horizon-client --bin horizon-client --no-default-features --features audio,controllers -- -D warnings
cargo clippy --workspace --all-targets -- -D warnings      # native must stay clean too
```

- The web build keeps `controllers` (gilrs reads the Gamepad API); `horizon-dualsense` builds without
  hidapi there, as on iOS. `.cargo/config.toml` sets getrandom's
  `wasm_js` backend for wasm only, so don't export it in `RUSTFLAGS` (it breaks native builds).
- The browser build needs the pinned nightly with `rust-src` (`rustup component add rust-src --toolchain
  nightly-2026-10-14`): std is rebuilt with atomics, and the module imports a shared memory the page hands
  each worker (`web/worker.js`).
- After a change, check that **both** the native client and the page reach the title screen
  (`Horizon: first frame after … ms` in the log or the browser console).
- `web/bundle.py` decides what is downloaded up front (`boot.tar.gz`) and what is fetched when read
  (`manifest.txt`). A file read at startup that is only in the manifest costs a blocking request.

## Testing in a browser

- Needs WebGPU. Chromium-based browsers on Linux need it switched on: `--enable-unsafe-webgpu
  --enable-features=Vulkan`. Headless works (`--headless=new --remote-debugging-port=…` with a scratch
  `--user-data-dir`) and can be driven over the DevTools protocol; read WebGPU validation errors from its
  `Log` domain, they aren't console messages.
- An adapter-less browser stops at `No available adapters`: that's the browser, not the game.
- The game logs to the browser console; a panic logs `Horizon panicked: …` with the Rust message. A worker's
  console is its own DevTools target: attach to it (`Target.setAutoAttach` with `flatten`) to see its logs.
  A panic aborts only that worker (`RuntimeError: unreachable`); its `join` then never returns.
