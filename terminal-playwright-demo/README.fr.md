# terminal-playwright-demo

<!-- rmux-demo-media:start -->
<p>
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="../assets/readme/demo-playwright-header-dark.svg">
    <img src="../assets/readme/demo-playwright-header.svg" alt="Playwright Demo" width="650">
  </picture><br>
  <a href="https://rmux.io/#demo-playwright">
    <img src="../assets/readme/demo-playwright-preview.png" alt="Lire la video Playwright Demo" width="720">
  </a>
</p>
<!-- rmux-demo-media:end -->

Tests Playwright-style pour de vraies apps terminal.

La demo ouvre deux terminaux: un test runner anime en live, et une page web simulee rendue dans un vrai pane rmux.

## Prerequis

`rmux` doit etre disponible dans le `PATH`.

## Lancer

```bash
cargo run -- check
cargo run -- smoke
cargo run
```

Le runner tape `rmux`, clique `[ Run ]`, attend un terminal calme, puis verifie:

```text
Result: Hello rmux
```

## Nettoyage

```bash
cargo run -- cleanup
```
