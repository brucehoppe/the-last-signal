use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const WIDTH: i32 = 44;
pub const HEIGHT: i32 = 28;
pub const FLOORS: usize = 3;
pub const MAX_HP: i32 = 24;
pub const MAX_MEDKITS: u32 = 5;
pub const FLOOR_NAMES: [&str; FLOORS] = [
    "The Surface Complex",
    "The Coolant Levels",
    "The Signal Vault",
];
/// Nine records, three per floor (id / 3 = floor).
pub const RECORDS: [&str; 9] = [
    "Maintenance 04: The relay failed after coolant was diverted to the sealed lower levels.",
    "Evacuation 11: Survivors left through the surface lift. Their destination was North Station.",
    "Operator 19: Restore the relay using all three archive keys, then return to the surface lift.",
    "Maintenance 22: The coolant was diverted on purpose. The sealed levels were flooded to stop a failing reactor.",
    "Warden Log 07: We sealed the lower levels behind us. The Custodians would not let us carry the archives out.",
    "Custodian Directive 3: Sentinels hold every relay until the Wardens return. This order was never rescinded.",
    "Warden Log 31: North Station has a mast. If the relay signal reaches it, the network wakes and the Custodians stand down.",
    "Custodian Directive 9: Standing down requires an authorised Warden signal. None has been received.",
    "Operator 40: Transmit from the vault lift. Whoever reads this: the Custodians were never enemies, only unfinished.",
];

/// A Custodian terminal's challenge. The answer is stated in `record`, which
/// the player only holds in damaged form until ECHO reads it back.
pub struct Challenge {
    pub question: &'static str,
    pub options: [&'static str; 3],
    pub answer: usize,
    pub record: usize,
}
/// One per floor.
pub const CHALLENGES: [Challenge; FLOORS] = [
    Challenge {
        question: "State the evacuation destination.",
        options: [
            "North Station",
            "The southern docks",
            "The orbital platform",
        ],
        answer: 0,
        record: 1,
    },
    Challenge {
        question: "State why the sealed levels were flooded.",
        options: [
            "A coolant line burst by accident",
            "To destroy the archives",
            "To stop a failing reactor",
        ],
        answer: 2,
        record: 3,
    },
    Challenge {
        question: "State what Custodian units require before standing down.",
        options: [
            "The Overseer's destruction",
            "An authorised Warden signal",
            "A relay power failure",
        ],
        answer: 1,
        record: 7,
    },
];
/// A record as it comes out of a failing archive: the heading survives, the
/// longer words are burned out. Only ECHO holds the full text.
pub fn damaged(record: &str) -> String {
    let (head, body) = record.split_once(':').unwrap_or(("", record));
    let body: String = body
        .split(' ')
        .map(|w| {
            let letters = w.chars().filter(|c| c.is_alphanumeric()).count();
            if letters >= 5 {
                w.chars()
                    .map(|c| if c.is_alphanumeric() { '#' } else { c })
                    .collect()
            } else {
                w.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ");
    format!("{head}:{body}")
}
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
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnemyKind {
    #[default]
    Sentinel,
    Hunter,
    Overseer,
}
impl EnemyKind {
    pub fn name(self) -> &'static str {
        match self {
            EnemyKind::Sentinel => "Sentinel",
            EnemyKind::Hunter => "Hunter",
            EnemyKind::Overseer => "Overseer",
        }
    }
    pub fn glyph(self) -> &'static str {
        match self {
            EnemyKind::Sentinel => "S",
            EnemyKind::Hunter => "H",
            EnemyKind::Overseer => "O",
        }
    }
    pub fn damage(self) -> i32 {
        match self {
            EnemyKind::Sentinel => 1,
            _ => 2,
        }
    }
    /// How far it notices you.
    pub fn sight(self) -> i32 {
        match self {
            EnemyKind::Sentinel => 7,
            _ => 9,
        }
    }
    pub fn max_hp(self, floor: usize) -> i32 {
        match self {
            EnemyKind::Sentinel => 6 + floor as i32,
            EnemyKind::Hunter => 5 + floor as i32,
            EnemyKind::Overseer => 12,
        }
    }
    /// The Overseer is heavy: it closes in only every other turn (it still strikes every turn).
    fn advances(self, turn: u32) -> bool {
        self != EnemyKind::Overseer || turn.is_multiple_of(2)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Enemy {
    pub pos: Pos,
    pub hp: i32,
    #[serde(default)]
    pub kind: EnemyKind,
}
/// Equipment lying in the complex: a module you can take with E.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Cache {
    pub pos: Pos,
    pub module: Module,
    pub taken: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TerminalState {
    Locked,
    Solved,
    Failed,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Terminal {
    pub pos: Pos,
    pub state: TerminalState,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Archive {
    pub pos: Pos,
    pub id: usize,
    pub recovered: bool,
}
/// Supplies lying on the floor: walk over one to take it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PickupKind {
    PowerCell,
    Medkit,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Pickup {
    pub pos: Pos,
    pub kind: PickupKind,
    pub taken: bool,
}
pub const START_ENERGY: u32 = 6;
/// Power cells can charge you past the starting level, up to this.
pub const MAX_ENERGY: u32 = 8;
pub const CELL_POWER: u32 = 2;
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
    /// Slots open up as you descend: one per floor, up to this many.
    pub const MAX_SLOTS: usize = 3;
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
/// The single module you begin with; the others are found in caches.
pub fn default_loadout() -> Vec<Module> {
    vec![Module::Shield]
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
            Faction::Wardens => "The human crew who evacuated and kept the archives.",
            Faction::Custodians => {
                "The automated security. Sentinels are theirs; they still guard the relays."
            }
        }
    }
}
/// Which faction authored each archive record (index = record id).
pub const RECORD_AUTHORS: [Faction; 9] = [
    Faction::Custodians,
    Faction::Wardens,
    Faction::Wardens,
    Faction::Custodians,
    Faction::Wardens,
    Faction::Custodians,
    Faction::Wardens,
    Faction::Custodians,
    Faction::Wardens,
];
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
    Fit(Module),
    Unfit(Module),
    Answer(usize),
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
    /// 0-based floor index. `archives`, `enemies`, `caches`, the map and the lift
    /// are all for this floor only.
    #[serde(default)]
    pub floor: usize,
    /// Every module you have found; `loadout` is the subset currently fitted.
    #[serde(default)]
    pub owned: Vec<Module>,
    /// Ids of every record recovered on any floor.
    #[serde(default)]
    pub records_found: Vec<usize>,
    #[serde(default)]
    pub caches: Vec<Cache>,
    #[serde(default)]
    pub pickups: Vec<Pickup>,
    /// This floor's Custodian terminal (absent in saves from before terminals).
    #[serde(default)]
    pub terminal: Option<Terminal>,
    /// Records ECHO has read back in full. Interface knowledge, not a rule: it
    /// changes what the journal shows, never what an action does.
    #[serde(default)]
    pub decoded: Vec<usize>,
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
    pub fn new_with(seed: u64, starter: &[Module]) -> Self {
        let mut fitted: Vec<Module> = vec![];
        for m in starter {
            if !fitted.contains(m) && fitted.is_empty() {
                fitted.push(*m);
            }
        }
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
            hp: MAX_HP,
            medkits: 3,
            energy: START_ENERGY,
            turn: 0,
            restored: false,
            outcome: Outcome::Exploring,
            events: vec![],
            chat: vec![],
            loadout: fitted.clone(),
            owned: fitted,
            actions: vec![],
            replayable: true,
            kills: 0,
            standing: [0, 0],
            floor: 0,
            records_found: vec![],
            caches: vec![],
            pickups: vec![],
            terminal: None,
            decoded: vec![],
        };
        g.build_floor(0);
        g
    }
    /// Module slots open on the current floor.
    pub fn slots(&self) -> usize {
        (1 + self.floor).min(Module::MAX_SLOTS)
    }
    /// Lay out floor `floor`: rooms, lift, relay, archives, foes and equipment.
    fn build_floor(&mut self, floor: usize) {
        let mut rng = Rng((self.seed ^ (floor as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)).max(1));
        self.tiles.fill(Tile::Wall);
        self.seen.fill(false);
        self.visible.fill(false);
        self.enemies.clear();
        self.archives.clear();
        self.caches.clear();
        self.restored = false;
        self.pickups.clear();
        let mut centers = vec![];
        let mut rooms = vec![];
        for row in 0..2 {
            for col in 0..3 {
                let x = 2 + col * 14 + rng.range(0, 2);
                let y = 2 + row * 13 + rng.range(0, 2);
                let w = rng.range(8, 11);
                let h = rng.range(7, 10);
                for yy in y..y + h {
                    for xx in x..x + w {
                        self.carve(Pos { x: xx, y: yy });
                    }
                }
                let p = Pos {
                    x: x + w / 2,
                    y: y + h / 2,
                };
                if let Some(prev) = centers.last().copied() {
                    self.corridor(prev, p);
                }
                centers.push(p);
                rooms.push((x, y, w, h));
            }
        }
        // Extra links turn the chain of rooms into loops, so there is more than
        // one way round a guard.
        let mut linked = false;
        for (a, b) in [(1, 4), (2, 5)] {
            if rng.range(0, 3) != 0 {
                self.corridor(centers[a], centers[b]);
                linked = true;
            }
        }
        if !linked {
            self.corridor(centers[1], centers[4]);
        }
        // Roles: the lift in a corner room, the relay in the opposite corner, and
        // the archives in three of the other four. The spare room holds the cache.
        let lift_room = [0, 2, 3, 5][rng.range(0, 4) as usize];
        let relay_room = 5 - lift_room;
        let mut others: Vec<usize> = (0..6)
            .filter(|r| *r != lift_room && *r != relay_room)
            .collect();
        for i in (1..others.len()).rev() {
            others.swap(i, rng.range(0, i as i32 + 1) as usize);
        }
        let spare_room = others[3];
        self.player = centers[lift_room];
        self.lift = centers[lift_room];
        self.relay = centers[relay_room];
        for (i, &room) in others[..3].iter().enumerate() {
            self.archives.push(Archive {
                pos: centers[room],
                id: floor * 3 + i,
                recovered: false,
            });
        }
        // Deeper floors trade sentinels for tougher foes instead of adding to the crowd.
        let mut posts: Vec<usize> = others[..3].to_vec();
        posts.insert(0, spare_room);
        for &room in posts.iter().skip(floor) {
            self.enemies.push(Enemy {
                pos: centers[room].offset(2, 1),
                hp: EnemyKind::Sentinel.max_hp(floor),
                kind: EnemyKind::Sentinel,
            });
        }
        self.enemies.push(Enemy {
            pos: centers[relay_room].offset(2, 1),
            hp: EnemyKind::Sentinel.max_hp(floor) + 2,
            kind: EnemyKind::Sentinel,
        });
        // Hunters join from floor 2: one more on each deeper floor.
        for h in 0..floor {
            self.enemies.push(Enemy {
                pos: centers[others[h]].offset(-2, -1),
                hp: EnemyKind::Hunter.max_hp(floor),
                kind: EnemyKind::Hunter,
            });
        }
        // The Overseer guards the final relay.
        if floor == FLOORS - 1 {
            self.enemies.push(Enemy {
                pos: centers[relay_room].offset(-2, 1),
                hp: EnemyKind::Overseer.max_hp(floor),
                kind: EnemyKind::Overseer,
            });
        }
        // One cache per floor, always holding a module you do not have yet.
        let missing: Vec<Module> = Module::ALL
            .iter()
            .copied()
            .filter(|m| !self.owned.contains(m))
            .collect();
        if !missing.is_empty() {
            let module = missing[rng.range(0, missing.len() as i32) as usize];
            self.caches.push(Cache {
                pos: centers[spare_room],
                module,
                taken: false,
            });
        }
        self.terminal = Some(Terminal {
            pos: centers[lift_room].offset(-2, -1),
            state: TerminalState::Locked,
        });
        // Supplies sit in room corners, off the direct line between doors: two
        // power cells and a medkit per floor, never in the lift room.
        for kind in [
            PickupKind::PowerCell,
            PickupKind::PowerCell,
            PickupKind::Medkit,
        ] {
            for _ in 0..8 {
                let room = rng.range(0, 6) as usize;
                let (x, y, w, h) = rooms[room];
                let pos = Pos {
                    x: if rng.range(0, 2) == 0 {
                        x + 1
                    } else {
                        x + w - 2
                    },
                    y: if rng.range(0, 2) == 0 {
                        y + 1
                    } else {
                        y + h - 2
                    },
                };
                if room != lift_room && !self.pickups.iter().any(|p| p.pos == pos) {
                    self.pickups.push(Pickup {
                        pos,
                        kind,
                        taken: false,
                    });
                    break;
                }
            }
        }
        if floor == 0 {
            self.log(
                "arrival",
                "Surface lift reached. Recover three archives, restore the relay, and return here.",
            );
        } else {
            self.log(
                "arrival",
                &format!(
                    "Floor {} of {}: {}. Recover three archives, restore the relay, and return to the lift.",
                    floor + 1,
                    FLOORS,
                    FLOOR_NAMES[floor]
                ),
            );
        }
        self.update_visibility(7);
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
            Action::Fit(m) => self.fit(m),
            Action::Unfit(m) => self.unfit(m),
            Action::Answer(i) => self.answer(i),
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
            && r.floor == self.floor
            && r.owned == self.owned
            && r.loadout == self.loadout
            && r.records_found == self.records_found
            && r.kills == self.kills
            && r.standing == self.standing
            && r.recovered() == self.recovered()
            && r.terminal.as_ref().map(|t| t.state) == self.terminal.as_ref().map(|t| t.state)
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
            self.collect();
        }
        self.finish_turn();
    }
    /// Take whatever supply lies underfoot, unless you cannot carry more.
    fn collect(&mut self) {
        let Some(i) = self
            .pickups
            .iter()
            .position(|p| !p.taken && p.pos == self.player)
        else {
            return;
        };
        match self.pickups[i].kind {
            PickupKind::PowerCell if self.energy < MAX_ENERGY => {
                self.energy = (self.energy + CELL_POWER).min(MAX_ENERGY);
                self.log("pickup", "Power cell: +2 power.");
            }
            PickupKind::Medkit if self.medkits < MAX_MEDKITS => {
                self.medkits += 1;
                self.log("pickup", "Picked up a medkit.");
            }
            _ => {
                self.log("status", "You cannot carry more of that. It stays here.");
                return;
            }
        }
        self.pickups[i].taken = true;
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
    /// Fit an owned module into a free slot. Takes a turn.
    pub fn fit(&mut self, m: Module) {
        if self.outcome != Outcome::Exploring {
            return;
        }
        if !self.owned.contains(&m) || self.loadout.contains(&m) {
            return;
        }
        if self.loadout.len() >= self.slots() {
            self.log("status", "No free module slot. Unfit one first.");
            return;
        }
        self.record(Action::Fit(m));
        self.loadout.push(m);
        self.log("equip", &format!("Fitted the {}.", m.name()));
        self.finish_turn();
    }
    /// Remove a fitted module (it stays owned). Takes a turn.
    pub fn unfit(&mut self, m: Module) {
        if self.outcome != Outcome::Exploring || !self.loadout.contains(&m) {
            return;
        }
        self.record(Action::Unfit(m));
        self.loadout.retain(|x| *x != m);
        self.log("equip", &format!("Unfitted the {}.", m.name()));
        self.finish_turn();
    }
    fn descend(&mut self) {
        self.floor += 1;
        self.hp = (self.hp + 8).min(MAX_HP);
        self.medkits = (self.medkits + 1).min(MAX_MEDKITS);
        self.energy = START_ENERGY;
        self.turn += 1;
        let f = self.floor;
        self.build_floor(f);
        let slots = self.slots();
        for m in self.owned.clone() {
            if !self.loadout.contains(&m) && self.loadout.len() < slots {
                self.loadout.push(m);
            }
        }
        self.log(
            "descend",
            &format!(
                "Rest bay: +8 health, +1 medkit, power restored. {} module slots open.",
                slots
            ),
        );
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
            if !self.records_found.contains(&id) {
                self.records_found.push(id);
            }
            self.log(
                "archive",
                &format!("Damaged record. {}", damaged(RECORDS[id])),
            );
            self.log(
                "echo",
                "ECHO: I can reconstruct that record. Ask me what it says.",
            );
            if RECORD_AUTHORS[id] == Faction::Wardens {
                self.shift_standing(Faction::Wardens, 1);
            }
            self.finish_turn();
            return;
        }
        if let Some(i) = self
            .caches
            .iter()
            .position(|c| !c.taken && c.pos.distance(self.player) <= 1)
        {
            self.record(Action::Interact);
            self.caches[i].taken = true;
            let m = self.caches[i].module;
            if !self.owned.contains(&m) {
                self.owned.push(m);
            }
            let fitted = self.loadout.len() < self.slots();
            if fitted {
                self.loadout.push(m);
            }
            self.log(
                "cache",
                &format!(
                    "Recovered a {} from the cache{}",
                    m.name(),
                    if fitted {
                        " and fitted it."
                    } else {
                        ". Every slot is full: open Equipment (I) to swap."
                    }
                ),
            );
            self.finish_turn();
            return;
        }
        if self.terminal_in_reach() {
            let c = &CHALLENGES[self.floor];
            self.log(
                "terminal",
                &format!(
                    "Custodian terminal: {} 1) {}  2) {}  3) {}",
                    c.question, c.options[0], c.options[1], c.options[2]
                ),
            );
            return;
        }
        if self.player.distance(self.relay) <= 1 {
            if self.recovered() == 3 && !self.restored {
                self.record(Action::Interact);
                self.restored = true;
                self.log("relay", "Relay restored. Return to the lift.");
                self.shift_standing(Faction::Wardens, 1);
                self.shift_standing(Faction::Custodians, 1);
                self.finish_turn();
            } else if !self.restored {
                self.log("status", "Relay needs three recovered archive keys.");
            } else {
                self.log("status", "Relay is online. Return to the lift.");
            }
            return;
        }
        if self.player.distance(self.lift) <= 1 {
            if self.restored {
                self.record(Action::Interact);
                if self.floor + 1 < FLOORS {
                    self.descend();
                } else {
                    self.outcome = Outcome::Escaped;
                    self.log(
                        "escape",
                        "Signal transmitted. You escaped with the recovered evidence.",
                    );
                }
            } else {
                self.log("status", "Restore the relay before departure.");
            }
            return;
        }
        self.log(
            "status",
            "Nothing to interact with here. Stand beside an archive, cache, relay, or lift.",
        );
    }
    /// True when a terminal that still accepts an answer is within reach.
    pub fn terminal_in_reach(&self) -> bool {
        self.terminal
            .as_ref()
            .is_some_and(|t| t.state == TerminalState::Locked && t.pos.distance(self.player) <= 1)
    }
    /// Answer this floor's terminal. One attempt: the right answer recharges you
    /// and earns Custodian trust, the wrong one costs power and trust.
    pub fn answer(&mut self, choice: usize) {
        if self.outcome != Outcome::Exploring || choice >= 3 || !self.terminal_in_reach() {
            return;
        }
        self.record(Action::Answer(choice));
        let right = CHALLENGES[self.floor].answer == choice;
        if let Some(t) = &mut self.terminal {
            t.state = if right {
                TerminalState::Solved
            } else {
                TerminalState::Failed
            };
        }
        if right {
            self.energy = MAX_ENERGY;
            self.log(
                "terminal",
                "Terminal: response accepted. Power recharged to full.",
            );
            self.shift_standing(Faction::Custodians, 2);
        } else {
            self.energy = self.energy.saturating_sub(2);
            self.log(
                "terminal",
                "Terminal: response rejected. It drains 2 power and locks.",
            );
            self.shift_standing(Faction::Custodians, -1);
        }
        self.finish_turn();
    }
    /// ECHO has read the recovered records back: the journal can show them whole.
    /// Returns how many were newly reconstructed.
    pub fn decode_all(&mut self) -> usize {
        let new: Vec<usize> = self
            .records_found
            .iter()
            .copied()
            .filter(|id| !self.decoded.contains(id))
            .collect();
        self.decoded.extend(&new);
        if !new.is_empty() {
            self.log(
                "echo",
                &format!(
                    "ECHO reconstructed {} damaged record(s). Read them in the journal (J).",
                    new.len()
                ),
            );
        }
        new.len()
    }
    /// A record as the player can read it right now.
    pub fn record_text(&self, id: usize) -> String {
        if self.decoded.contains(&id) {
            RECORDS[id].to_string()
        } else {
            damaged(RECORDS[id])
        }
    }
    fn finish_turn(&mut self) {
        self.turn += 1;
        for i in 0..self.enemies.len() {
            let (p, kind) = (self.enemies[i].pos, self.enemies[i].kind);
            if p.distance(self.player) == 1 {
                let who = kind.name().to_lowercase();
                if self.has(Module::Shield) && self.energy > 0 {
                    self.energy -= 1;
                    self.log(
                        "shield",
                        &format!("Shield Cell absorbed a {who} strike (-1 power)."),
                    );
                    continue;
                }
                self.hp -= kind.damage();
                self.log(
                    "damage",
                    &format!("A {who} strikes you for {} damage.", kind.damage()),
                );
                if self.hp <= 0 {
                    self.hp = 0;
                    self.outcome = Outcome::Dead;
                    self.log(
                        "death",
                        "Expedition lost. Start a new signal or reload your save.",
                    );
                    break;
                }
            } else if kind.advances(self.turn)
                && p.distance(self.player) <= kind.sight()
                && self.line_clear(p, self.player)
            {
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
                "A foe is adjacent: bump into it to strike for 3. Sentinels hit for 1, Hunters and Overseers for 2.",
            );
        }
        if self.hp <= 10 && self.medkits > 0 {
            return Some("Health is low: press H to use a medkit (+10 HP, costs a turn).");
        }
        if self.enemies.iter().any(|e| self.can_see(e.pos)) {
            return Some(
                "Foe in sight (S sentinel, H hunter, O overseer). Fight in a corridor and strike first.",
            );
        }
        if self
            .archives
            .iter()
            .any(|a| !a.recovered && a.pos.distance(self.player) <= 1)
        {
            return Some("Press E beside an archive (A) to recover its record.");
        }
        if self.terminal_in_reach() {
            return Some(
                "Press E at the terminal (T). One attempt: ask ECHO what your damaged records say first.",
            );
        }
        if self
            .records_found
            .iter()
            .any(|id| !self.decoded.contains(id))
        {
            return Some(
                "That record is damaged. Click the ECHO panel and ask what it says to reconstruct it.",
            );
        }
        if self
            .caches
            .iter()
            .any(|c| !c.taken && c.pos.distance(self.player) <= 1)
        {
            return Some("Press E beside a cache (C) to take the module inside.");
        }
        if self.recovered() == 3 && !self.restored {
            return Some(if self.relay.distance(self.player) <= 1 {
                "Press E at the relay (R) to restore it."
            } else {
                "All three keys recovered. Head for the relay (R)."
            });
        }
        if self.restored {
            return Some(if self.floor + 1 < FLOORS {
                "Relay online. Return to the lift (L) and press E to descend."
            } else {
                "Relay online. Return to the lift (L) and press E to transmit."
            });
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
            Outcome::Escaped if self.kills >= 10 => "Custodian's Bane",
            Outcome::Escaped if wardens >= 6 => "Warden's Friend",
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
                format!("Turns {}   Foes disabled {}", self.turn, self.kills),
                format!(
                    "Floor {}/{}   Records {}/{}   Power left {}",
                    self.floor + 1,
                    FLOORS,
                    self.records_found.len(),
                    RECORDS.len(),
                    self.energy
                ),
                format!("Wardens {wardens:+}   Custodians {custodians:+}"),
                epilogue.into(),
            ],
        }
    }
    pub fn knowledge(&self) -> serde_json::Value {
        let mut ids = self.records_found.clone();
        ids.sort_unstable();
        let facts: Vec<&str> = ids.iter().map(|&id| RECORDS[id]).collect();
        let threats: Vec<_> = self
            .enemies
            .iter()
            .filter(|e| self.can_see(e.pos))
            .map(|e| serde_json::json!({"position":e.pos,"hp":e.hp,"type":e.kind.name().to_lowercase()}))
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
        serde_json::json!({"turn":self.turn,"known_routes":routes,"position":self.player,"hp":self.hp,"medkits":self.medkits,"power":self.energy,"loadout":self.loadout.iter().map(|m|m.name()).collect::<Vec<_>>(),"owned_modules":self.owned.iter().map(|m|m.name()).collect::<Vec<_>>(),"module_slots":self.slots(),"floor":{"number":self.floor+1,"of":FLOORS,"name":FLOOR_NAMES[self.floor]},"known_caches":self.caches.iter().filter(|c|!c.taken&&self.discovered(c.pos)).map(|c|c.pos).collect::<Vec<_>>(),"factions":Faction::ALL.iter().map(|f|serde_json::json!({"name":f.name(),"about":f.about(),"standing":self.standing_of(*f)})).collect::<Vec<_>>(),"keys_recovered":self.recovered(),"relay_restored":self.restored,"outcome":self.outcome,"discovered_records":facts,"visible_threats":threats,"known_archives":landmarks,"known_lift":self.lift,"known_relay":if self.discovered(self.relay){Some(self.relay)}else{None},"terminal":self.terminal.as_ref().filter(|t|self.discovered(t.pos)).map(|t|{let c=&CHALLENGES[self.floor];serde_json::json!({"position":t.pos,"state":t.state,"challenge":c.question,"options":c.options,"note":"One attempt. The answer is stated in a record from this floor; if no discovered record states it, say so."})}),"records_player_cannot_read_yet":self.records_found.iter().filter(|id|!self.decoded.contains(id)).count(),"known_supplies":self.pickups.iter().filter(|p|!p.taken&&self.discovered(p.pos)).map(|p|serde_json::json!({"kind":p.kind,"position":p.pos})).collect::<Vec<_>>(),"recent_events":self.events.iter().rev().take(16).collect::<Vec<_>>()})
    }
    /// Bring a save from an older schema up to date (missing fields default).
    pub fn migrate(&mut self) {
        if self.owned.is_empty() {
            self.owned = self.loadout.clone();
        }
        for m in self.loadout.clone() {
            if !self.owned.contains(&m) {
                self.owned.push(m);
            }
        }
        if self.records_found.is_empty() {
            let found: Vec<usize> = self
                .archives
                .iter()
                .filter(|a| a.recovered)
                .map(|a| a.id)
                .collect();
            self.records_found = found;
        }
        let slots = self.slots();
        self.loadout.truncate(slots);
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
            || self.hp > MAX_HP
            || self.medkits > MAX_MEDKITS
            || self.energy > MAX_ENERGY
            || self.enemies.len() > 12
            || self.floor >= FLOORS
            || self.archives.len() != 3
        {
            return Err("Invalid expedition values".into());
        }
        let mut ids = HashSet::new();
        let mut positions = HashSet::new();
        for a in &self.archives {
            if !(self.floor * 3..self.floor * 3 + 3).contains(&a.id)
                || !ids.insert(a.id)
                || !self.floor(a.pos)
            {
                return Err("Invalid archive".into());
            }
        }
        for e in &self.enemies {
            if !self.floor(e.pos)
                || e.pos == self.player
                || !positions.insert(e.pos)
                || e.hp <= 0
                || e.hp > 20
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
        if self.loadout.len() > self.slots()
            || (1..self.loadout.len()).any(|i| self.loadout[..i].contains(&self.loadout[i]))
            || self.owned.len() > Module::ALL.len()
            || (1..self.owned.len()).any(|i| self.owned[..i].contains(&self.owned[i]))
            || self.loadout.iter().any(|m| !self.owned.contains(m))
            || self.records_found.iter().any(|&id| id >= RECORDS.len())
            || (1..self.records_found.len())
                .any(|i| self.records_found[..i].contains(&self.records_found[i]))
            || self.caches.len() > 2
            || self.caches.iter().any(|c| !self.floor(c.pos))
            || self.pickups.len() > 8
            || self.pickups.iter().any(|p| !self.floor(p.pos))
            || self.terminal.as_ref().is_some_and(|t| !self.floor(t.pos))
            || self
                .decoded
                .iter()
                .any(|id| !self.records_found.contains(id))
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
            let mut g = Game::new(seed);
            for floor in 0..FLOORS {
                g.build_floor(floor);
                g.floor = floor;
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
                assert!(reached.contains(&g.relay), "seed {seed} floor {floor}");
                assert!(g.archives.iter().all(|a| reached.contains(&a.pos)));
                assert!(g.caches.iter().all(|c| reached.contains(&c.pos)));
                assert!(g.enemies.iter().all(|e| reached.contains(&e.pos)));
            }
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
        g.floor = FLOORS - 1;
        g.build_floor(FLOORS - 1);
        g.interact();
        assert_eq!(g.outcome, Outcome::Exploring);
        g.enemies.clear();
        for a in &mut g.archives {
            a.recovered = true;
        }
        g.player = g.lift;
        g.interact();
        assert_eq!(
            g.outcome,
            Outcome::Exploring,
            "the lift needs the relay first"
        );
        g.player = g.relay;
        g.interact();
        assert!(g.restored);
        g.player = g.lift;
        g.interact();
        assert_eq!(g.outcome, Outcome::Escaped);
        g.validate().unwrap();
    }
    #[test]
    fn the_lift_descends_and_carries_your_state_down() {
        let mut g = Game::new_with(5, &[Module::Shield]);
        g.enemies.clear();
        g.hp = 10;
        g.medkits = 3;
        g.kills = 2;
        g.standing = [2, -1];
        let first_map = g.tiles.clone();
        for i in 0..3 {
            g.player = g.archives[i].pos;
            g.interact();
        }
        g.player = g.relay;
        g.interact();
        g.player = g.lift;
        g.interact();
        assert_eq!((g.floor, g.outcome), (1, Outcome::Exploring));
        assert_eq!((g.hp, g.medkits, g.energy), (18, 4, START_ENERGY));
        assert_eq!((g.kills, g.standing.len()), (2, 2));
        assert!(g.standing[0] >= 2, "faction standing carries over");
        assert!(!g.restored && g.archives.iter().all(|a| !a.recovered));
        assert_ne!(g.tiles, first_map, "a new floor is a new map");
        assert_eq!(g.records_found.len(), 3, "records stay in the journal");
        assert!(g.archives.iter().all(|a| (3..6).contains(&a.id)));
        assert_eq!(g.slots(), 2);
        g.validate().unwrap();
    }
    #[test]
    fn foes_get_harder_with_depth() {
        let mut kinds = vec![];
        for floor in 0..FLOORS {
            let mut g = Game::new(21);
            g.build_floor(floor);
            let count = |k| g.enemies.iter().filter(|e| e.kind == k).count();
            kinds.push((
                count(EnemyKind::Sentinel),
                count(EnemyKind::Hunter),
                count(EnemyKind::Overseer),
                g.enemies[0].hp,
            ));
        }
        assert_eq!(kinds[0], (5, 0, 0, 6));
        assert_eq!(kinds[1], (4, 1, 0, 7));
        assert_eq!(kinds[2], (3, 2, 1, 8));
    }
    #[test]
    fn hunters_hit_harder_and_the_overseer_is_slow() {
        let mut g = adjacent_sentinel(7, &[]);
        g.enemies[0].kind = EnemyKind::Hunter;
        g.wait();
        assert_eq!(g.hp, MAX_HP - 2);
        let mut o = adjacent_sentinel(7, &[]);
        o.enemies.clear();
        let start = o.player;
        let far = start.offset(4, 0);
        assert!(o.floor(far));
        o.enemies.push(Enemy {
            pos: far,
            hp: 14,
            kind: EnemyKind::Overseer,
        });
        let before = o.enemies[0].pos.distance(o.player);
        o.wait();
        o.wait();
        let after = o.enemies[0].pos.distance(o.player);
        assert_eq!(
            before - after,
            1,
            "two turns, but it advances only every other one"
        );
    }
    #[test]
    fn caches_give_new_modules_and_slots_limit_what_is_fitted() {
        let mut g = Game::new_with(5, &[Module::Shield]);
        g.enemies.clear();
        assert_eq!(g.caches.len(), 1);
        let found = g.caches[0].module;
        assert_ne!(found, Module::Shield, "a cache never repeats what you own");
        g.player = g.caches[0].pos;
        g.interact();
        assert!(g.owned.contains(&found));
        assert!(
            !g.has(found),
            "floor 1 has a single slot, so it is carried, not fitted"
        );
        g.fit(found);
        assert!(!g.has(found), "no free slot until you unfit something");
        g.unfit(Module::Shield);
        g.fit(found);
        assert!(g.has(found) && !g.has(Module::Shield));
        assert!(g.owned.contains(&Module::Shield));
        g.validate().unwrap();
    }
    #[test]
    fn old_saves_migrate() {
        let mut g = Game::new(3);
        g.owned.clear();
        g.records_found.clear();
        g.archives[1].recovered = true;
        g.loadout = vec![Module::Scanner, Module::Shield];
        g.migrate();
        assert_eq!(g.loadout, vec![Module::Scanner], "slots for floor 1 is 1");
        assert!(g.owned.contains(&Module::Scanner) && g.owned.contains(&Module::Shield));
        assert_eq!(g.records_found, vec![1]);
        g.validate().unwrap();
    }
    fn adjacent_sentinel(seed: u64, loadout: &[Module]) -> Game {
        let mut g = Game::new_with(seed, loadout);
        g.enemies.clear();
        let start = g.player;
        g.enemies.push(Enemy {
            pos: start.offset(1, 0),
            hp: 6,
            kind: EnemyKind::Sentinel,
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
        g.floor = FLOORS - 1;
        g.build_floor(FLOORS - 1);
        g.enemies.clear();
        for a in &mut g.archives {
            a.recovered = true;
        }
        g.player = g.relay;
        g.interact();
        g.player = g.lift;
        g.interact();
        assert_eq!(g.summary().rank, "Silent Signal");
        g.kills = 10;
        assert_eq!(g.summary().rank, "Custodian's Bane");
    }
    #[test]
    fn damaged_records_hide_every_terminal_answer_until_echo_reads_them() {
        for c in &CHALLENGES {
            let shown = damaged(RECORDS[c.record]).to_lowercase();
            let key = c.options[c.answer].to_lowercase();
            let longest = key.split(' ').max_by_key(|w| w.len()).unwrap();
            assert!(!shown.contains(longest), "{shown} gives away {longest}");
            assert!(RECORDS[c.record].to_lowercase().contains(longest));
        }
        let mut g = Game::new(4);
        g.enemies.clear();
        g.player = g.archives[1].pos;
        g.interact();
        assert!(g.record_text(1).contains('#'));
        assert_eq!(g.decode_all(), 1);
        assert_eq!(g.record_text(1), RECORDS[1]);
        assert_eq!(g.decode_all(), 0);
        g.validate().unwrap();
    }
    #[test]
    fn a_terminal_takes_one_answer_and_pays_or_punishes() {
        let mut g = Game::new(4);
        g.enemies.clear();
        g.energy = 3;
        g.player = g.terminal.as_ref().unwrap().pos.offset(1, 0);
        g.answer(CHALLENGES[0].answer);
        assert_eq!(
            (g.energy, g.standing_of(Faction::Custodians)),
            (MAX_ENERGY, 2)
        );
        g.answer(1);
        assert_eq!(g.standing_of(Faction::Custodians), 2, "one attempt only");
        let mut w = Game::new(4);
        w.enemies.clear();
        w.player = w.terminal.as_ref().unwrap().pos.offset(1, 0);
        w.answer((CHALLENGES[0].answer + 1) % 3);
        assert_eq!(
            (w.energy, w.standing_of(Faction::Custodians)),
            (START_ENERGY - 2, -1)
        );
        // The challenge reaches ECHO, the answer key does not.
        let k = w.knowledge().to_string();
        assert!(k.contains("evacuation destination") && !k.contains("\"answer\""));
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
