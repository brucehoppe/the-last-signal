use crate::core::Game;
#[cfg(not(target_arch = "wasm32"))]
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
#[cfg(not(target_arch = "wasm32"))]
#[derive(Serialize, Deserialize)]
struct Envelope {
    version: u32,
    game: Game,
}
#[cfg(not(target_arch = "wasm32"))]
pub fn default_path() -> PathBuf {
    if let Some(p) = std::env::var_os("LAST_SIGNAL_SAVE") {
        return PathBuf::from(p);
    }
    let base = if cfg!(target_os = "windows") {
        std::env::var_os("LOCALAPPDATA").map(PathBuf::from)
    } else {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
    };
    base.unwrap_or_else(|| PathBuf::from("."))
        .join("the-last-signal/expedition.save.json")
}
#[cfg(not(target_arch = "wasm32"))]
pub fn write(game: &Game, path: &Path) -> Result<(), String> {
    game.validate()?;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    use std::io::Write;
    let mut tmp = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    serde_json::to_writer_pretty(
        &mut tmp,
        &Envelope {
            version: 4,
            game: game.clone(),
        },
    )
    .map_err(|e| e.to_string())?;
    tmp.flush().map_err(|e| e.to_string())?;
    tmp.as_file().sync_all().map_err(|e| e.to_string())?;
    tmp.persist(path).map_err(|e| e.to_string())?;
    Ok(())
}
#[cfg(not(target_arch = "wasm32"))]
pub fn read(path: &Path) -> Result<Game, String> {
    use std::io::Read;
    let file = std::fs::File::open(path).map_err(|e| format!("Cannot open save: {e}"))?;
    let mut bytes = vec![];
    file.take(5_000_001)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 5_000_000 {
        return Err("Save exceeds 5 MB".into());
    }
    let envelope: Envelope =
        serde_json::from_slice(&bytes).map_err(|e| format!("Invalid save: {e}"))?;
    if envelope.version < 4 {
        return Err(
            "Save from the three-floor prototype: the complex has grown. Start a new signal."
                .into(),
        );
    }
    if envelope.version > 4 {
        return Err("Unsupported save version".into());
    }
    let mut game = envelope.game;
    game.migrate();
    game.validate()?;
    if game.outcome != crate::core::Outcome::Exploring {
        return Err("That expedition is over. Start a new signal.".into());
    }
    Ok(game)
}
/// Where a save goes once it has been used up: loading it, or finishing the
/// run it belongs to, moves it here so it cannot be loaded again. One run, one
/// life; the file is kept, not deleted, in case you want to look at it.
#[cfg(not(target_arch = "wasm32"))]
pub fn retired_path(path: &Path) -> PathBuf {
    let mut name = path
        .file_name()
        .map(|n| n.to_os_string())
        .unwrap_or_default();
    name.push(".loaded");
    path.with_file_name(name)
}
/// Move a save out of the way after it has been loaded.
#[cfg(not(target_arch = "wasm32"))]
pub fn retire(path: &Path) -> Result<(), String> {
    std::fs::rename(path, retired_path(path)).map_err(|e| e.to_string())
}
/// A run just ended: if the save on disk belongs to it (same seed), retire it,
/// so death cannot be undone by reloading an earlier turn.
#[cfg(not(target_arch = "wasm32"))]
pub fn retire_if_same_run(path: &Path, seed: u64) -> bool {
    let same = std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .is_some_and(|v| v["game"]["seed"].as_u64() == Some(seed));
    same && retire(path).is_ok()
}
/// Write a morgue file beside the save: the recap plus the last events of
/// the run. Returns where it went.
#[cfg(not(target_arch = "wasm32"))]
pub fn write_morgue(game: &Game, save_path: &Path, date: &str) -> Result<PathBuf, String> {
    let dir = save_path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
        .join("morgue");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!("{date}-seed{}-turn{}.txt", game.seed, game.turn));
    let mut text = game.recap();
    text.push_str("\nLast events:\n");
    for e in game
        .events
        .iter()
        .rev()
        .take(40)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
    {
        text.push_str(&format!("{:04}  {}\n", e.turn, e.text));
    }
    std::fs::write(&path, text).map_err(|e| e.to_string())?;
    Ok(path)
}
#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn memory_survives_save_and_overwrite() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("save.json");
        let mut g = Game::new(3);
        g.add_chat("user", "Remember our expedition.");
        write(&g, &p).unwrap();
        g.add_chat("assistant", "We arrived by the surface lift.");
        write(&g, &p).unwrap();
        let loaded = read(&p).unwrap();
        assert_eq!(loaded.chat.len(), 2);
        assert_eq!(loaded.tiles, g.tiles);
        assert_eq!(loaded.seed, 3);
    }
    #[test]
    fn bad_save_is_rejected() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("bad.json");
        std::fs::write(&p, "{}").unwrap();
        assert!(read(&p).is_err());
        let mut g = Game::new(3);
        g.tiles.clear();
        assert!(write(&g, &p).is_err());
    }
    #[test]
    fn morgue_files_hold_the_recap_and_events() {
        let d = tempfile::tempdir().unwrap();
        let mut g = Game::new(3);
        g.step(1, 0);
        g.hp = 0;
        g.outcome = crate::core::Outcome::Dead;
        let p = write_morgue(&g, &d.path().join("save.json"), "2026-09-22").unwrap();
        let text = std::fs::read_to_string(&p).unwrap();
        assert!(p
            .to_string_lossy()
            .contains("morgue/2026-09-22-seed3-turn1.txt"));
        assert!(text.starts_with("THE LAST SIGNAL  |  seed 3"));
        assert!(text.contains("Surface lift reached"));
    }
    #[test]
    fn a_finished_run_cannot_be_loaded() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("save.json");
        let mut g = Game::new(3);
        g.hp = 0;
        g.outcome = crate::core::Outcome::Dead;
        write(&g, &p).unwrap();
        assert!(read(&p).unwrap_err().contains("over"));
    }
    #[test]
    fn a_loaded_save_is_retired_and_death_retires_its_own_run_only() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("save.json");
        write(&Game::new(3), &p).unwrap();
        retire(&p).unwrap();
        assert!(!p.exists());
        assert!(retired_path(&p).exists());
        assert!(read(&p).is_err());
        write(&Game::new(3), &p).unwrap();
        assert!(
            !retire_if_same_run(&p, 4),
            "another run's save is left alone"
        );
        assert!(p.exists());
        assert!(retire_if_same_run(&p, 3));
        assert!(!p.exists());
        assert!(!retire_if_same_run(&p, 3), "nothing left to retire");
    }
}

// The browser demo has no filesystem: saving is switched off, not faked.
#[cfg(target_arch = "wasm32")]
pub const UNAVAILABLE: &str = "Saving is disabled in the browser demo.";
#[cfg(target_arch = "wasm32")]
pub fn default_path() -> PathBuf {
    PathBuf::new()
}
#[cfg(target_arch = "wasm32")]
pub fn write(_: &Game, _: &Path) -> Result<(), String> {
    Err(UNAVAILABLE.into())
}
#[cfg(target_arch = "wasm32")]
pub fn read(_: &Path) -> Result<Game, String> {
    Err(UNAVAILABLE.into())
}
#[cfg(target_arch = "wasm32")]
pub fn retire(_: &Path) -> Result<(), String> {
    Err(UNAVAILABLE.into())
}
#[cfg(target_arch = "wasm32")]
pub fn retire_if_same_run(_: &Path, _: u64) -> bool {
    false
}
