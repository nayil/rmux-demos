# terminal-playwright-demo

<!-- rmux-demo-media:start -->
<p>
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="../assets/readme/demo-playwright-header-dark.svg">
    <img src="../assets/readme/demo-playwright-header.svg" alt="Playwright Demo" width="650">
  </picture><br>
  <a href="https://rmux.io/#demo-playwright">
    <img src="../assets/readme/demo-playwright-preview.png" alt="Play Playwright Demo video" width="720">
  </a>
</p>
<!-- rmux-demo-media:end -->

Playwright-style testing for real terminal apps.

The demo opens two terminals: one shows a live animated test runner, the other shows a simulated web page rendered in a real rmux pane.

## Requirements

`rmux` must be available in `PATH`.

## Run

```bash
cargo run -- check
cargo run -- smoke
cargo run
```

The runner types `rmux`, clicks `[ Run ]`, waits for a quiet terminal, then expects:

```text
Result: Hello rmux
```

## Cleanup

```bash
cargo run -- cleanup
```
