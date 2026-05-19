# demo-orchestration

<!-- rmux-demo-media:start -->
<p>
  <a href="https://github.com/Helvesec/rmux-demos/tree/main/demo-orchestration">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="../assets/readme/demo-orchestration-header-dark.svg">
      <img src="../assets/readme/demo-orchestration-header.svg" alt="Orchestration Demo" width="650">
    </picture>
  </a><br>
  <sub><em>≃ 514 lines</em></sub><br>
  <a href="https://rmux.io/#demo-orchestration">
    <img src="../assets/readme/demo-orchestration-preview.png" alt="播放 Orchestration Demo 视频" width="720">
  </a>
</p>
<!-- rmux-demo-media:end -->

Claude 通过 rmux 控制其他 AI agent。

启动器会打开四个终端窗口：Codex、Gemini、Grok 和 Claude。Claude 会拿到 rmux 上下文，用来向其他 agent 发送输入并读取它们的 pane。

## 要求

`PATH` 中必须可用 `rmux`、`claude`、`codex`、`gemini` 和 `grok`。

## Safety Warning

> [!WARNING]
> For testing purposes, this demo starts AI CLIs with approval or sandbox bypass flags. Be careful with the commands you run, and only use this demo in directories you trust.

## Linux 和 macOS

```bash
./launch.sh check
./launch.sh
```

## Windows PowerShell

```powershell
.\launch.ps1 check
.\launch.ps1
```

## 在 Claude 中尝试

```text
Send Hi to all agents
Read all agents and summarize what they answered
```

## 清理

Linux 和 macOS：

```bash
./launch.sh cleanup
```

Windows PowerShell：

```powershell
.\launch.ps1 cleanup
```
