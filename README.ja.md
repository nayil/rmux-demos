# rmux デモ

rmux をプログラム可能なターミナルバックエンドとして見せる 5 つの小さなデモです。

`rmux` バイナリがインストールされ、`PATH` から実行できる必要があります。

## Safety Warning

> [!WARNING]
> For testing purposes, some demos start AI CLIs with approval or sandbox bypass flags. Be careful with the commands you run, and only use these demos in directories you trust.

## デモ

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
