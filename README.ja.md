# rmux デモ

rmux をプログラム可能なターミナルバックエンドとして見せる 5 つの小さなデモです。

各デモはそれぞれのディレクトリから実行してください。`rmux` バイナリがインストールされ、`PATH` から実行できる必要があります。

## デモ

- `broadcast-demo`: 1 つの prompt を複数の AI CLI に送ります。
- `mini-zellij`: rmux で動く小さな Zellij 風フロントエンドです。
- `web-claude-demo`: ブラウザとターミナルが同じ rmux pane に接続します。
- `demo-orchestration`: Claude が rmux 経由で他の agent を操作します。
- `terminal-playwright-demo`: ターミナルアプリ向けの Playwright 風テストです。

## クリーンアップ

多くのデモは次のコマンドで片付けられます。

```bash
cargo run -- cleanup
```

`demo-orchestration` では次を使います。

```bash
./launch.sh cleanup
```
