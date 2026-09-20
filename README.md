# The Last Signal

**[Try it in your browser](https://brucehoppe.github.io/the-last-signal/)**: a WebAssembly build of the real game. ECHO is a built-in script there and saving is off; the desktop build talks to a real local model through Ollama.

An original Rust expedition roguelike with an optional local-LLM companion.

**Version 0.1.0: a playable prototype and development handoff, not a finished commercial game.**

You enter a silent research complex, recover three archive keys, restore a relay, and return to the lift. ECHO, an optional local AI companion, discusses evidence you have actually discovered. Combat, discovery, objectives, and rewards are enforced by Rust.

Start with **[START-HERE.md](START-HERE.md)**. No reference repositories need to be downloaded.

## Included

- Six connected procedural rooms with reproducible seeds.
- Wall-blocked field of view, explored-map memory, five sentinels, bump combat.
- Three recoverable records, an evidence journal, relay restoration and extraction.
- Health, medkits and scanner pulses; victory and defeat.
- An original graphical desktop interface drawn with code, with no external art assets.
- A real Ollama HTTP integration on a background thread, with JSON-schema responses, timeouts, bounded response size, and no remote fallback URL.
- A discovered-state context builder and saved conversation history.
- Versioned JSON saves written through a temporary file and atomic replacement.
- Tests for connectivity, visibility, knowledge filtering, outcomes, saves, and the local HTTP contract.

## Run

Install Rust, then from this folder:

```sh
cargo run --locked --release
```

For a repeatable expedition:

```sh
cargo run --locked --release -- --seed 42
```

On Windows, use `run.cmd` after installing the prerequisites in START-HERE.

## Browser demo

The same Rust game compiles to WebAssembly (`cargo build --release --target wasm32-unknown-unknown`, then serve `web/` with the `.wasm` beside `index.html`). On wasm, `ureq`, `tempfile` and disk saves are excluded, and ECHO answers from a rule-based script that reads only the discovered-state snapshot. GitHub Actions builds and publishes it on every push to `main`. `web/gl.js` is miniquad's JS loader, the same version as the crate in `Cargo.lock` (MIT/Apache-2.0).

## Local AI

Install and open Ollama, then download a local model:

```sh
ollama pull qwen3:4b
```

Click the chat box, type a question, and press Enter. The model is not included in this ZIP. You can play without Ollama. To change the model or port, copy `config.example.json` to `config.json`; configuration is reloaded for every request. Model speed and quality depend on hardware. This is a starting candidate, not a benchmarked best model.

## Controls

| Key | Action |
|---|---|
| Arrows / WASD | Move; bump a sentinel to attack |
| E | Interact from the same or an adjacent tile |
| H | Use a medkit: up to 10 HP, maximum 24 |
| F | Spend scanner energy to extend sight until the next turn |
| Space | Wait one turn |
| J | Open recovered evidence |
| F5 / F9 | Save / load |
| Escape | Unfocus chat, close journal, or open pause menu |
| Enter | Start/resume from menu, or submit focused chat |
| Mouse wheel over chat | Scroll conversation |

While typing in chat, movement keys enter text. Click outside the box or press Escape to return to movement. The game does not auto-save on closing. A save made during generation contains the player's question but not a reply that has not arrived yet; save again after the reply if you want it persisted.

## Documents

- [Laptop setup](START-HERE.md)
- [Design and roadmap](docs/DESIGN.md)
- [Architecture and local AI](docs/ARCHITECTURE.md)
- [Reference lessons and links](docs/REFERENCES.md)
- [AI coding handoff](docs/HANDOFF.md)
- [Validation and limitations](docs/VALIDATION.md)

The source is original to this package. Reference games supplied design and engineering lessons; their source code and assets are not bundled or copied. Original project code is MIT-licensed; dependencies retain their own licenses.
