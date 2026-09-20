# Validation and known limits

Date: 2026-09-20. Environment: Linux x86_64, rustc 1.98.1.

## Executed checks

- `cargo test --locked --all-targets`: 11 tests passed (10 library tests and 1 integration test).
- Generated map connectivity and state validity checked across 100 seeds.
- A test-only route planner completed 20 seeded expeditions through normal movement, combat, healing, interaction, and extraction. It uses full-map knowledge for testing and is never exposed to the companion.
- Checked line-of-sight occlusion and visibility of the wall itself.
- Checked that hidden records, unknown relay position, and hidden enemies are absent from companion knowledge.
- Checked repeated seed generation, relay/exit gating, and valid final outcomes.
- Checked save round-trip, overwrite, conversation persistence, and malformed-save rejection.
- Checked the actual local HTTP client against a loopback mock server: request path, JSON schema, non-streaming and non-thinking options, and response parsing.
- Checked malformed and empty model response rejection, plus cloud model-name rejection.
- `cargo clippy --locked --all-targets -- -D warnings`: passed.
- `cargo build --locked --release`: passed on Linux.
- Source formatted with rustfmt. ZIP integrity, internal document links, and per-file SHA-256 manifest checked during packaging.

## Not verified here

- No actual Ollama model was downloaded or run. The HTTP test uses a mock; it does not establish model quality, speed, or prompt adherence.
- No Windows build or Windows playthrough was executed. The package contains source and Windows instructions, not an EXE.
- The gameplay-pass screens (game, journal, terminal, transmit, equipment) were checked from window captures of the macOS desktop build via `--preview`; no full interactive play session or real-model conversation was completed in this environment. Graphics, keyboard focus, chat layout, high-DPI scaling, and actual window behaviour still need laptop testing.
- No commercial-release, accessibility, or long-session balance certification.

## Known prototype limitations

- Single floor, fixed three-record story, five simple enemies, one save slot.
- Greedy enemy pursuit can stall behind obstacles; it is not full pathfinding.
- 1280x800 virtual interface; resizing stretches its layout. Text uses simple fixed-character wrapping and the built-in font. Long words and non-Latin text need UI work.
- Questions are limited to 140 characters. Stored replies are capped at 1200 characters. Requests include eight recent chat entries at up to 600 characters each, not all saved history.
- No voice, streaming tokens, request cancellation button, tool actions, embeddings, or autonomous NPC control.
- Closing the window does not auto-save. Press F5 after a reply to preserve it.
- Generation uses an older snapshot if you keep moving while ECHO thinks; its reply status identifies the snapshot turn.
- Local AI can invent claims. The journal and game state remain authoritative; dialogue never directly changes them.
- Older save formats require future migration work when the schema evolves.

## Laptop acceptance checklist

1. Build and start the game, then test all controls at your normal display scaling.
2. Complete a short expedition and verify both win and death screens.
3. Save, change state, reload, and confirm state and conversation restoration.
4. Ask a real model a question before and after recovering a record.
5. Confirm a hidden location is not present in the prompt snapshot.
6. Stop Ollama and check that failure leaves the game responsive.
7. Record model name, quantization, hardware, cold/warm latency, and any failures here.

## macOS acceptance run (2026-09-20)

- Hardware: Apple M5, 24 GB RAM, macOS. Model: `qwen3.5:9b` via local Ollama (set in an uncommitted `config.json`).
- Rendering bug found and fixed: the whole UI drew upside down because `Camera2D::from_display_rect` is y-up in macroquad 0.4. `src/main.rs` now flips the camera's y zoom. Mouse mapping was already manual and unaffected. The menu was confirmed upright in a real window.
- Real-model grounding, using the game's own `payload`/`request` (seed 42): before any discovery ECHO reported no evidence and said North Station's location is unknown; after recovering the Evacuation record it cited that record. Cold reply 9.1 s, warm 4.2 s. Replies ran on a background thread.
- Headless win: seeds 1-20 and 42 reach `Escaped` through the normal action API. Save/load is covered by unit tests.
- Played to a win in the real window (seed 42) by sending real keystrokes (Enter, WASD, E, H, F5, F9) with a route planner reading the F5 save: reached the SIGNAL RECEIVED screen at turn 155 with 3/3 keys and 18/24 HP. F5 saved ("Saved turn N" shown) and an F9 load mid-run resumed correctly. Some early scripted keystrokes were dropped, so an adaptive save-reading loop finished the run.
- Not done: Ollama-stopped failure path (not stopped, since it is the user's running service) and the death screen in the window.

## Performance and console run (2026-09-20)

Core hot paths, release build, before then after bounding-box visibility and LTO: `update_visibility(7)` 2.20 to 0.92 microseconds, `knowledge()` 66.7 to 52.0 microseconds, `Game::new` 4.3 to 2.0 microseconds. Native binary 2.65 to 1.81 MB; wasm 1.14 to 0.93 MB (359 KB gzipped, including the 270 KB font). In the window, frame time is vsync-bound at 16.7 ms and map drawing costs about 0.04 ms of CPU, so no render cache was added.

AI console against a real Ollama with 11 models: the list scrolls, per-model tests report score, latency, leak and recall, and "use selected" rewrites `config.json` atomically. `qwen3.5:9b` scored 100 (2.5 s). An earlier version wrongly flagged it as leaking because the detector matched the word "evacuation"; it now keys only on "North Station", which exists solely in the unrecovered record, and a regression test covers that. Models over 16 GB are skipped by "Test all" to avoid exhausting RAM.

## Three-floor balance (2026-09-20)

The planner test (`tests/expedition.rs`) plays whole expeditions through the normal action API with an omniscient route planner that charges every foe and heals below 12 HP. It is a floor for player skill, not a ceiling: it never uses corridors, scanning or refitting. With five sentinels on every floor plus hunters and the Overseer it won 22 of 200 and died mostly to being swarmed on floor 3. Deeper floors now trade sentinels for tougher foes (5, 4, 3 sentinels) and the rest bay restores 8 HP and a medkit; the planner then won 85 of 100 seeds. After the gameplay pass (guard awareness, tougher hunters and Overseer, lockdown hunters for a planner that disables everything, supplies on the floor) it wins 86 of 100, all deaths on floor 3. `difficulty_stays_in_band` fails if a change pushes that outside 70-95 wins or lets the planner die on floor 1. Every run, won or lost, must also replay exactly from its action log. The 200-seed run of the earlier tuning is not repeated in CI.

Screens checked in a real window: HUD on floor 2 (floor label, module costs, Analyzer ping revealing a distant archive), the Equipment screen and the paged journal. Fixes found that way: the key that opens Equipment also closed it in the same frame, and journal rows overlapped. Not hand-played end to end: floor 3 combat, the Overseer, and the end screen.

## Real-model check of the ECHO tasks

`cargo test --release --test real_model -- --ignored --nocapture` talks to the configured local model. On 2026-09-20 (macOS, Ollama, `qwen3.5:9b`, warm) it declined to answer the floor-1 terminal while the Evacuation record was unrecovered, quoted that record verbatim once it was recovered (3.9 s), and then answered the terminal from it (2.2 s). This is one model on one machine, not a benchmark; it is not run in CI.
