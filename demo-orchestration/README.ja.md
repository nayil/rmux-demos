# demo-orchestration

Claude が rmux 経由で他の AI agent を操作します。

ランチャーは Codex、Gemini、Grok、Claude の 4 つのターミナルウィンドウを開きます。Claude には、他の agent に入力を送り、pane を読むための rmux コンテキストが渡されます。

## 必要なもの

`rmux`, `claude`, `codex`, `gemini`, `grok` が `PATH` から実行できる必要があります。

## Safety Warning

> [!WARNING]
> For testing purposes, this demo starts AI CLIs with approval or sandbox bypass flags. Be careful with the commands you run, and only use this demo in directories you trust.

## Linux / macOS

```bash
./launch.sh check
./launch.sh
```

## Windows PowerShell

```powershell
.\launch.ps1 check
.\launch.ps1
```

## Claude で試す

```text
Send Hi to all agents
Read all agents and summarize what they answered
```

## クリーンアップ

Linux / macOS:

```bash
./launch.sh cleanup
```

Windows PowerShell:

```powershell
.\launch.ps1 cleanup
```
