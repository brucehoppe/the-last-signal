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
            version: 3,
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
    if !(1..=3).contains(&envelope.version) {
        return Err("Unsupported save version".into());
    }
    let mut game = envelope.game;
    game.migrate();
    game.validate()?;
    Ok(game)
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
