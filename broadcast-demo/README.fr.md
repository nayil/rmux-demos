# broadcast-demo

Une arene Ratatui pour plusieurs CLI IA.

L'application demarre des panes rmux caches pour Claude, Codex, Gemini, Vibe et Grok. Tu tapes un prompt en bas, et rmux l'envoie a tous les agents.

## Lancer

```bash
cargo run -- check
cargo run
```

## Controles

- Tape dans le prompt du bas.
- Appuie sur `Enter` pour broadcaster.
- Appuie sur `Esc` ou `Ctrl-C` pour quitter.

## Nettoyer

```bash
cargo run -- cleanup
```

Necessite `rmux`, `claude`, `codex`, `gemini`, `vibe` et `grok` dans le `PATH`.
