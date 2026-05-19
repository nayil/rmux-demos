# rmux デモ

rmux をプログラム可能なターミナルバックエンドとして見せる 5 つの小さなデモです。

`rmux` バイナリがインストールされ、`PATH` から実行できる必要があります。

## Safety Warning

> [!WARNING]
> For testing purposes, some demos start AI CLIs with approval or sandbox bypass flags. Be careful with the commands you run, and only use these demos in directories you trust.

## デモ

以下は、RMUX で構築できるものを示す短いデモです。RMUX はターミナルネイティブなワークフローの新しい可能性を開きます。特に*マルチエージェント編成*に向いています。まだ足りないデモが 1 つあります。それはあなたのプロジェクトです。RMUX で何かを作ったら、pull request でここに追加してください。

<!-- rmux-demo-gallery:start -->
<!-- rmux-demo-gallery-item:start -->
<p>
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/readme/demo-orchestration-header-dark.svg">
    <img src="assets/readme/demo-orchestration-header.svg" alt="Orchestration Demo" width="650">
  </picture><br>
  <a href="https://rmux.io/#demo-orchestration">
    <img src="assets/readme/demo-orchestration-preview.png" alt="Orchestration Demo の動画を見る" width="720">
  </a>
</p>
<!-- rmux-demo-gallery-item:end -->

<!-- rmux-demo-gallery-item:start -->
<p>
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/readme/demo-broadcast-header-dark.svg">
    <img src="assets/readme/demo-broadcast-header.svg" alt="Broadcast Demo" width="650">
  </picture><br>
  <a href="https://rmux.io/#demo-broadcast">
    <img src="assets/readme/demo-broadcast-preview.png" alt="Broadcast Demo の動画を見る" width="720">
  </a>
</p>
<!-- rmux-demo-gallery-item:end -->

<!-- rmux-demo-gallery-item:start -->
<p>
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/readme/demo-zellij-header-dark.svg">
    <img src="assets/readme/demo-zellij-header.svg" alt="Zellij Demo" width="650">
  </picture><br>
  <a href="https://rmux.io/#demo-zellij">
    <img src="assets/readme/demo-zellij-preview.png" alt="Zellij Demo の動画を見る" width="720">
  </a>
</p>
<!-- rmux-demo-gallery-item:end -->

<!-- rmux-demo-gallery-item:start -->
<p>
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/readme/demo-mirroring-header-dark.svg">
    <img src="assets/readme/demo-mirroring-header.svg" alt="Mirroring Demo" width="650">
  </picture><br>
  <a href="https://rmux.io/#demo-mirroring">
    <img src="assets/readme/demo-mirroring-preview.png" alt="Mirroring Demo の動画を見る" width="720">
  </a>
</p>
<!-- rmux-demo-gallery-item:end -->

<!-- rmux-demo-gallery-item:start -->
<p>
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/readme/demo-playwright-header-dark.svg">
    <img src="assets/readme/demo-playwright-header.svg" alt="Playwright Demo" width="650">
  </picture><br>
  <a href="https://rmux.io/#demo-playwright">
    <img src="assets/readme/demo-playwright-preview.png" alt="Playwright Demo の動画を見る" width="720">
  </a>
</p>
<!-- rmux-demo-gallery-item:end -->
<!-- rmux-demo-gallery:end -->

## デモディレクトリ

- `broadcast-demo`: 1 つの prompt で複数の AI CLI を競わせます。
- `mini-zellij`: rmux で動く小さな Zellij 風ワークスペースです。
- `web-claude-demo`: ブラウザとターミナルが同じ pane に接続します。
- `demo-orchestration`: Claude が rmux 経由で Codex、Gemini、Grok を操作します。
- `terminal-playwright-demo`: ターミナルアプリ向けの Playwright 風テストです。

## Rust デモ

各デモのディレクトリから実行します。

```bash
cargo run -- check
cargo run
cargo run -- cleanup
```

## Orchestration デモ

Linux / macOS:

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
