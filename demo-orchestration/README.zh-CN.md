# demo-orchestration

Claude 通过 rmux 控制其他 AI agent。

启动脚本会打开四个终端窗口：Codex、Gemini、Grok 和 Claude。Claude 会拿到必要上下文，用 rmux 给其他 agent 发送输入并读取它们的 pane。

## 运行

```bash
./launch.sh check
./launch.sh
```

在 Claude 窗口中尝试：

```text
Send Hi to all agents
Read all agents and summarize what they answered
```

## 清理

```bash
./launch.sh cleanup
```

需要 `rmux`、`claude`、`codex`、`gemini` 和 `grok` 在 `PATH` 中。
