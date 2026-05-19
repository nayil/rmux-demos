# mini-zellij

<!-- rmux-demo-media:start -->
<p>
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="../assets/readme/demo-zellij-header-dark.svg">
    <img src="../assets/readme/demo-zellij-header.svg" alt="Zellij Demo" width="650">
  </picture><br>
  <a href="https://rmux.io/#demo-zellij">
    <img src="../assets/readme/demo-zellij-preview.png" alt="Lire la video Zellij Demo" width="720">
  </a>
</p>
<!-- rmux-demo-media:end -->

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
