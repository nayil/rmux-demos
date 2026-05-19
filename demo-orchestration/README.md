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
    <img src="../assets/readme/demo-orchestration-preview.png" alt="Play Orchestration Demo video" width="720">
  </a>
</p>
<!-- rmux-demo-media:end -->

Claude controls other AI agents through rmux.

The launcher opens four terminal windows: Codex, Gemini, Grok, and Claude. Claude gets the rmux context it needs to send input to the other agents and read their panes.

## Requirements

`rmux`, `claude`, `codex`, `gemini`, and `grok` must be available in `PATH`.

## Safety Warning

> [!WARNING]
> For testing purposes, this demo starts AI CLIs with approval or sandbox bypass flags. Be careful with the commands you run, and only use this demo in directories you trust.

## Linux and macOS

```bash
./launch.sh check
./launch.sh
```

## Windows PowerShell

```powershell
.\launch.ps1 check
.\launch.ps1
```

## Try in Claude

```text
Send Hi to all agents
Read all agents and summarize what they answered
```

## Cleanup

Linux and macOS:

```bash
./launch.sh cleanup
```

Windows PowerShell:

```powershell
.\launch.ps1 cleanup
```
