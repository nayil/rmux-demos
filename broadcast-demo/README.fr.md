# broadcast-demo

Une arene Ratatui pour CLI IA.

L'app cree cinq panes rmux caches, puis affiche un prompt propre en bas. Appuie sur `Enter` et chaque agent selectionne recoit le meme prompt.

## Prerequis

- `rmux` dans le `PATH`
- au moins une CLI IA supportee dans le `PATH`: `claude`, `codex`, `gemini`, `vibe` ou `grok`

Si une seule CLI IA est installee, la demo la reutilise pour les cinq panes.

## Warning securite

> [!WARNING]
> For testing purposes, this demo may start AI CLIs with approval or sandbox bypass flags. Be careful with the commands you run, and only use this demo in directories you trust.

## Lancer

```bash
cargo run -- check
cargo run
```

## Controles

- Tape dans le prompt du bas.
- Appuie sur `Enter` pour broadcaster.
- Clique une pane pour cibler seulement cet agent.
- Clique le prompt du bas pour revenir au mode broadcast.
- Appuie sur `Esc` ou `Ctrl-C` pour quitter.

## Nettoyage

```bash
cargo run -- cleanup
```
