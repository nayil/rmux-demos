# terminal-playwright-demo

Des tests Playwright-style pour de vraies applications terminal.

Le runner ouvre deux terminaux: un affiche la checklist de tests en live, l'autre affiche une page web simulee rendue dans un vrai pane rmux.

## Lancer

```bash
cargo run -- check
cargo run -- smoke
cargo run
```

Le runner tape `rmux`, clique sur `[ Run ]`, attend le quiet state, puis verifie:

```text
Result: Hello rmux
```

## Nettoyer

```bash
cargo run -- cleanup
```
