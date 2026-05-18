# terminal-playwright-demo

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
