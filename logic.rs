//! Pure logic: no SDK types, so it can be unit-tested with plain `cargo test`.
//! It turns "what the player does" into vanilla command strings that glue (lib.rs) sends through
//! the `world` capability (operator-level, silent, one tick).
use std::collections::VecDeque;

pub const MAX_CMDS_PER_TICK: usize = 14; // tick fuel is 5e7: keep the output small
pub const MAX_CLONES: usize = 8;
const HISTORY_TICKS: usize = 20 * 60; // echo can look back up to 60 s

#[derive(Clone, Debug, PartialEq)]
pub struct Hit { pub bx: i32, pub by: i32, pub bz: i32, pub x: f64, pub y: f64, pub z: f64, pub block: String }

/// One tick of the player, filled by glue from game.player / game.raycast / input.poll.
#[derive(Clone, Debug, Default)]
pub struct Snap {
    pub x: f64, pub y: f64, pub z: f64, pub yaw: f32, pub pitch: f32,
    pub sneaking: bool,
    pub held: String,           // "minecraft:cobblestone", "" for empty hand
    pub looking_at: Option<Hit>,
    pub attack: bool,           // input.mapping("key.attack").down
    pub use_: bool,             // input.mapping("key.use").down
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mode { Follow, Echo }

pub struct Clones {
    pub n: usize,
    pub mode: Mode,
    pub copy_actions: bool,
    pub name: String,
    history: VecDeque<Snap>,
    last_held: Vec<String>,
    last_sneak: Vec<bool>,
    prev: Option<Snap>,
}

impl Clones {
    pub fn new() -> Self {
        Self { n: 0, mode: Mode::Follow, copy_actions: true, name: "Steve".into(),
               history: VecDeque::new(), last_held: vec![], last_sneak: vec![], prev: None }
    }

    fn sel(i: usize) -> String { format!("@e[tag=hclone_{i},limit=1]") }

    pub fn spawn(&mut self, count: usize, p: &Snap) -> Vec<String> {
        let mut out = vec![self.clear()[0].clone()];
        self.n = count.clamp(0, MAX_CLONES);
        self.last_held = vec![String::new(); self.n];
        self.last_sneak = vec![false; self.n];
        for i in 0..self.n {
            let (x, z, _) = self.slot_pos(i, p);
            // VERIFY: mannequin NBT (profile / immovable) against the 26.3 data format
            out.push(format!(
                "summon minecraft:mannequin {x:.2} {:.2} {z:.2} {{Tags:[\"hclone\",\"hclone_{i}\"],\
                 profile:{{name:\"{}\"}},Invulnerable:1b,NoGravity:1b}}", p.y, self.name));
        }
        out
    }

    pub fn clear(&mut self) -> Vec<String> {
        self.n = 0; self.history.clear();
        vec!["kill @e[tag=hclone]".to_string()]
    }

    /// Where clone `i` stands in Follow mode: an arc behind the player.
    fn slot_pos(&self, i: usize, p: &Snap) -> (f64, f64, f32) {
        let spread = 40.0_f64; // degrees between neighbours
        let mid = (self.n.max(1) as f64 - 1.0) / 2.0;
        let a = (p.yaw as f64 + 180.0 + (i as f64 - mid) * spread).to_radians();
        // Minecraft: yaw 0 faces +Z, x = -sin(yaw), z = cos(yaw)
        (p.x - a.sin() * 2.0, p.z + a.cos() * 2.0, p.yaw)
    }

    /// Echo mode: clone i replays the player `2*(i+1)` seconds ago.
    fn echo_snap(&self, i: usize) -> Option<&Snap> {
        let back = 40 * (i + 1);
        if self.history.len() <= back { return None; }
        self.history.get(self.history.len() - 1 - back)
    }

    pub fn tick(&mut self, p: &Snap) -> Vec<String> {
        let mut out = Vec::new();
        if self.n == 0 { return out; }
        let (broke, placed) = match &self.prev {
            Some(pv) => (detect_break(pv, p), detect_place(pv, p)),
            None => (None, None),
        };
        self.prev = Some(p.clone());
        self.history.push_back(p.clone());
        if self.history.len() > HISTORY_TICKS { self.history.pop_front(); }

        for i in 0..self.n {
            // 1. where this clone is, and which snapshot drives its actions
            let (cx, cz, yaw, pitch, src): (f64, f64, f32, f32, Snap) = match self.mode {
                Mode::Follow => { let (x, z, y) = self.slot_pos(i, p); (x, z, y, p.pitch, p.clone()) }
                Mode::Echo => match self.echo_snap(i) {
                    Some(s) => (s.x, s.z, s.yaw, s.pitch, s.clone()),
                    None => continue, // not enough history yet
                },
            };
            let cy = if self.mode == Mode::Echo { src.y } else { p.y };
            out.push(format!("tp {} {cx:.2} {cy:.2} {cz:.2} {yaw:.1} {pitch:.1}", Self::sel(i)));

            // 2. sneaking pose (VERIFY: Pose is writable through data merge)
            if src.sneaking != self.last_sneak[i] {
                self.last_sneak[i] = src.sneaking;
                out.push(format!("data merge entity {} {{Pose:\"{}\"}}", Self::sel(i),
                                 if src.sneaking { "crouching" } else { "standing" }));
            }
            // 3. same item in hand: copied from the player (@s is the player: commands run as owner)
            if src.held != self.last_held[i] {
                self.last_held[i] = src.held.clone();
                out.push(format!("item replace entity {} weapon.mainhand from entity @s weapon.mainhand", Self::sel(i)));
            }
            // 4. mining and building, shifted by the clone's offset from the player
            if self.copy_actions && self.mode == Mode::Follow {
                let (dx, dz) = ((cx - p.x).round() as i32, (cz - p.z).round() as i32);
                if let Some(h) = &broke {
                    out.push(format!("setblock {} {} {} air destroy", h.bx + dx, h.by, h.bz + dz));
                }
                if let Some(h) = &placed {
                    if is_placeable(&src.held) {
                        let (fx, fy, fz) = face_of(h);
                        out.push(format!("setblock {} {} {} {}", h.bx + fx + dx, h.by + fy, h.bz + fz + dz, src.held));
                    }
                }
            }
        }
        out.truncate(MAX_CMDS_PER_TICK);
        out
    }
}

fn same_block(a: &Hit, b: &Hit) -> bool { (a.bx, a.by, a.bz) == (b.bx, b.by, b.bz) }
fn eye_dist(s: &Snap, h: &Hit) -> f64 {
    let (dx, dy, dz) = (h.x - s.x, h.y - (s.y + 1.62), h.z - s.z);
    (dx * dx + dy * dy + dz * dz).sqrt()
}
fn view_stable(a: &Snap, b: &Snap) -> bool {
    (a.x - b.x).abs() < 0.3 && (a.z - b.z).abs() < 0.3 && (a.yaw - b.yaw).abs() < 1.5 && (a.pitch - b.pitch).abs() < 1.5
}
/// The client has no "block broken" event, so a break is inferred: attack was held, the view did not move,
/// and the block we aimed at is gone (the ray now ends farther away or hits nothing).
pub fn detect_break(prev: &Snap, cur: &Snap) -> Option<Hit> {
    if !prev.attack || !view_stable(prev, cur) { return None; }
    let ph = prev.looking_at.as_ref()?;
    match &cur.looking_at {
        None => Some(ph.clone()),
        Some(ch) if same_block(ph, ch) => None,
        Some(ch) => if eye_dist(cur, ch) > eye_dist(prev, ph) + 0.4 { Some(ph.clone()) } else { None },
    }
}
/// Place = the use button went down while aiming at a block (also fires for doors/chests: acceptable).
pub fn detect_place(prev: &Snap, cur: &Snap) -> Option<Hit> {
    if cur.use_ && !prev.use_ { cur.looking_at.clone() } else { None }
}

/// Which side of the block was hit: the unit step from the block to the placed block.
pub fn face_of(h: &Hit) -> (i32, i32, i32) {
    let (rx, ry, rz) = (h.x - h.bx as f64, h.y - h.by as f64, h.z - h.bz as f64);
    let e = 1e-3;
    if rx < e { (-1, 0, 0) } else if rx > 1.0 - e { (1, 0, 0) }
    else if ry < e { (0, -1, 0) } else if ry > 1.0 - e { (0, 1, 0) }
    else if rz < e { (0, 0, -1) } else { (0, 0, 1) }
}

/// Cheap filter so tools/food do not become setblock noise. Unknown ids fail silently in-game.
pub fn is_placeable(id: &str) -> bool {
    !id.is_empty() && id != "minecraft:air"
        && !["_sword", "_pickaxe", "_axe", "_shovel", "_hoe", "_helmet", "_chestplate", "_leggings",
             "_boots", "bow", "apple", "bread", "stick", "bucket", "ingot", "_bomb"]
            .iter().any(|s| id.contains(s))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn snap() -> Snap { Snap { x: 10.0, y: 64.0, z: 10.0, held: "minecraft:stone".into(), ..Default::default() } }

    #[test] fn face_top_and_west() {
        let top = Hit { bx: 1, by: 2, bz: 3, x: 1.5, y: 3.0, z: 3.5, block: "x".into() };
        assert_eq!(face_of(&top), (0, 1, 0));
        let west = Hit { bx: 1, by: 2, bz: 3, x: 1.0, y: 2.5, z: 3.5, block: "x".into() };
        assert_eq!(face_of(&west), (-1, 0, 0));
    }
    #[test] fn spawn_makes_n_summons() {
        let mut c = Clones::new();
        let cmds = c.spawn(3, &snap());
        assert_eq!(cmds.iter().filter(|s| s.starts_with("summon")).count(), 3);
    }
    #[test] fn follow_copies_break_with_offset() {
        let mut c = Clones::new(); c.spawn(1, &snap());
        let dirt = Hit { bx: 10, by: 64, bz: 12, x: 10.5, y: 64.5, z: 12.0, block: "minecraft:dirt".into() };
        let mut a = snap(); a.attack = true; a.looking_at = Some(dirt);
        let mut b = snap(); b.attack = true; // block vanished: ray hits nothing
        c.tick(&a);
        let cmds = c.tick(&b);
        assert!(cmds.iter().any(|l| l.starts_with("setblock") && l.ends_with("air destroy")), "{cmds:?}");
    }
    #[test] fn break_needs_stable_view() {
        let dirt = Hit { bx: 10, by: 64, bz: 12, x: 10.5, y: 64.5, z: 12.0, block: "d".into() };
        let mut a = snap(); a.attack = true; a.looking_at = Some(dirt);
        let mut b = snap(); b.yaw = 30.0;
        assert!(detect_break(&a, &b).is_none());
    }
    #[test] fn place_on_use_edge() {
        let h = Hit { bx: 1, by: 2, bz: 3, x: 1.5, y: 3.0, z: 3.5, block: "x".into() };
        let a = snap(); let mut b = snap(); b.use_ = true; b.looking_at = Some(h);
        assert!(detect_place(&a, &b).is_some());
    }
    #[test] fn echo_waits_for_history() {
        let mut c = Clones::new(); c.mode = Mode::Echo; c.spawn(1, &snap());
        assert!(c.tick(&snap()).is_empty());
    }
    #[test] fn budget_respected() {
        let mut c = Clones::new(); c.spawn(MAX_CLONES, &snap());
        let mut s = snap(); s.sneaking = true;
        assert!(c.tick(&s).len() <= MAX_CMDS_PER_TICK);
    }
}
