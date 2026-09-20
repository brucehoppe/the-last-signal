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
