This repo is a Rust -> WebAssembly component mod for the Horizon game (Minecraft 26.3 in Rust, Android port).
Read HANDOFF.md and COPILOT_TASK.md before changing anything. Keep src/logic.rs free of SDK types and covered by
unit tests; only src/lib.rs talks to the Horizon mod SDK. Never invent SDK functions: read the SDK sources in
horizon-rust/horizon. The deliverable is player_clones.hmod built by .github/workflows/build.yml.
