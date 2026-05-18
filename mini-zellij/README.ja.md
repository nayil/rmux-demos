# mini-zellij

rmux 上に作った小さな Zellij 風ターミナルワークスペースです。

UI は Ratatui です。pane は `ratatui-rmux` で描画される本物の rmux pane です。

## 必要なもの

`rmux` が `PATH` から実行できる必要があります。

## 実行

```bash
cargo run -- check
cargo run
```

## 操作

- pane をクリックして focus します。
- 入力は focus された pane に送られます。
- `Ctrl-b %` で縦 split。
- `Ctrl-b "` で横 split。
- `Ctrl-b d` で detach。
- もう一度 `cargo run` すると reattach します。
- `Ctrl-q` または `Ctrl-c` で終了してクリーンアップします。

## クリーンアップ

```bash
cargo run -- cleanup
```
