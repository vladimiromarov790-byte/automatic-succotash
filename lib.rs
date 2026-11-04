//! SDK glue, written against the REAL WIT contract (horizon.wit, package horizon:mods@0.1.0; the
//! ElyHorizon fork's crates/rustcraft-sdk/wit/horizon.wit is the same contract under another name).
//! Verified from the WIT: get-player, raycast, input.mapping/held, command.run-quiet, chat.show, events.
//! NOT verified (no compiler here): the Rust SDK wrapper's exact fn/type names, marked `SDK?`.
mod logic;
use logic::{Clones, Hit, Mode, Snap};

use horizon_mod_sdk::{chat, command, game, input, Mod, Registration}; // SDK? crate/module names

struct PlayerClones { c: Clones }

impl Mod for PlayerClones {
    fn init() -> Result<(Self, Registration), String> {
        // SDK? builder names: the README example shows `Registration::new().on_tick()`.
        let reg = Registration::new().command("clones").on_tick();
        Ok((Self { c: Clones::new() }, reg))
    }

    fn command(&mut self, _name: &str, args: &str) -> Result<Option<String>, String> {
        let a: Vec<&str> = args.split_whitespace().collect();
        let Some(p) = self.snapshot() else { return Err("not in a world".into()) };
        let out = match a.as_slice() {
            ["spawn"] => self.c.spawn(2, &p),
            ["spawn", n] => self.c.spawn(n.parse().unwrap_or(2), &p),
            ["mode", "follow"] => { self.c.mode = Mode::Follow; vec![] }
            ["mode", "echo"] => { self.c.mode = Mode::Echo; vec![] }
            ["actions", v] => { self.c.copy_actions = *v == "on"; vec![] }
            ["name", n] => { self.c.name = n.to_string(); vec![] }
            ["clear"] => self.c.clear(),
            _ => return Ok(Some("/clones spawn [n] | mode follow|echo | actions on|off | name <nick> | clear".into())),
        };
        let sent = out.len();
        self.send(out);
        Ok(Some(format!("clones: {} command(s) sent, {} clone(s) active", sent, self.c.n)))
    }

    fn tick(&mut self, _tick: u64) {
        if let Some(p) = self.snapshot() {
            let cmds = self.c.tick(&p);
            self.send(cmds);
        }
    }
}

impl PlayerClones {
    fn snapshot(&self) -> Option<Snap> {
        let pl = game::get_player()?;                 // WIT: option<player{x,y,z,yaw,pitch,..}>
        let down = |name: &str| input::mapping(name).map(|m| m.down).unwrap_or(false); // WIT: option<mapping-state>
        let looking_at = game::raycast(6.0).map(|h| Hit { // WIT: option<hit{block-x.. x,y,z, block}>
            bx: h.block_x, by: h.block_y, bz: h.block_z, x: h.x, y: h.y, z: h.z, block: h.block });
        Some(Snap {
            x: pl.x, y: pl.y, z: pl.z, yaw: pl.yaw, pitch: pl.pitch,
            sneaking: down("key.sneak"),
            held: input::held().map(|h| h.id).unwrap_or_default(), // WIT: option<held-item{id,count,slot}>
            looking_at, attack: down("key.attack"), use_: down("key.use"),
        })
    }

    /// `command.run-quiet`: no chat feedback, runs with the player's rights (single player: cheats on),
    /// at most 64 a tick, 256 a call. `@s` in our commands is therefore the player.
    fn send(&self, cmds: Vec<String>) {
        if !cmds.is_empty() { command::run_quiet(&cmds); }   // SDK? may take &[String] or Vec<String>
    }
}

horizon_mod_sdk::export_mod!(PlayerClones);                // SDK? macro path
#[allow(unused)] fn _chat_is_declared_capability() { let _ = chat::show; }
