# terminal-playwright-demo

本物のターミナルアプリ向けの Playwright 風テストです。

runner は 2 つのターミナルを開きます。片方はライブのテストチェックリスト、もう片方は本物の rmux pane に描画された模擬 Web ページです。

## 実行

```bash
cargo run -- check
cargo run -- smoke
cargo run
```

runner は `rmux` を入力し、`[ Run ]` をクリックし、quiet state を待ってから次を検証します。

```text
Result: Hello rmux
```

## クリーンアップ

```bash
cargo run -- cleanup
```
