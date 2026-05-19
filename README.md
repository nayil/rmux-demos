# rmux demos

Five small demos that show rmux as a programmable terminal backend.

The `rmux` binary must be installed and available in `PATH`.

## Safety Warning

> [!WARNING]
> For testing purposes, some demos start AI CLIs with approval or sandbox bypass flags. Be careful with the commands you run, and only use these demos in directories you trust.

## Watch the demos

<!-- rmux-demo-gallery:start -->
<!-- rmux-demo-gallery-item:start -->
<p>
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/readme/demo-orchestration-header-dark.svg">
    <img src="assets/readme/demo-orchestration-header.svg" alt="Orchestration Demo" width="650">
  </picture><br>
  <a href="https://rmux.io/demos/demo-orchestration.mp4">
    <img src="assets/readme/demo-orchestration-preview.png" alt="Play Orchestration Demo video" width="720">
  </a>
</p>
<!-- rmux-demo-gallery-item:end -->

<!-- rmux-demo-gallery-item:start -->
<p>
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/readme/demo-broadcast-header-dark.svg">
    <img src="assets/readme/demo-broadcast-header.svg" alt="Broadcast Demo" width="650">
  </picture><br>
  <a href="https://rmux.io/demos/demo-broadcast.mp4">
    <img src="assets/readme/demo-broadcast-preview.png" alt="Play Broadcast Demo video" width="720">
  </a>
</p>
<!-- rmux-demo-gallery-item:end -->

<!-- rmux-demo-gallery-item:start -->
<p>
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/readme/demo-zellij-header-dark.svg">
    <img src="assets/readme/demo-zellij-header.svg" alt="Zellij Demo" width="650">
  </picture><br>
  <a href="https://rmux.io/demos/demo-zellij.mp4">
    <img src="assets/readme/demo-zellij-preview.png" alt="Play Zellij Demo video" width="720">
  </a>
</p>
<!-- rmux-demo-gallery-item:end -->

<!-- rmux-demo-gallery-item:start -->
<p>
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/readme/demo-mirroring-header-dark.svg">
    <img src="assets/readme/demo-mirroring-header.svg" alt="Mirroring Demo" width="650">
  </picture><br>
  <a href="https://rmux.io/demos/demo-mirroring.mp4">
    <img src="assets/readme/demo-mirroring-preview.png" alt="Play Mirroring Demo video" width="720">
  </a>
</p>
<!-- rmux-demo-gallery-item:end -->

<!-- rmux-demo-gallery-item:start -->
<p>
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/readme/demo-playwright-header-dark.svg">
    <img src="assets/readme/demo-playwright-header.svg" alt="Playwright Demo" width="650">
  </picture><br>
  <a href="https://rmux.io/demos/demo-playwright.mp4">
    <img src="assets/readme/demo-playwright-preview.png" alt="Play Playwright Demo video" width="720">
  </a>
</p>
<!-- rmux-demo-gallery-item:end -->
<!-- rmux-demo-gallery:end -->

## Demos

- `broadcast-demo`: one prompt races across multiple AI CLIs.
- `mini-zellij`: a tiny Zellij-style workspace powered by rmux.
- `web-claude-demo`: browser and terminal attached to the same pane.
- `demo-orchestration`: Claude controls Codex, Gemini, and Grok through rmux.
- `terminal-playwright-demo`: Playwright-style tests for terminal apps.

## Rust Demos

Run from the demo directory:

```bash
cargo run -- check
cargo run
cargo run -- cleanup
```

## Orchestration Demo

Linux and macOS:

```bash
cd demo-orchestration
./launch.sh check
./launch.sh
./launch.sh cleanup
```

Windows PowerShell:

```powershell
cd demo-orchestration
.\launch.ps1 check
.\launch.ps1
.\launch.ps1 cleanup
```
