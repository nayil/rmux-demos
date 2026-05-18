# mini-zellij

rmux 上に作った小さな Zellij 風ターミナルワークスペースです。

UI は Ratatui です。pane は本物の rmux pane で、`ratatui-rmux` で描画します。

## 実行

```bash
cargo run -- check
cargo run
```

## 操作

- pane をクリックしてフォーカスします。
- 入力はフォーカス中の pane に送られます。
- `Ctrl-b %` で縦分割します。
- `Ctrl-b "` で横分割します。
- `Ctrl-b d` でデタッチします。
- もう一度 `cargo run` で再接続します。
- `Ctrl-q` または `Ctrl-c` で終了して片付けます。

## クリーンアップ

```bash
cargo run -- cleanup
```
