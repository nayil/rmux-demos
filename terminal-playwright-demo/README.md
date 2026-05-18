# terminal-playwright-demo

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
