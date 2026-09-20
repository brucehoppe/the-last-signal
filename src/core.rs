use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const WIDTH: i32 = 44;
pub const HEIGHT: i32 = 28;
pub const RECORDS: [&str; 3] = [
    "Maintenance 04: The relay failed after coolant was diverted to the sealed lower levels.",
    "Evacuation 11: Survivors left through the surface lift. Their destination was North Station.",
    "Operator 19: Restore the relay using all three archive keys, then return to the surface lift.",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Pos {
    pub x: i32,
    pub y: i32,
}
impl Pos {
    pub fn distance(self, other: Self) -> i32 {
        (self.x - other.x).abs() + (self.y - other.y).abs()
    }
    pub fn offset(self, dx: i32, dy: i32) -> Self {
        Self {
            x: self.x + dx,
            y: self.y + dy,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tile {
    Wall,
    Floor,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Enemy {
    pub pos: Pos,
    pub hp: i32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Archive {
    pub pos: Pos,
    pub id: usize,
    pub recovered: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Outcome {
    Exploring,
    Escaped,
    Dead,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Event {
    pub turn: u32,
    pub kind: String,
    pub text: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Chat {
    pub role: String,
    pub content: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Game {
    pub seed: u64,
    pub tiles: Vec<Tile>,
    pub seen: Vec<bool>,
    pub visible: Vec<bool>,
    pub player: Pos,
    pub lift: Pos,
    pub relay: Pos,
    pub enemies: Vec<Enemy>,
    pub archives: Vec<Archive>,
    pub hp: i32,
    pub medkits: u32,
    pub energy: u32,
    pub turn: u32,
    pub restored: bool,
    pub outcome: Outcome,
    pub events: Vec<Event>,
    pub chat: Vec<Chat>,
}
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn range(&mut self, lo: i32, hi: i32) -> i32 {
        lo + (self.next() % ((hi - lo) as u64)) as i32
    }
}

impl Game {
    pub fn new(seed: u64) -> Self {
        let mut rng = Rng(seed.max(1));
        let mut g = Self {
            seed,
            tiles: vec![Tile::Wall; (WIDTH * HEIGHT) as usize],
            seen: vec![false; (WIDTH * HEIGHT) as usize],
            visible: vec![false; (WIDTH * HEIGHT) as usize],
            player: Pos { x: 0, y: 0 },
            lift: Pos { x: 0, y: 0 },
            relay: Pos { x: 0, y: 0 },
            enemies: vec![],
            archives: vec![],
            hp: 24,
            medkits: 3,
            energy: 6,
            turn: 0,
            restored: false,
            outcome: Outcome::Exploring,
            events: vec![],
            chat: vec![],
        };
        let mut centers = vec![];
        for row in 0..2 {
            for col in 0..3 {
                let x = 2 + col * 14 + rng.range(0, 2);
                let y = 2 + row * 13 + rng.range(0, 2);
                let w = rng.range(8, 11);
                let h = rng.range(7, 10);
                for yy in y..y + h {
                    for xx in x..x + w {
                        g.carve(Pos { x: xx, y: yy });
                    }
                }
                let p = Pos {
                    x: x + w / 2,
                    y: y + h / 2,
                };
                if let Some(prev) = centers.last().copied() {
                    g.corridor(prev, p);
                }
                centers.push(p);
            }
        }
        g.player = centers[0];
        g.lift = centers[0];
        g.relay = centers[5];
        for (id, &pos) in centers[2..5].iter().enumerate() {
            g.archives.push(Archive {
                pos,
                id,
                recovered: false,
            });
        }
        for (i, &c) in centers.iter().enumerate().skip(1) {
            g.enemies.push(Enemy {
                pos: c.offset(2, 1),
                hp: if i == 5 { 8 } else { 6 },
            });
        }
        g.log(
            "arrival",
            "Surface lift reached. Recover three archives, restore the relay, and return here.",
        );
        g.update_visibility(7);
        g
    }
    pub fn index(p: Pos) -> Option<usize> {
        if p.x >= 0 && p.y >= 0 && p.x < WIDTH && p.y < HEIGHT {
            Some((p.y * WIDTH + p.x) as usize)
        } else {
            None
        }
    }
    pub fn floor(&self, p: Pos) -> bool {
        Self::index(p).is_some_and(|i| self.tiles[i] == Tile::Floor)
    }
    fn carve(&mut self, p: Pos) {
        if let Some(i) = Self::index(p) {
            self.tiles[i] = Tile::Floor;
        }
    }
    fn corridor(&mut self, a: Pos, b: Pos) {
        for x in a.x.min(b.x)..=a.x.max(b.x) {
            self.carve(Pos { x, y: a.y });
        }
        for y in a.y.min(b.y)..=a.y.max(b.y) {
            self.carve(Pos { x: b.x, y });
        }
    }
    pub fn can_see(&self, p: Pos) -> bool {
        Self::index(p).is_some_and(|i| self.visible[i])
    }
    pub fn discovered(&self, p: Pos) -> bool {
        Self::index(p).is_some_and(|i| self.seen[i])
    }
    pub fn line_clear(&self, a: Pos, b: Pos) -> bool {
        let (mut x, mut y) = (a.x, a.y);
        let dx = (b.x - x).abs();
        let dy = -(b.y - y).abs();
        let sx = if x < b.x { 1 } else { -1 };
        let sy = if y < b.y { 1 } else { -1 };
        let mut err = dx + dy;
        loop {
            if x == b.x && y == b.y {
                return true;
            }
            if (x != a.x || y != a.y) && !self.floor(Pos { x, y }) {
                return false;
            }
            let e = 2 * err;
            if e >= dy {
                err += dy;
                x += sx;
            }
            if e <= dx {
                err += dx;
                y += sy;
            }
        }
    }
    pub fn update_visibility(&mut self, radius: i32) {
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let p = Pos { x, y };
                let i = Self::index(p).unwrap();
                let dx = x - self.player.x;
                let dy = y - self.player.y;
                self.visible[i] =
                    dx * dx + dy * dy <= radius * radius && self.line_clear(self.player, p);
                self.seen[i] |= self.visible[i];
            }
        }
    }
    pub fn log(&mut self, kind: &str, text: &str) {
        self.events.push(Event {
            turn: self.turn,
            kind: kind.into(),
            text: text.into(),
        });
        if self.events.len() > 2000 {
            self.events.remove(0);
        }
    }
    pub fn add_chat(&mut self, role: &str, text: &str) {
        self.chat.push(Chat {
            role: role.into(),
            content: text.chars().take(1200).collect(),
        });
        if self.chat.len() > 100 {
            self.chat.remove(0);
        }
    }
    pub fn recovered(&self) -> usize {
        self.archives.iter().filter(|a| a.recovered).count()
    }
    pub fn step(&mut self, dx: i32, dy: i32) {
        if self.outcome != Outcome::Exploring || dx.abs() + dy.abs() != 1 {
            return;
        }
        let next = self.player.offset(dx, dy);
        if !self.floor(next) {
            return;
        }
        if let Some(i) = self.enemies.iter().position(|e| e.pos == next) {
            self.enemies[i].hp -= 3;
            self.log("combat", "You hit a sentinel for 3 damage.");
            if self.enemies[i].hp <= 0 {
                self.enemies.remove(i);
                self.log("combat", "Sentinel disabled.");
            }
        } else {
            self.player = next;
        }
        self.finish_turn();
    }
    pub fn wait(&mut self) {
        if self.outcome == Outcome::Exploring {
            self.finish_turn();
        }
    }
    pub fn heal(&mut self) {
        if self.outcome != Outcome::Exploring {
            return;
        }
        if self.medkits == 0 {
            self.log("status", "No medkits remain.");
            return;
        }
        if self.hp == 24 {
            self.log("status", "Health is already full.");
            return;
        }
        self.medkits -= 1;
        self.hp = (self.hp + 10).min(24);
        self.log("healing", "Used a medkit: restored up to 10 health.");
        self.finish_turn();
    }
    pub fn scan(&mut self) {
        if self.outcome != Outcome::Exploring {
            return;
        }
        if self.energy == 0 {
            self.log("status", "Scanner energy depleted.");
            return;
        }
        self.energy -= 1;
        self.log(
            "scan",
            "Scanner pulse extends sight until your next action. Walls still block it.",
        );
        self.finish_turn();
        self.update_visibility(11);
    }
    pub fn interact(&mut self) {
        if self.outcome != Outcome::Exploring {
            return;
        }
        if let Some(i) = self
            .archives
            .iter()
            .position(|a| !a.recovered && a.pos.distance(self.player) <= 1)
        {
            self.archives[i].recovered = true;
            let id = self.archives[i].id;
            self.log("archive", RECORDS[id]);
            self.finish_turn();
            return;
        }
        if self.player.distance(self.relay) <= 1 {
            if self.recovered() == 3 && !self.restored {
                self.restored = true;
                self.log("relay", "Relay restored. Return to the surface lift.");
                self.finish_turn();
            } else if !self.restored {
                self.log("status", "Relay needs three recovered archive keys.");
            } else {
                self.log("status", "Relay is online. Return to the surface lift.");
            }
            return;
        }
        if self.player.distance(self.lift) <= 1 {
            if self.restored {
                self.outcome = Outcome::Escaped;
                self.log(
                    "escape",
                    "Signal transmitted. You escaped with the recovered evidence.",
                );
            } else {
                self.log("status", "Restore the relay before departure.");
            }
            return;
        }
        self.log(
            "status",
            "Nothing to interact with here. Stand beside an archive, relay, or lift.",
        );
    }
    fn finish_turn(&mut self) {
        self.turn += 1;
        for i in 0..self.enemies.len() {
            let p = self.enemies[i].pos;
            if p.distance(self.player) == 1 {
                self.hp -= 1;
                self.log("damage", "A sentinel strikes you for 1 damage.");
                if self.hp <= 0 {
                    self.hp = 0;
                    self.outcome = Outcome::Dead;
                    self.log(
                        "death",
                        "Expedition lost. Start a new signal or reload your save.",
                    );
                    break;
                }
            } else if p.distance(self.player) <= 7 && self.line_clear(p, self.player) {
                let mut choices = [
                    p.offset(1, 0),
                    p.offset(-1, 0),
                    p.offset(0, 1),
                    p.offset(0, -1),
                ];
                choices.sort_by_key(|q| q.distance(self.player));
                if let Some(q) = choices.into_iter().find(|q| {
                    q.distance(self.player) < p.distance(self.player)
                        && self.floor(*q)
                        && *q != self.player
                        && !self.enemies.iter().any(|e| e.pos == *q)
                }) {
                    self.enemies[i].pos = q;
                }
            }
        }
        self.update_visibility(7);
    }
    /// Shortest path over discovered walkable tiles only. `None` when the
    /// target is undiscovered or not connected through the known map.
    pub fn known_route(&self, target: Pos) -> Option<Vec<Pos>> {
        if !self.discovered(target) || !self.floor(target) {
            return None;
        }
        let mut previous = std::collections::HashMap::from([(self.player, self.player)]);
        let mut queue = std::collections::VecDeque::from([self.player]);
        while let Some(p) = queue.pop_front() {
            if p == target {
                let mut path = vec![p];
                let mut cur = p;
                while cur != self.player {
                    cur = previous[&cur];
                    path.push(cur);
                }
                path.reverse();
                return Some(path);
            }
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let n = p.offset(dx, dy);
                if self.floor(n) && self.discovered(n) && !previous.contains_key(&n) {
                    previous.insert(n, p);
                    queue.push_back(n);
                }
            }
        }
        None
    }
    pub fn knowledge(&self) -> serde_json::Value {
        let facts: Vec<&str> = self
            .archives
            .iter()
            .filter(|a| a.recovered)
            .map(|a| RECORDS[a.id])
            .collect();
        let threats: Vec<_> = self
            .enemies
            .iter()
            .filter(|e| self.can_see(e.pos))
            .map(|e| serde_json::json!({"position":e.pos,"hp":e.hp,"type":"sentinel"}))
            .collect();
        let landmarks: Vec<_> = self
            .archives
            .iter()
            .filter(|a| self.discovered(a.pos))
            .map(|a| serde_json::json!({"kind":"archive","position":a.pos,"recovered":a.recovered}))
            .collect();
        let mut targets = vec![("lift", self.lift)];
        if self.discovered(self.relay) {
            targets.push(("relay", self.relay));
        }
        targets.extend(
            self.archives
                .iter()
                .filter(|a| !a.recovered && self.discovered(a.pos))
                .map(|a| ("archive", a.pos)),
        );
        let routes: Vec<_> = targets
            .into_iter()
            .map(|(kind, pos)| {
                let steps = self.known_route(pos).map(|r| r.len() - 1);
                serde_json::json!({"target":kind,"position":pos,"steps":steps,"as_of_turn":self.turn})
            })
            .collect();
        serde_json::json!({"turn":self.turn,"known_routes":routes,"position":self.player,"hp":self.hp,"medkits":self.medkits,"scanner_energy":self.energy,"keys_recovered":self.recovered(),"relay_restored":self.restored,"outcome":self.outcome,"discovered_records":facts,"visible_threats":threats,"known_archives":landmarks,"known_lift":self.lift,"known_relay":if self.discovered(self.relay){Some(self.relay)}else{None},"recent_events":self.events.iter().rev().take(16).collect::<Vec<_>>()})
    }
    pub fn validate(&self) -> Result<(), String> {
        let n = (WIDTH * HEIGHT) as usize;
        if self.tiles.len() != n || self.seen.len() != n || self.visible.len() != n {
            return Err("Invalid map dimensions".into());
        }
        if !self.floor(self.player) || !self.floor(self.lift) || !self.floor(self.relay) {
            return Err("Invalid player or landmark".into());
        }
        if self.hp < 0
            || self.hp > 24
            || self.medkits > 3
            || self.energy > 6
            || self.enemies.len() > 5
            || self.archives.len() != 3
        {
            return Err("Invalid expedition values".into());
        }
        let mut ids = HashSet::new();
        let mut positions = HashSet::new();
        for a in &self.archives {
            if a.id >= 3 || !ids.insert(a.id) || !self.floor(a.pos) {
                return Err("Invalid archive".into());
            }
        }
        for e in &self.enemies {
            if !self.floor(e.pos)
                || e.pos == self.player
                || !positions.insert(e.pos)
                || e.hp <= 0
                || e.hp > 8
            {
                return Err("Invalid sentinel".into());
            }
        }
        if self.events.len() > 2000
            || self.chat.len() > 100
            || self.chat.iter().any(|c| {
                !matches!(c.role.as_str(), "user" | "assistant") || c.content.chars().count() > 1200
            })
        {
            return Err("Invalid history".into());
        }
        if self.restored && self.recovered() != 3 {
            return Err("Missing relay keys".into());
        }
        if (self.outcome == Outcome::Dead) != (self.hp == 0)
            || self.outcome == Outcome::Escaped && !self.restored
        {
            return Err("Inconsistent outcome".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    #[test]
    fn generated_objectives_are_reachable() {
        for seed in 0..100 {
            let g = Game::new(seed);
            g.validate().unwrap();
            let mut reached = HashSet::from([g.player]);
            let mut q = VecDeque::from([g.player]);
            while let Some(p) = q.pop_front() {
                for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    let n = p.offset(dx, dy);
                    if g.floor(n) && reached.insert(n) {
                        q.push_back(n);
                    }
                }
            }
            assert!(reached.contains(&g.relay));
            assert!(g.archives.iter().all(|a| reached.contains(&a.pos)));
        }
    }
    #[test]
    fn hidden_records_and_threats_are_not_sent() {
        let mut g = Game::new(42);
        g.seen.fill(false);
        g.visible.fill(false);
        let k = g.knowledge();
        assert_eq!(k["discovered_records"].as_array().unwrap().len(), 0);
        assert!(k["known_relay"].is_null());
        assert_eq!(k["visible_threats"].as_array().unwrap().len(), 0);
        assert!(!k.to_string().contains("North Station"));
    }
    #[test]
    fn routes_use_only_discovered_tiles() {
        let mut g = Game::new(42);
        g.seen.fill(false);
        g.visible.fill(false);
        g.seen[Game::index(g.player).unwrap()] = true;
        assert!(g.known_route(g.relay).is_none());
        assert!(g.known_route(g.lift).is_some_and(|r| r.len() == 1));
        g.seen.fill(true);
        let r = g.known_route(g.relay).unwrap();
        assert_eq!(r[0], g.player);
        assert_eq!(*r.last().unwrap(), g.relay);
        // Sever the known map: hide one tile on the route.
        let cut = r[r.len() / 2];
        g.seen[Game::index(cut).unwrap()] = false;
        g.visible[Game::index(cut).unwrap()] = false;
        let detour = g.known_route(g.relay);
        assert!(detour.is_none_or(|d| !d.contains(&cut)));
    }
    #[test]
    fn same_seed_same_map() {
        assert_eq!(Game::new(9).tiles, Game::new(9).tiles);
        assert_ne!(Game::new(9).tiles, Game::new(10).tiles);
    }
    #[test]
    fn exit_requires_records_and_relay() {
        let mut g = Game::new(9);
        g.interact();
        assert_eq!(g.outcome, Outcome::Exploring);
        g.enemies.clear();
        for a in &mut g.archives {
            a.recovered = true;
        }
        g.player = g.relay;
        g.interact();
        assert!(g.restored);
        g.player = g.lift;
        g.interact();
        assert_eq!(g.outcome, Outcome::Escaped);
        g.validate().unwrap();
    }
    #[test]
    fn wall_blocks_sight() {
        let mut g = Game::new(2);
        g.tiles.fill(Tile::Floor);
        let a = Pos { x: 2, y: 2 };
        let wall = Pos { x: 3, y: 2 };
        g.tiles[Game::index(wall).unwrap()] = Tile::Wall;
        assert!(!g.line_clear(a, Pos { x: 4, y: 2 }));
        assert!(g.line_clear(a, wall));
    }
}
