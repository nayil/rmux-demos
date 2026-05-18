# broadcast-demo

A Ratatui arena for AI CLIs.

The app creates five hidden rmux panes, then shows one clean prompt at the bottom. Press `Enter` and every selected agent races through the same prompt.

## Requirements

- `rmux` in `PATH`
- at least one supported AI CLI in `PATH`: `claude`, `codex`, `gemini`, `vibe`, or `grok`

If only one AI CLI is installed, the demo reuses it for all five panes.

## Safety Warning

> [!WARNING]
> For testing purposes, this demo may start AI CLIs with approval or sandbox bypass flags. Be careful with the commands you run, and only use this demo in directories you trust.

## Run

```bash
cargo run -- check
cargo run
```

## Controls

- Type in the bottom prompt.
- Press `Enter` to broadcast.
- Click a pane to target only that agent.
- Click the bottom prompt to return to broadcast mode.
- Press `Esc` or `Ctrl-C` to quit.

## Cleanup

```bash
cargo run -- cleanup
```
