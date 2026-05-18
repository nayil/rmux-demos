# web-claude-demo

Un navigateur et un terminal attaches au meme pane rmux.

La demo lance un petit bridge WebSocket. Tape dans le navigateur ou dans le terminal: les deux vues restent synchronisees.

## Lancer

```bash
cargo run -- check
cargo run
```

Ouvre:

```text
http://127.0.0.1:8080
```

Pour un iPhone sur le meme Wi-Fi, utilise l'IP locale de ta machine.

## Options

```bash
RMUX_WEB_CMD='claude || exec bash'
PORT=8080
```

## Nettoyer

```bash
cargo run -- cleanup
```
