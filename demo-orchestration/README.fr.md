# demo-orchestration

Claude controle d'autres agents IA via rmux.

Le launcher ouvre quatre terminaux: Codex, Gemini, Grok et Claude. Claude recoit le contexte necessaire pour envoyer de l'input aux autres agents et lire leurs panes.

## Lancer

```bash
./launch.sh check
./launch.sh
```

Dans la fenetre Claude, essaie:

```text
Send Hi to all agents
Read all agents and summarize what they answered
```

## Nettoyer

```bash
./launch.sh cleanup
```

Necessite `rmux`, `claude`, `codex`, `gemini` et `grok` dans le `PATH`.
