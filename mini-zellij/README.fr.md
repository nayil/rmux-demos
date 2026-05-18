# mini-zellij

Un mini workspace terminal type Zellij construit sur rmux.

L'UI est en Ratatui. Les panes sont de vrais panes rmux rendus via `ratatui-rmux`.

## Prerequis

`rmux` doit etre disponible dans le `PATH`.

## Lancer

```bash
cargo run -- check
cargo run
```

## Controles

- Clique une pane pour la focus.
- Tape pour envoyer l'input a la pane focus.
- `Ctrl-b %` split vertical.
- `Ctrl-b "` split horizontal.
- `Ctrl-b d` detach.
- Relance `cargo run` pour reattach.
- `Ctrl-q` ou `Ctrl-c` quitte et nettoie.

## Nettoyage

```bash
cargo run -- cleanup
```
