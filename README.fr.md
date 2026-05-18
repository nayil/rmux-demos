# demos rmux

Cinq petites demos qui montrent rmux comme backend de terminaux programmable.

Lance chaque demo depuis son dossier. Le binaire `rmux` doit etre installe et disponible dans le `PATH`.

## Demos

- `broadcast-demo`: envoie un meme prompt a plusieurs CLI IA.
- `mini-zellij`: petit frontend type Zellij propulse par rmux.
- `web-claude-demo`: navigateur et terminal attaches au meme pane rmux.
- `demo-orchestration`: Claude controle d'autres agents via rmux.
- `terminal-playwright-demo`: tests Playwright-style pour applications terminal.

## Nettoyage

La plupart des demos ont une commande de nettoyage:

```bash
cargo run -- cleanup
```

Pour `demo-orchestration`:

```bash
./launch.sh cleanup
```
