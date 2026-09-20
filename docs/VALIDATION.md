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
- No interactive graphical/visual QA session was completed in this environment. Graphics, keyboard focus, chat layout, high-DPI scaling, and actual window behaviour still need laptop testing.
- No commercial-release, accessibility, or long-session balance certification.

## Known prototype limitations

- Single floor, fixed three-record story, five simple enemies, one save slot.
- Greedy enemy pursuit can stall behind obstacles; it is not full pathfinding.
- 1280x800 virtual interface; resizing stretches its layout. Text uses simple fixed-character wrapping and the built-in font. Long words and non-Latin text need UI work.
- Questions are limited to 68 characters. Stored replies are capped at 1200 characters. Requests include eight recent chat entries at up to 600 characters each, not all saved history.
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
