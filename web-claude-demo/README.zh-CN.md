# web-claude-demo

浏览器和终端连接到同一个 rmux pane。

演示会启动一个小的 WebSocket bridge。你可以在浏览器或终端输入，两边会保持同步。

## 运行

```bash
cargo run -- check
cargo run
```

打开：

```text
http://127.0.0.1:8080
```

如果使用同一 Wi-Fi 上的 iPhone，请使用本机局域网 IP。

## 选项

```bash
RMUX_WEB_CMD='claude || exec bash'
PORT=8080
```

## 清理

```bash
cargo run -- cleanup
```
