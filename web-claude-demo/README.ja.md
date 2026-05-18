# web-claude-demo

ブラウザとターミナルが同じ rmux pane に接続するデモです。

小さな WebSocket bridge を起動します。ブラウザまたはターミナルで入力すると、両方の表示が同期します。

## 必要なもの

- `PATH` から実行できる `rmux`
- `PATH` から実行できる `claude`、または別コマンドを指定する `RMUX_WEB_CMD`

## Safety Warning

> [!WARNING]
> For testing purposes, the default Claude command uses approval or sandbox bypass flags. Be careful with the commands you run, and only use this demo in directories you trust.

## 実行

```bash
cargo run -- check
cargo run
```

開く URL:

```text
http://127.0.0.1:8080
```

同じ Wi-Fi 上のスマートフォンからは、このマシンのローカル IP を使います。

## オプション

```bash
RMUX_WEB_CMD='IS_DEMO=1 claude --dangerously-skip-permissions --permission-mode bypassPermissions || exec bash'
PORT=8080
```

## クリーンアップ

```bash
cargo run -- cleanup
```
