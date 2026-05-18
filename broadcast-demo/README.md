# broadcast-demo

A Ratatui arena for multiple AI CLIs.

The app starts hidden rmux panes for Claude, Codex, Gemini, Vibe, and Grok. You type one prompt at the bottom, and rmux broadcasts it to every agent.

## Run

```bash
cargo run -- check
cargo run
```

## Controls

- Type in the bottom prompt.
- Press `Enter` to broadcast.
- Press `Esc` or `Ctrl-C` to quit.

## Cleanup

```bash
cargo run -- cleanup
```

Requires `rmux`, `claude`, `codex`, `gemini`, `vibe`, and `grok` in `PATH`.
