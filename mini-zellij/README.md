# mini-zellij

A tiny Zellij-style terminal workspace built on rmux.

The UI is Ratatui. The panes are real rmux panes rendered through `ratatui-rmux`.

## Requirements

`rmux` must be available in `PATH`.

## Run

```bash
cargo run -- check
cargo run
```

## Controls

- Click a pane to focus it.
- Type to send input to the focused pane.
- `Ctrl-b %` splits vertically.
- `Ctrl-b "` splits horizontally.
- `Ctrl-b d` detaches.
- Run `cargo run` again to reattach.
- `Ctrl-q` or `Ctrl-c` quits and cleans up.

## Cleanup

```bash
cargo run -- cleanup
```
