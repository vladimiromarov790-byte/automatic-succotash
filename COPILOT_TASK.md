# Task for GitHub Copilot (coding agent): build the "Player Clones" Horizon mod into a .hmod

## Context
This repo (or an uploaded `player-clones-source.zip`) holds a Rust source project for a SANDBOXED WEBASSEMBLY MOD
for Horizon (Minecraft 26.3 rewritten in Rust; Android port Vasysik/horizon-android 0.7.7, upstream
horizon-rust/horizon). Mod API 0.1 (package `horizon:mods@0.1.0`). A `.hmod` is a zip with `horizon-mod.toml`
and `mod.wasm` (a wasmtime component-model component). The mod spawns `minecraft:mannequin` clones of the player
via vanilla commands (`command.run-quiet`) that follow the player, echo it with a delay, and repeat its mining
and building. Read HANDOFF.md first: it lists exactly what is verified and what is guessed.

## Step 0: layout
If `player-clones-source.zip` is in the repo root: unzip it, move the contents of the `clones/` folder to the
repo root (so `Cargo.toml`, `horizon-mod.toml`, `src/`, `.github/workflows/build.yml`, `HANDOFF.md` sit at the
root, keeping the hidden `.github` folder), delete the zip, commit.

## Step 1: find the real SDK and fix `src/lib.rs`
Lines marked `SDK?` in `src/lib.rs` are guesses at the Rust SDK wrapper. Read the real sources in
https://github.com/horizon-rust/horizon (look for `crates/horizon-mod-sdk` or `crates/*-sdk`, its `wit/horizon.wit`,
`examples/`, `docs/mods.md`, and the `mods/` directory with example mods such as coords-hud / crosshair-hud) and
make the crate compile against it: Cargo dependency (git, pinned to a commit), `Mod` trait signature,
`Registration` builder (`commands`, `tick`), `game::get_player`, `game::raycast`, `input::mapping`,
`input::held`, `command::run_quiet`, `export_mod!`. Keep `src/logic.rs` SDK-free.
Match the manifest to the real schema (`crates/horizon-mods` manifest parser or `docs/mods.md`):
capabilities `game-read`, `chat`, `command`, `input`; `input` is experimental (`experimental = ["input"]`).

## Step 2: build
Use the project's own tool (`cargo-horizon-mod` or `cargo xtask mod build`, whichever exists; see docs/mods.md) to
produce `player_clones.hmod`, and run its validator/check if it has one. `cargo test --lib` must pass
(unit tests in `src/logic.rs`). Fix the workflow `.github/workflows/build.yml` if the tool name or install
command differs. If an exact command is not documented, read the tool's source.

## Step 3: publish
Make the workflow green on push and on manual dispatch. On a `v*` tag it should create a GitHub Release with
`player_clones.hmod` attached (so it can be downloaded on a phone). Create the first release `v0.1.0`.

## Step 4: report
Comment with the direct download link of the `.hmod` and a list of anything that still could not be verified.

## Constraints and fallbacks
- Do not invent API: if a function in the guessed list does not exist in the real SDK, adapt the design.
- If `input` or `raycast` is not in the API level of the target build, make `input` optional behind a cargo
  feature and keep the `/clones spawn|mode|clear` commands working without mining/building mirroring.
- Clones use `summon minecraft:mannequin ...` NBT (`profile`, `Invulnerable`, `NoGravity`); if the 26.3 data
  format differs, fix it in `Clones::spawn`. Keep command output under 64 lines a tick.
- Do not commit build artifacts other than via Releases.
