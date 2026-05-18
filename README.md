# rmux demos

Five small demos that show rmux as a programmable terminal backend.

Run each demo from its own directory. The `rmux` binary must be installed and available in `PATH`.

## Demos

- `broadcast-demo`: broadcast one prompt to multiple AI CLIs.
- `mini-zellij`: a small Zellij-style frontend powered by rmux.
- `web-claude-demo`: browser and terminal attached to the same rmux pane.
- `demo-orchestration`: Claude controls other agents through rmux.
- `terminal-playwright-demo`: Playwright-style testing for terminal apps.

## Cleanup

Most demos provide a cleanup command:

```bash
cargo run -- cleanup
```

For `demo-orchestration`:

```bash
./launch.sh cleanup
```
