# web-claude-demo

ブラウザとターミナルを同じ rmux pane に接続します。

小さな WebSocket bridge を起動します。ブラウザでもターミナルでも入力でき、両方の表示が同期します。

## 実行

```bash
cargo run -- check
cargo run
```

開く URL:

```text
http://127.0.0.1:8080
```

同じ Wi-Fi の iPhone から見る場合は、マシンのローカル IP を使います。

## オプション

```bash
RMUX_WEB_CMD='claude || exec bash'
PORT=8080
```

## クリーンアップ

```bash
cargo run -- cleanup
```
