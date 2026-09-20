# The Last Signal: design brief

## Identity

Working title: The Last Signal. No title/trademark availability research has been performed. The project is an original, compact science-fiction roguelike. The central loop is **explore, recover evidence, interpret it with ECHO, choose a next action**.

The player is an expedition operator inside an abandoned complex. ECHO is a damaged companion with access to the player's discoveries. The game's identity should come from consequential discoveries and an evolving relationship, with short readable dialogue.

## Implemented slice

Three floors of six rooms each, joined into loops. On every floor, three authored archive records unlock a relay; restore it, then return to the lift, which descends (or, on the last floor, offers a choice of transmission). Foes escalate: five sentinels on floor 1, then four plus a hunter, then three plus two hunters and an Overseer. Modules are found in caches, with one slot opening per floor. Health, medkits, and scanner energy supply limited resources. A seed reproduces the map. Victory and death both leave chat available for discussion.

The records and objective chain are fixed in version 0.1; geometry and room roles change by seed. The text of ECHO's messages does not change mechanics. Scanner pulses extend sight but do not penetrate walls, and expire on the next turn.

## Deliberate limits

Negotiating enemies, generated quests, NPC followers, voices, multiplayer, and model training are not implemented. Factions are two small standings, not a full reputation system. ECHO is a conversational companion, not a physical follower. Conversations are text only. Model wording can be inaccurate despite filtered facts and a schema.

## Added after the first baseline

Power-module loadouts with a shared power pool, two factions with standing and a ranked epilogue, an action log with deterministic replay, onboarding hints, threat overlays, a bundled legible font, and an AI console that benchmarks local models. See docs/REFERENCES.md for the lesson each one implements.

## Added in the gameplay pass

The first playable slice was a fixed chain of rooms with flat bump combat, an empty walk back to the lift, standing that only changed an epilogue, and a companion nobody needed. This pass addressed each:

- **ECHO has a job.** Records are recovered damaged; only ECHO holds the full text (`Game::record_text`, `Game::decode_all`). Terminals (`CHALLENGES`) test the player on those records, one attempt each. The panel offers suggested questions and draws `known_route` results on the map. The rule that model text never changes mechanics still holds: a reply arriving is what reconstructs records, not what the reply says, and the offline script (`ai::demo_reply`) answers when Ollama cannot. Terminal answers never enter `knowledge()`.
- **Each record explains something.** `ECHO_NOTES` gives the practical consequence of every record once it is read back (Milestone 2).
- **Guards think.** `Awareness` (Idle, Alert, Searching), last-seen pursuit with breadth-first pathing, posts to return to, a sentinel leash, hunter tracking, and alerts that spread within `SIGNAL_RANGE`.
- **Positioning pays.** Ambush damage on unready or stunned foes; looping floors; the Pulse Emitter.
- **Standing has teeth.** `custodian_response` stands units down or locks the floor down when a relay is restored.
- **The ending is a choice.** `Signal` options gated by trust and by optional data fragments, one of which recasts ECHO itself.
- **Reasons to replay.** A score and a shared daily seed.

The difficulty band test (`tests/expedition.rs`) was kept honest throughout: the omniscient, charge-everything planner wins 86 of 100 seeds. It never uses stealth, pulses, terminals or truces, so it is a floor for player skill.

## Roadmap

### Milestone 1: verify the baseline on the destination laptop

Build, play a full expedition, test a real local model, and save/reload conversation. Record model tag, hardware, context settings, cold-start latency, and warm latency. Fix usability before expanding.

### Milestone 2: deepen discovery

Add a richer authored evidence graph. Each recovered record should reveal an option, correct a belief, or explain a place. Store evidence IDs and provenance in game state. Add tests that unrecovered facts never enter prompts. Separate authored reliable facts from unreliable in-world testimony.

### Milestone 3: useful companion tools

Expose small read-only functions such as visible-threat summaries and known-route queries. Let Rust calculate routes; do not ask the LLM to invent directions. If tool execution is introduced, use a closed action schema and check current state before each action. Requests need request IDs, run IDs, and snapshot turns.

### Milestone 4: one distinctive resource choice (implemented as found-in-the-world power modules)

Introduce power modules with competing needs: scanner, shield, and analysis. Every choice needs visible costs and benefits. Avoid making model latency a gameplay penalty or requiring high-end hardware to win.

### Milestone 5: release quality

Accessibility, remappable controls, readable UI scaling, multiple save slots, durable save migrations, balanced encounters, audio settings, a complete onboarding path, and packaged executables. Consider an offline model setup assistant only after the runtime is stable. Review all dependency and future asset licenses before distribution.

## Acceptance criteria for a meaningful AI feature

- It uses actual discovered state.
- The player can tell facts from speculative dialogue.
- It changes what the player understands or can choose.
- It remains responsive if inference is slow or fails.
- It does not reveal hidden content through its prompt.
- Authoritative game state stays valid even if the model produces nonsense.

## Reference policy

Study the principle, express a requirement, write an original implementation, and test the behaviour. This package includes no reference-game code or art. If future work adapts source, track the exact origin and license in a separate notice file.
