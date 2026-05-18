# terminal-playwright-demo

Playwright-style testing for real terminal apps.

The runner opens two terminals: one shows the live test checklist, the other shows a simulated web page rendered in a real rmux pane.

## Run

```bash
cargo run -- check
cargo run -- smoke
cargo run
```

The runner types `rmux`, clicks `[ Run ]`, waits for quiet, then expects:

```text
Result: Hello rmux
```

## Cleanup

```bash
cargo run -- cleanup
```
