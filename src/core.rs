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
pub const START_ENERGY: u32 = 6;
pub const SCAN_COST: u32 = 1;
pub const ANALYZE_COST: u32 = 2;
pub const MAX_ACTIONS: usize = 20_000;
/// Fitted power modules. All of them draw on one shared power pool, so every
/// module you fit is a decision about what to spend it on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Module {
    Scanner,
    Shield,
    Analyzer,
}
impl Module {
    pub const ALL: [Module; 3] = [Module::Scanner, Module::Shield, Module::Analyzer];
    pub const SLOTS: usize = 2;
    pub fn name(self) -> &'static str {
        match self {
            Module::Scanner => "Scanner Array",
            Module::Shield => "Shield Cell",
            Module::Analyzer => "Field Analyzer",
        }
    }
    pub fn key_hint(self) -> &'static str {
        match self {
            Module::Scanner => "F",
            Module::Shield => "auto",
            Module::Analyzer => "G",
        }
    }
    pub fn effect(self) -> &'static str {
        match self {
            Module::Scanner => "Sight radius 11 for one turn. Costs 1 power per pulse.",
            Module::Shield => "Absorbs a sentinel strike. Costs 1 power per hit absorbed.",
            Module::Analyzer => {
                "Pinpoints the nearest unrecovered archive through walls. Costs 2 power."
            }
        }
    }
}
pub fn default_loadout() -> Vec<Module> {
    vec![Module::Scanner, Module::Shield]
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Faction {
    Wardens,
    Custodians,
}
impl Faction {
    pub const ALL: [Faction; 2] = [Faction::Wardens, Faction::Custodians];
    pub fn name(self) -> &'static str {
        match self {
            Faction::Wardens => "Wardens",
            Faction::Custodians => "Custodians",
        }
    }
    pub fn about(self) -> &'static str {
        match self {
            Faction::Wardens => "The complex's human crew, who evacuated through the surface lift and kept the archives.",
            Faction::Custodians => "The complex's automated security. Sentinels are theirs, and they still guard the relay.",
        }
    }
}
/// Which faction authored each archive record (index = record id).
pub const RECORD_AUTHORS: [Faction; 3] = [Faction::Custodians, Faction::Wardens, Faction::Wardens];
/// A player action. The ordered list of these plus the seed and loadout is
/// enough to replay a whole expedition.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    Move(i32, i32),
    Wait,
    Heal,
    Scan,
    Analyze,
    Interact,
}
#[derive(Clone, Debug)]
pub struct Summary {
    pub rank: &'static str,
    pub lines: Vec<String>,
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
    // Added after save version 1; defaults keep old saves loadable.
    #[serde(default = "default_loadout")]
    pub loadout: Vec<Module>,
    #[serde(default)]
    pub actions: Vec<Action>,
    /// True while `actions` is a complete record of the run (false for saves
    /// made before the log existed, or if it overflowed).
    #[serde(default)]
    pub replayable: bool,
    #[serde(default)]
    pub kills: u32,
    /// Standing with `Faction::ALL`, in the same order.
    #[serde(default)]
    pub standing: [i32; 2],
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
        Self::new_with(seed, &default_loadout())
    }
    pub fn new_with(seed: u64, loadout: &[Module]) -> Self {
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
            energy: START_ENERGY,
            turn: 0,
            restored: false,
            outcome: Outcome::Exploring,
            events: vec![],
            chat: vec![],
            loadout: loadout.to_vec(),
            actions: vec![],
            replayable: true,
            kills: 0,
            standing: [0, 0],
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
        // Only the circle's bounding box can be visible; everything else is cleared.
        self.visible.fill(false);
        let (px, py) = (self.player.x, self.player.y);
        for y in (py - radius).max(0)..=(py + radius).min(HEIGHT - 1) {
            for x in (px - radius).max(0)..=(px + radius).min(WIDTH - 1) {
                let (dx, dy) = (x - px, y - py);
                if dx * dx + dy * dy <= radius * radius
                    && self.line_clear(self.player, Pos { x, y })
                {
                    let i = (y * WIDTH + x) as usize;
                    self.visible[i] = true;
                    self.seen[i] = true;
                }
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
            self.events.drain(..self.events.len() - 2000);
        }
    }
    pub fn add_chat(&mut self, role: &str, text: &str) {
        self.chat.push(Chat {
            role: role.into(),
            content: text.chars().take(1200).collect(),
        });
        if self.chat.len() > 100 {
            self.chat.drain(..self.chat.len() - 100);
        }
    }
    pub fn recovered(&self) -> usize {
        self.archives.iter().filter(|a| a.recovered).count()
    }
    pub fn has(&self, m: Module) -> bool {
        self.loadout.contains(&m)
    }
    fn record(&mut self, a: Action) {
        if self.actions.len() < MAX_ACTIONS {
            self.actions.push(a);
        } else {
            self.replayable = false;
        }
    }
    /// Perform a recorded action through the same rules a player uses.
    pub fn apply(&mut self, a: Action) {
        match a {
            Action::Move(dx, dy) => self.step(dx, dy),
            Action::Wait => self.wait(),
            Action::Heal => self.heal(),
            Action::Scan => self.scan(),
            Action::Analyze => self.analyze(),
            Action::Interact => self.interact(),
        }
    }
    pub fn replay(seed: u64, loadout: &[Module], actions: &[Action]) -> Self {
        let mut g = Self::new_with(seed, loadout);
        for &a in actions {
            g.apply(a);
        }
        g
    }
    /// True when replaying the recorded actions from the seed reproduces this
    /// exact game state: the log is a faithful, deterministic trace.
    pub fn replay_matches(&self) -> bool {
        if !self.replayable {
            return false;
        }
        let r = Self::replay(self.seed, &self.loadout, &self.actions);
        r.player == self.player
            && r.hp == self.hp
            && r.medkits == self.medkits
            && r.energy == self.energy
            && r.turn == self.turn
            && r.outcome == self.outcome
            && r.restored == self.restored
            && r.kills == self.kills
            && r.standing == self.standing
            && r.recovered() == self.recovered()
            && r.enemies.len() == self.enemies.len()
            && r.enemies
                .iter()
                .zip(&self.enemies)
                .all(|(a, b)| a.pos == b.pos && a.hp == b.hp)
    }
    fn shift_standing(&mut self, f: Faction, by: i32) {
        let i = Faction::ALL.iter().position(|x| *x == f).unwrap();
        self.standing[i] = (self.standing[i] + by).clamp(-20, 20);
        self.log(
            "standing",
            &format!("{} standing {:+} (now {}).", f.name(), by, self.standing[i]),
        );
    }
    pub fn standing_of(&self, f: Faction) -> i32 {
        self.standing[Faction::ALL.iter().position(|x| *x == f).unwrap()]
    }
    pub fn step(&mut self, dx: i32, dy: i32) {
        if self.outcome != Outcome::Exploring || dx.abs() + dy.abs() != 1 {
            return;
        }
        let next = self.player.offset(dx, dy);
        if !self.floor(next) {
            return;
        }
        self.record(Action::Move(dx, dy));
        if let Some(i) = self.enemies.iter().position(|e| e.pos == next) {
            self.enemies[i].hp -= 3;
            self.log("combat", "You hit a sentinel for 3 damage.");
            if self.enemies[i].hp <= 0 {
                self.enemies.remove(i);
                self.kills += 1;
                self.log("combat", "Sentinel disabled.");
                self.shift_standing(Faction::Custodians, -1);
            }
        } else {
            self.player = next;
        }
        self.finish_turn();
    }
    pub fn wait(&mut self) {
        if self.outcome == Outcome::Exploring {
            self.record(Action::Wait);
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
        self.record(Action::Heal);
        self.medkits -= 1;
        self.hp = (self.hp + 10).min(24);
        self.log("healing", "Used a medkit: restored up to 10 health.");
        self.finish_turn();
    }
    pub fn scan(&mut self) {
        if self.outcome != Outcome::Exploring {
            return;
        }
        if !self.has(Module::Scanner) {
            self.log(
                "status",
                "No Scanner Array fitted. Change your loadout before an expedition.",
            );
            return;
        }
        if self.energy < SCAN_COST {
            self.log("status", "Not enough power for a scanner pulse.");
            return;
        }
        self.record(Action::Scan);
        self.energy -= SCAN_COST;
        self.log(
            "scan",
            "Scanner pulse extends sight until your next action. Walls still block it.",
        );
        self.finish_turn();
        self.update_visibility(11);
    }
    pub fn analyze(&mut self) {
        if self.outcome != Outcome::Exploring {
            return;
        }
        if !self.has(Module::Analyzer) {
            self.log(
                "status",
                "No Field Analyzer fitted. Change your loadout before an expedition.",
            );
            return;
        }
        if self.energy < ANALYZE_COST {
            self.log("status", "Not enough power for the analyzer (needs 2).");
            return;
        }
        let Some(target) = self
            .archives
            .iter()
            .filter(|a| !a.recovered)
            .min_by_key(|a| a.pos.distance(self.player))
            .map(|a| a.pos)
        else {
            self.log("status", "No unrecovered archives remain to analyze.");
            return;
        };
        self.record(Action::Analyze);
        self.energy -= ANALYZE_COST;
        for dy in -1..=1 {
            for dx in -1..=1 {
                if let Some(i) = Self::index(target.offset(dx, dy)) {
                    self.seen[i] = true;
                }
            }
        }
        let (dx, dy) = (target.x - self.player.x, target.y - self.player.y);
        let dir = if dx.abs() >= dy.abs() {
            if dx >= 0 {
                "east"
            } else {
                "west"
            }
        } else if dy >= 0 {
            "south"
        } else {
            "north"
        };
        self.log(
            "analysis",
            &format!(
                "Analyzer pinged an archive {} tiles away, to the {dir}. It is now on your map.",
                self.player.distance(target)
            ),
        );
        self.finish_turn();
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
            self.record(Action::Interact);
            self.archives[i].recovered = true;
            let id = self.archives[i].id;
            self.log("archive", RECORDS[id]);
            if RECORD_AUTHORS[id] == Faction::Wardens {
                self.shift_standing(Faction::Wardens, 1);
            }
            self.finish_turn();
            return;
        }
        if self.player.distance(self.relay) <= 1 {
            if self.recovered() == 3 && !self.restored {
                self.record(Action::Interact);
                self.restored = true;
                self.log("relay", "Relay restored. Return to the surface lift.");
                self.shift_standing(Faction::Wardens, 1);
                self.shift_standing(Faction::Custodians, 1);
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
                self.record(Action::Interact);
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
                if self.has(Module::Shield) && self.energy > 0 {
                    self.energy -= 1;
                    self.log(
                        "shield",
                        "Shield Cell absorbed a sentinel strike (-1 power).",
                    );
                    continue;
                }
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
    /// Context-sensitive onboarding tip, derived purely from state so it needs
    /// no saved flags and can never disagree with the game.
    pub fn hint(&self) -> Option<&'static str> {
        if self.outcome != Outcome::Exploring {
            return None;
        }
        if self.turn == 0 {
            return Some("Move with WASD or the arrows. J opens your evidence, Esc the menu.");
        }
        if self
            .enemies
            .iter()
            .any(|e| e.pos.distance(self.player) == 1)
        {
            return Some(
                "A sentinel is adjacent: bump into it to strike for 3. It hits back for 1.",
            );
        }
        if self.hp <= 10 && self.medkits > 0 {
            return Some("Health is low: press H to use a medkit (+10 HP, costs a turn).");
        }
        if self.enemies.iter().any(|e| self.can_see(e.pos)) {
            return Some(
                "Sentinel in sight (S). Let it come to you in a corridor, then strike first.",
            );
        }
        if self
            .archives
            .iter()
            .any(|a| !a.recovered && a.pos.distance(self.player) <= 1)
        {
            return Some("Press E beside an archive (A) to recover its record.");
        }
        if self.recovered() == 3 && !self.restored {
            return Some(if self.relay.distance(self.player) <= 1 {
                "Press E at the relay (R) to restore it."
            } else {
                "All three keys recovered. Head for the relay (R)."
            });
        }
        if self.restored {
            return Some("Relay online. Return to the lift (L) and press E to transmit.");
        }
        if self.recovered() == 0 && self.turn < 25 {
            return Some("Explore to find archives (A). Scan (F) or analyze (G) if fitted; each costs power.");
        }
        None
    }
    pub fn summary(&self) -> Summary {
        let wardens = self.standing_of(Faction::Wardens);
        let custodians = self.standing_of(Faction::Custodians);
        let rank = match self.outcome {
            Outcome::Dead => "Lost Signal",
            Outcome::Exploring => "Expedition in progress",
            Outcome::Escaped if self.kills == 0 => "Silent Signal",
            Outcome::Escaped if self.kills >= 4 => "Custodian's Bane",
            Outcome::Escaped if wardens >= 3 => "Warden's Friend",
            Outcome::Escaped => "Signal Bearer",
        };
        let epilogue = match self.outcome {
            Outcome::Escaped if custodians >= 1 => {
                "The Custodians let the signal pass and stand down."
            }
            Outcome::Escaped => "The Wardens receive the signal. The Custodians will remember the sentinels you broke.",
            Outcome::Dead => "The relay stays dark. The Wardens' last message goes unanswered.",
            Outcome::Exploring => "",
        };
        Summary {
            rank,
            lines: vec![
                format!("Turns {}   Sentinels disabled {}", self.turn, self.kills),
                format!(
                    "Archives {}/3   Power left {}",
                    self.recovered(),
                    self.energy
                ),
                format!("Wardens {wardens:+}   Custodians {custodians:+}"),
                epilogue.into(),
            ],
        }
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
        serde_json::json!({"turn":self.turn,"known_routes":routes,"position":self.player,"hp":self.hp,"medkits":self.medkits,"power":self.energy,"loadout":self.loadout.iter().map(|m|m.name()).collect::<Vec<_>>(),"factions":Faction::ALL.iter().map(|f|serde_json::json!({"name":f.name(),"about":f.about(),"standing":self.standing_of(*f)})).collect::<Vec<_>>(),"keys_recovered":self.recovered(),"relay_restored":self.restored,"outcome":self.outcome,"discovered_records":facts,"visible_threats":threats,"known_archives":landmarks,"known_lift":self.lift,"known_relay":if self.discovered(self.relay){Some(self.relay)}else{None},"recent_events":self.events.iter().rev().take(16).collect::<Vec<_>>()})
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
        if self.loadout.len() > Module::SLOTS
            || (1..self.loadout.len()).any(|i| self.loadout[..i].contains(&self.loadout[i]))
            || self.energy > START_ENERGY
            || self.actions.len() > MAX_ACTIONS
            || self.standing.iter().any(|s| s.abs() > 20)
        {
            return Err("Invalid loadout or trace".into());
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
    fn adjacent_sentinel(seed: u64, loadout: &[Module]) -> Game {
        let mut g = Game::new_with(seed, loadout);
        g.enemies.clear();
        let start = g.player;
        g.enemies.push(Enemy {
            pos: start.offset(1, 0),
            hp: 6,
        });
        g
    }
    #[test]
    fn shield_spends_power_instead_of_health() {
        let mut g = adjacent_sentinel(7, &[Module::Shield]);
        g.wait();
        assert_eq!((g.hp, g.energy), (24, START_ENERGY - 1));
        g.energy = 0;
        g.wait();
        assert_eq!(g.hp, 23, "with no power the strike lands");
        let mut bare = adjacent_sentinel(7, &[Module::Scanner]);
        bare.wait();
        assert_eq!((bare.hp, bare.energy), (23, START_ENERGY));
    }
    #[test]
    fn unfitted_modules_do_nothing() {
        let mut g = Game::new_with(7, &[Module::Shield]);
        let before = (g.turn, g.energy);
        g.scan();
        g.analyze();
        assert_eq!((g.turn, g.energy), before);
        assert!(g.actions.is_empty());
    }
    #[test]
    fn analyzer_reveals_an_archive_but_not_a_route() {
        let mut g = Game::new_with(7, &[Module::Analyzer, Module::Scanner]);
        g.enemies.clear();
        let target = g
            .archives
            .iter()
            .min_by_key(|a| a.pos.distance(g.player))
            .unwrap()
            .pos;
        assert!(!g.discovered(target));
        g.analyze();
        assert!(g.discovered(target));
        assert_eq!(g.energy, START_ENERGY - ANALYZE_COST);
        assert!(
            g.known_route(target).is_none(),
            "the tile is known, the way is not"
        );
        g.energy = 1;
        let turn = g.turn;
        g.analyze();
        assert_eq!(g.turn, turn, "insufficient power is refused");
    }
    #[test]
    fn factions_react_to_what_you_do() {
        let mut g = adjacent_sentinel(7, &[Module::Shield]);
        g.enemies[0].hp = 3;
        g.step(1, 0);
        assert_eq!((g.kills, g.standing_of(Faction::Custodians)), (1, -1));
        g.player = g.archives[1].pos;
        g.interact();
        assert_eq!(g.standing_of(Faction::Wardens), 1, "Warden-authored record");
        g.player = g.archives[0].pos;
        g.interact();
        assert_eq!(
            g.standing_of(Faction::Wardens),
            1,
            "Custodian-authored record"
        );
    }
    #[test]
    fn replay_reproduces_the_run_and_detects_tampering() {
        let mut g = Game::new(11);
        for (dx, dy) in [(1, 0), (0, 1), (-1, 0), (0, -1), (1, 0)] {
            g.step(dx, dy);
            g.wait();
        }
        assert!(!g.actions.is_empty());
        assert!(g.replay_matches());
        g.hp -= 1;
        assert!(
            !g.replay_matches(),
            "state that the log cannot explain is caught"
        );
    }
    #[test]
    fn hints_follow_the_situation() {
        let mut g = Game::new(3);
        assert!(g.hint().unwrap().contains("WASD"));
        g.enemies.clear();
        g.turn = 40;
        assert!(g.hint().is_none());
        g.hp = 8;
        assert!(g.hint().unwrap().contains("medkit"));
        g.hp = 24;
        g.player = g.archives[0].pos;
        assert!(g.hint().unwrap().contains("archive"));
    }
    #[test]
    fn summary_ranks_the_run() {
        let mut g = Game::new(3);
        g.enemies.clear();
        for a in &mut g.archives {
            a.recovered = true;
        }
        g.player = g.relay;
        g.interact();
        g.player = g.lift;
        g.interact();
        assert_eq!(g.summary().rank, "Silent Signal");
        g.kills = 4;
        assert_eq!(g.summary().rank, "Custodian's Bane");
    }
    #[test]
    fn invalid_loadouts_are_rejected() {
        let mut g = Game::new(3);
        g.loadout = vec![Module::Shield, Module::Shield];
        assert!(g.validate().is_err());
        g.loadout = Module::ALL.to_vec();
        assert!(g.validate().is_err());
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
