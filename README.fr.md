# demos rmux

Cinq petites demos qui montrent rmux comme backend de terminaux programmable.

Le binaire `rmux` doit etre installe et disponible dans le `PATH`.

## Warning securite

> [!WARNING]
> For testing purposes, some demos start AI CLIs with approval or sandbox bypass flags. Be careful with the commands you run, and only use these demos in directories you trust.

## Demos

- `broadcast-demo`: un prompt lance une course entre plusieurs CLI IA.
- `mini-zellij`: mini workspace type Zellij propulse par rmux.
- `web-claude-demo`: navigateur et terminal attaches au meme pane.
- `demo-orchestration`: Claude controle Codex, Gemini et Grok via rmux.
- `terminal-playwright-demo`: tests Playwright-style pour applications terminal.

## Demos Rust

Depuis le dossier de la demo:

```bash
cargo run -- check
cargo run
cargo run -- cleanup
```

## Demo orchestration

Linux et macOS:

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
