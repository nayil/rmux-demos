# rmux 演示

这里有五个小演示，用来展示 rmux 作为可编程终端后端的能力。

`rmux` 二进制必须已安装，并且在 `PATH` 中可用。

## Safety Warning

> [!WARNING]
> For testing purposes, some demos start AI CLIs with approval or sandbox bypass flags. Be careful with the commands you run, and only use these demos in directories you trust.

## 观看演示

<!-- rmux-demo-gallery:start -->
<!-- rmux-demo-gallery-item:start -->
<p>
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/readme/demo-orchestration-header-dark.svg">
    <img src="assets/readme/demo-orchestration-header.svg" alt="Orchestration Demo" width="650">
  </picture><br>
  <a href="https://rmux.io/demos/demo-orchestration.mp4">
    <img src="assets/readme/demo-orchestration-preview.png" alt="播放 Orchestration Demo 视频" width="720">
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
    <img src="assets/readme/demo-broadcast-preview.png" alt="播放 Broadcast Demo 视频" width="720">
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
    <img src="assets/readme/demo-zellij-preview.png" alt="播放 Zellij Demo 视频" width="720">
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
    <img src="assets/readme/demo-mirroring-preview.png" alt="播放 Mirroring Demo 视频" width="720">
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
    <img src="assets/readme/demo-playwright-preview.png" alt="播放 Playwright Demo 视频" width="720">
  </a>
</p>
<!-- rmux-demo-gallery-item:end -->
<!-- rmux-demo-gallery:end -->

## 演示

- `broadcast-demo`：用一个 prompt 让多个 AI CLI 同时竞速。
- `mini-zellij`：一个由 rmux 驱动的小型 Zellij 风格工作区。
- `web-claude-demo`：浏览器和终端连接到同一个 pane。
- `demo-orchestration`：Claude 通过 rmux 控制 Codex、Gemini 和 Grok。
- `terminal-playwright-demo`：面向终端应用的 Playwright 风格测试。

## Rust 演示

在对应演示目录中运行：

```bash
cargo run -- check
cargo run
cargo run -- cleanup
```

## Orchestration 演示

Linux 和 macOS：

```bash
cd demo-orchestration
./launch.sh check
./launch.sh
./launch.sh cleanup
```

Windows PowerShell：

```powershell
cd demo-orchestration
.\launch.ps1 check
.\launch.ps1
.\launch.ps1 cleanup
```
