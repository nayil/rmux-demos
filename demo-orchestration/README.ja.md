# demo-orchestration

Claude が rmux 経由で他の AI agent を操作します。

ランチャーは Codex、Gemini、Grok、Claude の 4 つのターミナルを開きます。Claude は他の agent に入力を送り、pane を読むためのコンテキストを持ちます。

## 実行

```bash
./launch.sh check
./launch.sh
```

Claude のウィンドウで試してください。

```text
Send Hi to all agents
Read all agents and summarize what they answered
```

## クリーンアップ

```bash
./launch.sh cleanup
```

`rmux`、`claude`、`codex`、`gemini`、`grok` が `PATH` に必要です。
