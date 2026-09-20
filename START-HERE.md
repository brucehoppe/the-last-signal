# Start here: laptop setup

This ZIP contains the source project. It does not contain a Windows executable, Rust compiler, Ollama runtime, or model weights. Extract the entire ZIP before running anything. Keep Cargo.toml, Cargo.lock and src together.

## 1. Install Rust on Windows

1. Download rustup from https://rustup.rs/ . Use the default Windows MSVC toolchain.
2. If prompted for Microsoft build tools, install Visual Studio Build Tools with **Desktop development with C++**, including the Windows SDK. https://visualstudio.microsoft.com/visual-cpp-build-tools/
3. Close and reopen your terminal so it picks up PATH changes.
4. Check `cargo --version`. This project pins Rust 1.98.1 through rust-toolchain.toml; rustup downloads it when first building the project if needed.

You do not need SDL2: this new implementation uses Macroquad rather than the reference games' renderers.

## 2. Play before configuring the model

Open the extracted `the-last-signal` folder in VS Code. Open its terminal and run:

```powershell
cargo run --locked --release
```

Or double-click `run.cmd`. The first build downloads crates and takes longer. Subsequent runs reuse the compiled game. Internet is needed for the initial toolchain/dependency installation. After that, the built game itself does not need an internet connection.

Press Enter at the opening screen. Explore using WASD or arrow keys. Reach each `A` and press E from beside it. After collecting all three, reach `R` and press E. Return to `L` and press E to win. Sentinels are `S`; moving into one attacks it. Use H when injured. Each successful move, attack, heal, scan, archive recovery, or relay activation gives enemies a turn. Walls and unsuccessful interactions do not consume a turn.

## 3. Add the local companion

Install Ollama from https://ollama.com/download/windows and start it. In a terminal:

```powershell
ollama pull qwen3:4b
ollama list
```

The model download is separate and can be several gigabytes. The default model is a small initial candidate for a laptop; actual speed needs measurement on your machine. Memory capacity alone does not establish response speed.

Ollama normally starts its server automatically. If it is not running, use `ollama serve`. If this reports that the port is already in use, check whether Ollama is already serving; do not run a second server.

For an explicitly local setup, set Ollama's `OLLAMA_NO_CLOUD=1` environment variable and restart Ollama. The game itself uses a fixed loopback address, rejects model names containing `cloud`, disables HTTP redirects, and never supplies a cloud URL. A localhost server can be configured by its operator to proxy elsewhere, so use the official local Ollama service with a downloaded model.

Click the game's chat box and ask: `What should I do first?`

After recovering an archive, ask: `What does our recovered evidence tell us?`

Then F5 to save, F9 to reload, and ask a follow-up. Recent conversation turns and recovered facts are sent again. This implements bounded conversation memory, not model training or unlimited recall.

## 4. Model configuration

Copy `config.example.json` to `config.json`. It contains:

```json
{"model":"qwen3:4b","port":11434,"timeout_seconds":90}
```

Change the model to the exact downloaded name from `ollama list`. Timeout must be between 5 and 180 seconds. Requests use `think:false` and a 4096-token context budget. Some other model families may need adapter changes. A model must follow the requested reply schema; invalid responses produce an error rather than silently becoming game state.

## 5. Saves

Default Windows save: `%LOCALAPPDATA%\the-last-signal\expedition.save.json`.

Linux/macOS use `$XDG_DATA_HOME/the-last-signal/expedition.save.json`, or `~/.local/share/the-last-signal/expedition.save.json` when XDG_DATA_HOME is unset. Override with the `LAST_SIGNAL_SAVE` environment variable if necessary. The game currently has one save slot. New Expedition does not overwrite that slot until you save. Loading while an AI request is active is disabled to prevent attaching a reply to the wrong expedition.

## 6. Troubleshooting

| Symptom | Next step |
|---|---|
| cargo not found | Reopen the terminal after installing Rust; verify rustup installation. |
| link.exe missing | Install Microsoft's C++ desktop build tools and Windows SDK. |
| Window starts but ECHO cannot connect | Start Ollama; check `ollama list` and the configured port. |
| HTTP 404 | Download the configured model and check its exact name. |
| Request times out | Try a smaller local model, close memory-heavy apps, or increase timeout up to 180 seconds. |
| Invalid JSON/schema reply | Retry; if persistent, use the default model and current Ollama. |
| Save failure | Check the save directory is writable and disk space is available. |
| Movement keys type letters | Click outside the chat box or press Escape to leave text entry. |
| Game window fails on Linux | Install your distribution's OpenGL/Mesa and X11 runtime libraries; use a graphical session. |

## 7. Development checks

Run `verify.cmd` on Windows, or:

```sh
cargo fmt -- --check
cargo test --locked --all-targets
cargo clippy --locked --all-targets -- -D warnings
```

Validation results for the creation environment are in docs/VALIDATION.md. Windows and real-model behaviour must be checked on the destination laptop.
