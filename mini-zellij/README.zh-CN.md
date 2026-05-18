# mini-zellij

一个构建在 rmux 上的小型 Zellij 风格终端工作区。

UI 使用 Ratatui。pane 是通过 `ratatui-rmux` 渲染的真实 rmux pane。

## 要求

`PATH` 中必须可用 `rmux`。

## 运行

```bash
cargo run -- check
cargo run
```

## 控制

- 点击 pane 来聚焦。
- 输入会发送到当前聚焦的 pane。
- `Ctrl-b %` 垂直 split。
- `Ctrl-b "` 水平 split。
- `Ctrl-b d` detach。
- 再次运行 `cargo run` 可以 reattach。
- `Ctrl-q` 或 `Ctrl-c` 退出并清理。

## 清理

```bash
cargo run -- cleanup
```
