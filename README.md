# rmux demos

Five small demos that show rmux as a programmable terminal backend.

The `rmux` binary must be installed and available in `PATH`.

## Safety Warning

> [!WARNING]
> For testing purposes, some demos start AI CLIs with approval or sandbox bypass flags. Be careful with the commands you run, and only use these demos in directories you trust.

## Demos

- `broadcast-demo`: one prompt races across multiple AI CLIs.
- `mini-zellij`: a tiny Zellij-style workspace powered by rmux.
- `web-claude-demo`: browser and terminal attached to the same pane.
- `demo-orchestration`: Claude controls Codex, Gemini, and Grok through rmux.
- `terminal-playwright-demo`: Playwright-style tests for terminal apps.

## Rust Demos

Run from the demo directory:

```bash
cargo run -- check
cargo run
cargo run -- cleanup
```

## Orchestration Demo

Linux and macOS:

```bash
cd demo-orchestration
./launch.sh check
./launch.sh
./launch.sh cleanup
```

Windows PowerShell:

```powershell
cd demo-orchestration
.\launch.ps1 check
.\launch.ps1
.\launch.ps1 cleanup
```
