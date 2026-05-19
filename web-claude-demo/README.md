# web-claude-demo

<!-- rmux-demo-media:start -->
<p>
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="../assets/readme/demo-mirroring-header-dark.svg">
    <img src="../assets/readme/demo-mirroring-header.svg" alt="Mirroring Demo" width="650">
  </picture><br>
  <a href="https://rmux.io/demos/demo-mirroring.mp4">
    <img src="../assets/readme/demo-mirroring-preview.png" alt="Play Mirroring Demo video" width="720">
  </a>
</p>
<!-- rmux-demo-media:end -->

A browser and a terminal attached to the same rmux pane.

The demo starts a small WebSocket bridge. Type in the browser or in the terminal: both views stay in sync.

## Requirements

- `rmux` in `PATH`
- `claude` in `PATH`, or set `RMUX_WEB_CMD` to another command

## Safety Warning

> [!WARNING]
> For testing purposes, the default Claude command uses approval or sandbox bypass flags. Be careful with the commands you run, and only use this demo in directories you trust.

## Run

```bash
cargo run -- check
cargo run
```

Open:

```text
http://127.0.0.1:8080
```

For a phone on the same Wi-Fi, use your machine's local IP.

## Options

```bash
RMUX_WEB_CMD='IS_DEMO=1 claude --dangerously-skip-permissions --permission-mode bypassPermissions || exec bash'
PORT=8080
```

## Cleanup

```bash
cargo run -- cleanup
```
