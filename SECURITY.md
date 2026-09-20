# Security policy

## Reporting a vulnerability

Please report suspected vulnerabilities privately through
[GitHub private vulnerability reporting](https://github.com/brucehoppe/the-last-signal/security/advisories/new)
rather than a public issue. This is a prototype maintained by one person; expect
a best-effort response.

## Scope and design notes

- The game talks only to a local Ollama server on `127.0.0.1`. It rejects model
  names containing `cloud`, disables HTTP redirects, bounds response size and
  time, and has no remote fallback URL.
- Model output is treated as dialogue, never as commands or game state.
- Saves are validated on load and written atomically.
- Do not put secrets in `config.json`; it is git-ignored and holds only a model
  name, port and timeout.
