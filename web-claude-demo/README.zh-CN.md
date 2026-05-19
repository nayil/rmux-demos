# web-claude-demo

<!-- rmux-demo-media:start -->
<p>
  <a href="https://github.com/Helvesec/rmux-demos/tree/main/web-claude-demo">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="../assets/readme/demo-mirroring-header-dark.svg">
      <img src="../assets/readme/demo-mirroring-header.svg" alt="Mirroring Demo" width="650">
    </picture>
  </a><br>
  <a href="https://rmux.io/#demo-mirroring">
    <img src="../assets/readme/demo-mirroring-preview.png" alt="播放 Mirroring Demo 视频" width="720">
  </a>
</p>
<!-- rmux-demo-media:end -->

浏览器和终端连接到同一个 rmux pane。

demo 会启动一个小型 WebSocket bridge。在浏览器或终端中输入，两边都会保持同步。

## 要求

- `PATH` 中可用的 `rmux`
- `PATH` 中可用的 `claude`，或者用 `RMUX_WEB_CMD` 指定其他命令

## Safety Warning

> [!WARNING]
> For testing purposes, the default Claude command uses approval or sandbox bypass flags. Be careful with the commands you run, and only use this demo in directories you trust.

## 运行

```bash
cargo run -- check
cargo run
```

打开：

```text
http://127.0.0.1:8080
```

同一 Wi-Fi 上的手机可以使用这台机器的局域网 IP。

## 选项

```bash
RMUX_WEB_CMD='IS_DEMO=1 claude --dangerously-skip-permissions --permission-mode bypassPermissions || exec bash'
PORT=8080
```

## 清理

```bash
cargo run -- cleanup
```
