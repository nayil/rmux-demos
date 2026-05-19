# mini-zellij

<!-- rmux-demo-media:start -->
<p>
  <a href="https://github.com/Helvesec/rmux-demos/tree/main/mini-zellij">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="../assets/readme/demo-zellij-header-dark.svg">
      <img src="../assets/readme/demo-zellij-header.svg" alt="Zellij Demo" width="650">
    </picture>
  </a><br>
  <sub><em>≃ 944 lines</em></sub><br>
  <a href="https://rmux.io/#demo-zellij">
    <img src="../assets/readme/demo-zellij-preview.png" alt="Play Zellij Demo video" width="720">
  </a>
</p>
<!-- rmux-demo-media:end -->

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
