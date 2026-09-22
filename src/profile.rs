//! What outlives a run: a history of finished expeditions, kept beside the
//! save on desktop and in the browser's local storage in the demo. It never
//! feeds back into the rules; it is a record, not a progression system.
use crate::core::{Difficulty, Game, Outcome, Signal, FLOORS, RECORDS};
use serde::{Deserialize, Serialize};

/// Seeds from here up are daily signals: seed = DAILY_BASE + days since 1970.
pub const DAILY_BASE: u64 = 20_000_000;
/// Only this many finished runs are remembered, newest last.
pub const MAX_RUNS: usize = 200;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Run {
    pub seed: u64,
    /// The day number when this was a daily signal.
    pub daily: Option<u64>,
    #[serde(default)]
    pub difficulty: Difficulty,
    pub rank: String,
    pub score: i32,
    pub won: bool,
    pub floor: usize,
    pub turns: u32,
    pub kills: u32,
    pub records: usize,
    pub signal: Option<Signal>,
    /// Calendar date, YYYY-MM-DD.
    pub when: String,
}
impl Run {
    pub fn line(&self) -> String {
        format!(
            "{}  {:<6}  {:>5}  {:<16}  floor {}/{}  {} turns  seed {}{}",
            self.when,
            self.difficulty.name().to_lowercase(),
            self.score,
            self.rank,
            self.floor + 1,
            FLOORS,
            self.turns,
            self.seed,
            if self.daily.is_some() { "  daily" } else { "" }
        )
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Profile {
    pub runs: Vec<Run>,
}
impl Profile {
    /// Note a finished run. Returns the record, or None while a run is unfinished.
    pub fn record(&mut self, g: &Game, when: &str) -> Option<Run> {
        if g.outcome == Outcome::Exploring {
            return None;
        }
        let run = Run {
            seed: g.seed,
            daily: daily_day(g.seed),
            difficulty: g.difficulty,
            rank: g.summary().rank.to_string(),
            score: g.score(),
            won: g.outcome == Outcome::Escaped,
            floor: g.floor,
            turns: g.turn,
            kills: g.kills,
            records: g.records_found.len().min(RECORDS.len()),
            signal: g.signal,
            when: when.to_string(),
        };
        self.runs.push(run.clone());
        if self.runs.len() > MAX_RUNS {
            self.runs.drain(..self.runs.len() - MAX_RUNS);
        }
        Some(run)
    }
    pub fn best(&self) -> Option<&Run> {
        self.runs.iter().max_by_key(|r| r.score)
    }
    pub fn wins(&self) -> usize {
        self.runs.iter().filter(|r| r.won).count()
    }
    /// The best attempt at a given day's signal.
    pub fn daily_result(&self, day: u64) -> Option<&Run> {
        self.runs
            .iter()
            .filter(|r| r.daily == Some(day))
            .max_by_key(|r| r.score)
    }
    /// One line for the title screen.
    pub fn headline(&self, today: u64) -> String {
        if self.runs.is_empty() {
            return "No expeditions recorded yet.".into();
        }
        let best = self.best().map_or(0, |r| r.score);
        let daily = match self.daily_result(today) {
            Some(r) => format!("{} ({})", r.score, r.rank),
            None => "not yet attempted".into(),
        };
        format!(
            "BEST {best}   RUNS {}   WINS {}   TODAY'S SIGNAL {daily}",
            self.runs.len(),
            self.wins()
        )
    }
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
    pub fn from_json(s: &str) -> Option<Self> {
        let mut p: Profile = serde_json::from_str(s).ok()?;
        p.runs.truncate(MAX_RUNS);
        Some(p)
    }
}
pub fn daily_day(seed: u64) -> Option<u64> {
    (DAILY_BASE..DAILY_BASE + 1_000_000)
        .contains(&seed)
        .then(|| seed - DAILY_BASE)
}
/// Days since 1970 for a clock reading in seconds.
pub fn day_of(epoch_seconds: f64) -> u64 {
    (epoch_seconds / 86_400.).max(0.) as u64
}
/// YYYY-MM-DD for a clock reading in seconds (proleptic Gregorian, UTC).
pub fn date_of(epoch_seconds: f64) -> String {
    let z = day_of(epoch_seconds) as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

#[cfg(not(target_arch = "wasm32"))]
pub fn default_path() -> std::path::PathBuf {
    crate::save::default_path().with_file_name("profile.json")
}
#[cfg(not(target_arch = "wasm32"))]
pub fn load() -> Profile {
    std::fs::read_to_string(default_path())
        .ok()
        .and_then(|s| Profile::from_json(&s))
        .unwrap_or_default()
}
#[cfg(not(target_arch = "wasm32"))]
pub fn store(p: &Profile) -> Result<(), String> {
    let path = default_path();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, p.to_json()).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())
}

// In the browser the profile lives in localStorage, through a tiny plugin in
// web/storage.js. If storage is blocked the demo simply forgets between visits.
#[cfg(target_arch = "wasm32")]
extern "C" {
    fn last_signal_store_get() -> sapp_jsutils::JsObject;
    fn last_signal_store_set(value: sapp_jsutils::JsObjectWeak);
}
#[cfg(target_arch = "wasm32")]
pub fn load() -> Profile {
    let object = unsafe { last_signal_store_get() };
    if object.is_nil() || object.is_undefined() {
        return Profile::default();
    }
    let mut s = String::new();
    object.to_string(&mut s);
    Profile::from_json(&s).unwrap_or_default()
}
#[cfg(target_arch = "wasm32")]
pub fn store(p: &Profile) -> Result<(), String> {
    let value = sapp_jsutils::JsObject::string(&p.to_json());
    unsafe { last_signal_store_set(value.weak()) };
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dates_and_days() {
        assert_eq!(date_of(0.), "1970-01-01");
        assert_eq!(date_of(951_782_400.), "2000-02-29");
        assert_eq!(date_of(1_790_000_000.), "2026-09-21");
        assert_eq!(day_of(86_400. * 3. + 5.), 3);
        assert_eq!(daily_day(DAILY_BASE + 20_000), Some(20_000));
        assert_eq!(daily_day(42), None);
    }
    #[test]
    fn finished_runs_are_recorded_and_round_trip() {
        let mut p = Profile::default();
        let mut g = Game::new(DAILY_BASE + 7);
        assert!(
            p.record(&g, "2026-09-22").is_none(),
            "unfinished runs are not recorded"
        );
        g.hp = 0;
        g.outcome = Outcome::Dead;
        let run = p.record(&g, "2026-09-22").unwrap();
        assert_eq!(run.daily, Some(7));
        assert!(!run.won);
        assert_eq!(p.daily_result(7).map(|r| r.score), Some(run.score));
        assert!(p.daily_result(8).is_none());
        let again = Profile::from_json(&p.to_json()).unwrap();
        assert_eq!(again, p);
        assert!(again.headline(7).contains("TODAY'S SIGNAL"));
        assert!(Profile::from_json("not json").is_none());
    }
    #[test]
    fn history_is_bounded() {
        let mut p = Profile::default();
        let mut g = Game::new(1);
        g.hp = 0;
        g.outcome = Outcome::Dead;
        for _ in 0..MAX_RUNS + 5 {
            p.record(&g, "2026-01-01");
        }
        assert_eq!(p.runs.len(), MAX_RUNS);
    }
}
