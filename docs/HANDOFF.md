# Handoff for the laptop AI coding assistant

## Read first

README.md, START-HERE.md, docs/ARCHITECTURE.md, docs/DESIGN.md, docs/VALIDATION.md, then the four source modules. This is an original project called `the-last-signal`, not a fork of a reference game.

## Current objective

Establish a reliable local desktop build with real Ollama on the user's laptop. Preserve the original Rust game and make the local companion a useful part of discovery. Do not silently switch to a cloud model or replace the game with a web app.

## First session

1. Run cargo test --locked --all-targets.
2. Run cargo run --locked --release -- --seed 42.
3. Confirm movement, collision, journal, combat, extraction, and save/load.
4. Open Ollama and download the configured local model.
5. Ask a question before discovery and verify it acknowledges missing information.
6. Recover a record, ask what it means, save, reload, and ask a follow-up.
7. Shut down Ollama and submit a question. The game should remain responsive and show a useful error.
8. Record actual hardware and latency in docs/VALIDATION.md. Do not infer performance from RAM size alone.

## Working rules

- Keep authoritative rules in core.rs.
- Do not send the full Game or seed to the LLM; use knowledge().
- Do not run HTTP/model inference on the rendering thread.
- Treat model text as dialogue, not commands.
- Preserve versioned saves; add migrations for schema changes.
- Keep Cargo.lock and verify dependency updates individually.
- Use references to learn patterns; do not import whole projects unless deliberately chosen.
- Document any adapted code and preserve license notices.
- Report what was actually built/tested, and distinguish mocks from real-model tests.
- Do not claim a commercial-ready release from this prototype.

## Suggested next task

Improve the companion chat UI: width-aware wrapping, longer multiline input, keyboard focus indication, retry-last-question, and request cancellation. Keep the game usable while requests run. Add a small number of tests for actual failure risks, not implementation-mirroring tests.

## Future prompt

Implement a read-only known-route query. The route must be computed by Rust using discovered walkable tiles. Return no route when the target is undiscovered or disconnected in the known map. Give the companion the result with the snapshot turn. Keep route calculation separate from natural-language phrasing, and test that hidden cells are excluded.

## Completion definition for the initial project

A full short expedition can be played, won, lost, saved and resumed. A real local model can answer grounded questions without freezing the window. The user can move the source to their laptop and continue development with this handoff.
