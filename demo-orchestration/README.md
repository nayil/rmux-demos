# demo-orchestration

Claude controls other AI agents through rmux.

The launcher opens four terminal windows: Codex, Gemini, Grok, and Claude. Claude receives enough context to send input to the other agents and read their panes.

## Run

```bash
./launch.sh check
./launch.sh
```

In the Claude window, try:

```text
Send Hi to all agents
Read all agents and summarize what they answered
```

## Cleanup

```bash
./launch.sh cleanup
```

Requires `rmux`, `claude`, `codex`, `gemini`, and `grok` in `PATH`.
