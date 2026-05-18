# broadcast-demo

一个用于多个 AI CLI 的 Ratatui 竞技场。

应用会为 Claude、Codex、Gemini、Vibe 和 Grok 启动隐藏的 rmux pane。你在底部输入一个 prompt，rmux 会广播给所有 agent。

## 运行

```bash
cargo run -- check
cargo run
```

## 操作

- 在底部 prompt 输入内容。
- 按 `Enter` 广播。
- 按 `Esc` 或 `Ctrl-C` 退出。

## 清理

```bash
cargo run -- cleanup
```

需要 `rmux`、`claude`、`codex`、`gemini`、`vibe` 和 `grok` 在 `PATH` 中。
