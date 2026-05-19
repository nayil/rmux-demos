# broadcast-demo

<!-- rmux-demo-media:start -->
<p>
  <a href="https://github.com/Helvesec/rmux-demos/tree/main/broadcast-demo">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="../assets/readme/demo-broadcast-header-dark.svg">
      <img src="../assets/readme/demo-broadcast-header.svg" alt="Broadcast Demo" width="650">
    </picture>
  </a><br>
  <a href="https://rmux.io/#demo-broadcast">
    <img src="../assets/readme/demo-broadcast-preview.png" alt="Play Broadcast Demo video" width="720">
  </a>
</p>
<!-- rmux-demo-media:end -->

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
