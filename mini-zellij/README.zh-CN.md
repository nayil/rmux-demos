# mini-zellij

一个基于 rmux 的轻量 Zellij 风格终端工作区。

界面使用 Ratatui。pane 是真实的 rmux pane，并通过 `ratatui-rmux` 渲染。

## 运行

```bash
cargo run -- check
cargo run
```

## 操作

- 点击 pane 来聚焦。
- 输入内容会发送到当前聚焦的 pane。
- `Ctrl-b %` 垂直分屏。
- `Ctrl-b "` 水平分屏。
- `Ctrl-b d` 分离会话。
- 再次运行 `cargo run` 可重新连接。
- `Ctrl-q` 或 `Ctrl-c` 退出并清理。

## 清理

```bash
cargo run -- cleanup
```
