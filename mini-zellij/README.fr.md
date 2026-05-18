# mini-zellij

Un petit workspace terminal style Zellij construit sur rmux.

L'interface est en Ratatui. Les panes sont de vrais panes rmux rendus via `ratatui-rmux`.

## Lancer

```bash
cargo run -- check
cargo run
```

## Controles

- Clique sur un pane pour le focus.
- Tape pour envoyer l'input au pane focus.
- `Ctrl-b %` split vertical.
- `Ctrl-b "` split horizontal.
- `Ctrl-b d` detache la session.
- Relance `cargo run` pour reattacher.
- `Ctrl-q` ou `Ctrl-c` quitte et nettoie.

## Nettoyer

```bash
cargo run -- cleanup
```
