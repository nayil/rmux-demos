# web-claude-demo

A browser and a terminal attached to the same rmux pane.

The demo starts a small WebSocket bridge. Type in the browser or in the terminal: both views stay in sync.

## Run

```bash
cargo run -- check
cargo run
```

Open:

```text
http://127.0.0.1:8080
```

For an iPhone on the same Wi-Fi, use your machine's local IP.

## Options

```bash
RMUX_WEB_CMD='claude || exec bash'
PORT=8080
```

## Cleanup

```bash
cargo run -- cleanup
```
