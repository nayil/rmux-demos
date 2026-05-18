# terminal-playwright-demo

本物のターミナルアプリ向けの Playwright 風テストです。

デモは 2 つのターミナルを開きます。1 つはライブで動くテストランナー、もう 1 つは本物の rmux pane に描画された simulated web page です。

## 必要なもの

`rmux` が `PATH` から実行できる必要があります。

## 実行

```bash
cargo run -- check
cargo run -- smoke
cargo run
```

runner は `rmux` と入力し、`[ Run ]` をクリックし、terminal が quiet になるのを待って、次を期待します。

```text
Result: Hello rmux
```

## クリーンアップ

```bash
cargo run -- cleanup
```
