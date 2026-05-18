# demo-orchestration

Claude controle d'autres agents IA via rmux.

Le launcher ouvre quatre fenetres terminal: Codex, Gemini, Grok et Claude. Claude recoit le contexte rmux necessaire pour envoyer du texte aux autres agents et lire leurs panes.

## Prerequis

`rmux`, `claude`, `codex`, `gemini` et `grok` doivent etre disponibles dans le `PATH`.

## Warning securite

> [!WARNING]
> For testing purposes, this demo starts AI CLIs with approval or sandbox bypass flags. Be careful with the commands you run, and only use this demo in directories you trust.

## Linux et macOS

```bash
./launch.sh check
./launch.sh
```

## Windows PowerShell

```powershell
.\launch.ps1 check
.\launch.ps1
```

## A essayer dans Claude

```text
Send Hi to all agents
Read all agents and summarize what they answered
```

## Nettoyage

Linux et macOS:

```bash
./launch.sh cleanup
```

Windows PowerShell:

```powershell
.\launch.ps1 cleanup
```
