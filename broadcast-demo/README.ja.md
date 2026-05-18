# broadcast-demo

複数の AI CLI を並べる Ratatui のアリーナです。

Claude、Codex、Gemini、Vibe、Grok 用の隠れた rmux pane を起動します。下の prompt に入力すると、rmux が全 agent に送ります。

## 実行

```bash
cargo run -- check
cargo run
```

## 操作

- 下の prompt に入力します。
- `Enter` でブロードキャストします。
- `Esc` または `Ctrl-C` で終了します。

## クリーンアップ

```bash
cargo run -- cleanup
```

`rmux`、`claude`、`codex`、`gemini`、`vibe`、`grok` が `PATH` に必要です。
