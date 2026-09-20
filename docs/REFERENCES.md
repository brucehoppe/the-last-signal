# Reference lessons and provenance

Reviewed during this project on 2026-09-20. These are references, not dependencies. No code or assets from these repositories were copied into this package. You do not need to download them to build The Last Signal.

## Inspected source snapshots

| Project | Source | Checked commit | Commit date | Lesson applied |
|---|---|---|---|---|
| RuggRogue | https://github.com/tung/ruggrogue | `164ff8380deb21964cbe65bd60db7c5cd7dbd107` | 2024-08-10 | Separated screen modes, saved state and bounded project scope. |
| Electric Organ | https://github.com/gridbugs/electric-organ | `6e5af917252d890e63b092c95fbdaec81d815af8` | 2026-01-28 | Game state separated from interface; typed messages feed companion context. |
| Boat Journey | https://github.com/gridbugs/boat-journey | `ec0629136efd42a6a581299a7f5210777b6ec18d` | 2023-05-27 | A companion has a natural role within a shared expedition. |

All three checked repositories identify their code as MIT-licensed. Their assets may have separate terms. This project does not bundle those assets or their source.

## Design and engineering references

- RuggRogue architecture guide: https://tung.github.io/ruggrogue/source-code-guide/
- Electric Organ developer description: https://www.gridbugs.org/electric-organ/
- Boat Journey developer description: https://www.gridbugs.org/boat-journey/
- RustRoguelike: https://github.com/nsmryan/RustRoguelike — action logs, replay, layered rules; the README describes the larger game as unfinished. Used as an architectural idea source, not imported code.
- Cogmind: https://www.gridsagegames.com/cogmind/ — readable tactical information and meaningful equipment changes. Commercial design reference, not a source or asset donor.
- Caves of Qud: https://cavesofqud.com/ — world history and relationships. Commercial design reference; the prototype does not implement its breadth of systems.
- Shattered Pixel Dungeon: https://github.com/00-Evan/shattered-pixel-dungeon — a reference for finished-game completeness and onboarding. Java/libGDX, not part of this Rust implementation.
- Brogue CE: https://github.com/tmewett/BrogueCE — tactical dungeon decisions and clear objectives; not imported.
- Original Rust/tcod tutorial: https://tomassedovic.github.io/roguelike-tutorial/ — baseline genre mechanics, not the dependency stack used here.

## Actual dependencies and official documentation

- Macroquad 0.4.16: https://docs.rs/macroquad/0.4.16/macroquad/
- ureq 2.12.1: https://docs.rs/ureq/2.12.1/ureq/ — deliberately uses the reviewed 2.x HTTP API, with TLS and compression disabled because this client uses loopback HTTP only.
- Serde: https://serde.rs/
- serde_json: https://docs.rs/serde_json/
- tempfile: https://docs.rs/tempfile/
- Ollama chat API: https://docs.ollama.com/api/chat
- Ollama structured outputs: https://docs.ollama.com/capabilities/structured-outputs
- Ollama local-only setting: https://docs.ollama.com/faq
- Qwen3 4B model tag: https://ollama.com/library/qwen3:4b

Cargo.lock records the resolved dependency versions for this build. Model weights and the Ollama runtime are separately installed and have their own licenses. No claim is made that a reference game's source currently compiles on the user's laptop; this package tests its own original source.

## Lessons carried forward: status

| Reference | Lesson | Implementation |
|---|---|---|
| RuggRogue | Understandable screens and systems | Separate game, journal, loadout, AI console, pause and end screens |
| Electric Organ | Equipment changes how you play | Two-slot loadout of Scanner, Shield and Analyzer sharing one power pool, with costs shown on the HUD |
| Boat Journey | The companion contributes | ECHO discusses only discovered facts, factions and loadout; a built-in script does the same in the browser demo |
| RustRoguelike | Traceable, replayable actions | `Action` log plus seed and loadout; `Game::replay_matches` proves a run reproduces exactly, tested on every winning seed |
| Cogmind | Clear tactical information | Power costs on the HUD, sentinel HP bars, and a danger halo on tiles a sentinel can strike |
| Caves of Qud | Relationships and history | Wardens and Custodians, record authorship, standing that reacts to play, and an epilogue that reflects it |

## Five additional ideas added in this pass

| Idea | Inspiration | Implementation |
|---|---|---|
| Contextual onboarding | Shattered Pixel Dungeon | `Game::hint()`, derived from state so it cannot drift from the rules |
| Run summary with a rank | Brogue CE / DCSS end-of-run screens | `Game::summary()`: Silent Signal, Warden's Friend, Custodian's Bane, Signal Bearer, Lost Signal |
| Threat legibility | Brogue CE, Cogmind | Halo and HP bars on visible sentinels |
| Model choice you can trust | Requested by the player | AI console: discovers installed models, scores them on grounding, leakage, schema, speed and brevity, saves the choice atomically. Cloud-named models are never offered |
| Readable text | Accessibility | Bundled JetBrains Mono drawn at the real pixel size, sharp on high-DPI screens and in the browser |

Font: JetBrains Mono, Copyright 2020 The JetBrains Mono Project Authors, SIL Open Font License 1.1 (`assets/fonts/OFL.txt`).
