# Player Clones — handoff for the next Claude

## Goal (from the user)
A "global" Horizon mod (Android port Vasysik/horizon-android 0.7.7, Minecraft 26.3 in Rust): CLONES OF THE PLAYER
that do what the player does. Not another content/ruby mod: the previous suite (rubycraft_*) was data-only.

## Why this is a sandboxed WASM mod, not a content mod
- Content `.hmod` (api 0.2, `registries`) = data only, cannot run behaviour.
- Native `.hnmod` can't be compiled on Android (port README).
- Android 0.7.7 runs SANDBOXED CODE `.hmod`: a zip of `horizon-mod.toml` + `mod.wasm`, wasmtime component model,
  WIT package `horizon:mods@0.1.0`, 64-bit APKs only. Capabilities seen in upstream/fork PRs: `game-read`
  (player, clock, `raycast`), `chat`, `hud`, `input` (keybinds, `input.poll`), `ui`, `render`, `world`
  (raycast, run commands at operator level silently in one tick, sounds), `storage`. Upstream `main` docs moved
  to native mods, so the 0.1 API is only documented in PRs #327, #328, #2113 and the fork Stink-O/horizon.

## Design
Clones are `minecraft:mannequin` entities (player-model entity) driven ONLY by vanilla commands via `world`:
- `Follow`: arc of up to 8 clones behind you; each tick `tp`; copies sneak pose, held item
  (`item replace ... from entity @s`), and REPEATS YOUR MINING/BUILDING shifted by its offset
  (`setblock ... air destroy`, `setblock ... <held block>` on the face you hit).
- `Echo`: clone i replays you 2*(i+1) s later (60 s ring buffer).
- Commands: `/clones spawn [n] | mode follow|echo | actions on|off | name <nick> | clear`.
`src/logic.rs` is pure Rust (no SDK) with unit tests; `src/lib.rs` is the only SDK glue.

## UPDATE v0.2 (this session): contract now READ, not guessed
Real WIT found at https://github.com/Sk0vorodka/ElyHorizon/blob/feat/elyby/crates/rustcraft-sdk/wit/horizon.wit
(fork of Horizon, package renamed rustcraft:mods; identical shape to horizon:mods@0.1.0). Facts used:
- `game.get-player` (x,y,z,yaw,pitch,health,food,dimension), `game.raycast(distance)`.
- NO `world` capability on clients (that interface is server mods, api 0.3). Commands go through capability
  `command`: `run` (8/call, feedback in chat) and `run-quiet` (256/call, 64/tick, silent, with the player's rights).
- `input` is EXPERIMENTAL: manifest needs `experimental = ["input"]`. It gives `held()`, `mapping("key.sneak"|
  "key.attack"|"key.use")` -> down, `poll()` events. There is no break/place event: logic.rs infers them
  (`detect_break`: attack held + view stable + aimed block vanished; `detect_place`: use pressed on a block).
- Registration = {commands, tick, frame, chat}; command hook returns result<option<string>, string>.
- Rust SDK README example (`Mod`, `Registration::new().on_tick()`, `game::get_player()`, `export_mod!`) is in
  the ElyHorizon README. Remaining `SDK?` marks in src/lib.rs are wrapper names only.
- Android 0.7.7 may predate `input`/`raycast` (API level 2): if the mod is refused with "update Horizon",
  drop `input` and fall back to a `/clones` command-only version (no mirroring of mining).

## STATE: NOT COMPILED, NOT RUN
The sandbox had no cargo/rustc and no network. Nothing was built or tested. Do these in order:
1. Push this folder to a GitHub repo, run the Actions workflow (`.github/workflows/build.yml`): it runs the
   logic tests, builds with `cargo-horizon-mod`, and uploads `player_clones.hmod`. (User is on a phone.)
2. Pin `horizon-mod-sdk` in Cargo.toml to a commit matching Android 0.7.7's upstream sync
   (see docs/android-upstream.json in Vasysik/horizon-android, branch `android`).
3. Fix `src/lib.rs` against the REAL SDK (`crates/horizon-mod-sdk/src/lib.rs`, `wit/horizon.wit`): names of
   `Mod`, `Registration`, `game::player`, `game::raycast`, `world::run_command`, `input::*`, `export_mod!`
   are guesses. Also `capabilities` in horizon-mod.toml (undeclared imports are refused) and API level
   (`raycast` is API level 2: Android build must be new enough, else "update Horizon").
4. In-game checks (single player only: `world` commands need operator on the integrated server):
   - `/summon minecraft:mannequin` NBT in `spawn()` (`profile`, `Invulnerable`, `NoGravity`) vs 26.3 format.
     Fallback if mannequin is missing/odd: armor_stand with player head.
   - `Pose:"crouching"` via `data merge` may be ignored; drop it if so.
   - Break/place detection is a heuristic (attack released / use pressed on last aimed block). Replace by real
     `input.poll` events if the SDK offers them, and detect a real break by comparing the block before/after.
   - Command budget: `MAX_CMDS_PER_TICK = 14` (tick fuel 5e7). Tune if the mod gets stopped for fuel.
5. Ideas after it works: clones with the player's real skin (profile of the logged-in name), swing animation,
   clones that mine toward a target (pathing), `/clones task mine|follow|guard`, HUD panel with status.
