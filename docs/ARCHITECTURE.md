# Architecture and local AI

## Source map

| File | Responsibility |
|---|---|
| src/core.rs | Deterministic generation, state, turn actions, visibility, objectives, validation, discovered knowledge |
| src/ai.rs | Config, prompt construction, loopback HTTP, worker thread, schema validation |
| src/save.rs | Save location, version envelope, bounded reads, atomic writes, validation |
| src/main.rs | Macroquad desktop window, map, HUD, chat, journal, menus, input |
| src/lib.rs | Exposes the game services for headless tests |

The core does not call Macroquad or Ollama. The UI reads state and invokes explicit game actions. Rust determines every mechanical effect.

## Local inference lifecycle

1. Player submits a question.
2. Read and validate config.json, or use defaults.
3. Snapshot discovered knowledge and the last eight chat entries before appending the new question.
4. Send POST /api/chat on a background thread to 127.0.0.1, with a bounded timeout.
5. Poll the channel from the graphics loop; the window remains responsive.
6. Parse the Ollama envelope, then parse its JSON reply object.
7. Display the reply as advisory conversation, labelled by snapshot turn.
8. Persist conversation on the next explicit save.

Only one request may be active in the UI. Load and New Expedition are disabled while waiting. Movement remains allowed, so a reply may describe an older turn. There are no model-executable actions. Closing the application ends the process; there is no live cancellation button or streaming output in v0.1.

## Knowledge boundary

The model receives player status, visible enemies, discovered archive locations, recovered record text, the known lift, a relay location only after discovery, the last ten player-observable events, the terminal's challenge and options once the terminal has been seen, transmission options on the last floor, and recent chat. It does not receive the seed, the full map, hidden enemy locations, unrecovered record text, or any terminal's answer. If the request fails, the built-in script (`ai::demo_reply`) answers from the same snapshot.

Records persist independently of the event log. Conversation is capped at 100 entries, 1200 characters each. Eight recent entries, each truncated to 600 characters, are submitted. Player questions are limited to 68 characters by the compact two-line input widget. Event history is capped at 2000 entries. This is bounded episodic memory; there is no vector database, summary model, or training step.

The three public mission objectives are not treated as secrets. Their detailed recovered evidence is. The prompt tells ECHO to distinguish supplied facts from speculation, but this is behavioural guidance, not a guarantee. The model can still hallucinate. Critical evidence stays visible in the journal.

## Output and network controls

The server is constrained to reply JSON with a `reply` string. The client validates the field, rejects empty/invalid output, removes control characters, limits displayed text to 1200 characters and limits the HTTP response to 64 KiB. Unexpected keys do not execute anything. No game-state writes are driven by generated text.

The client uses HTTP on fixed 127.0.0.1, not a configurable remote host, and disables redirects. Model names containing `cloud` are rejected. Use downloaded Ollama models and configure Ollama for local-only mode. These client rules cannot audit the server operator's own routing configuration.

## Saves

Version 1 is an envelope containing the full serializable Game. Write to a temp file beside the destination, flush, sync, then persist over the destination. Validation checks map sizes, positions, record IDs, bounded statistics/history, and outcome consistency before accepting data. Read size is limited to 5 MB. Future schema changes need explicit migration code; do not silently interpret newer versions.

## Lessons used

RuggRogue: screen separation and saved state. Electric Organ: distinct game logic and structured messages. RustRoguelike: inspectable action consequences. Boat Journey: companion context tied to an expedition. Cogmind: a readable information panel. Qud: discovery and world history, reduced to a small authored evidence chain here.

## Next engineering improvements

Split the growing UI into modules; add input remapping, width-aware text wrapping and richer text entry; add route-query tools and captured snapshot fixtures; improve enemy pathfinding; add request cancellation and richer recovery; add schema migrations before changing saved structures. Keep each change playable and reviewable.
