# rmux 演示

这里有五个小演示，用来展示 rmux 作为可编程终端后端的能力。

请在每个演示目录内运行命令。`rmux` 二进制必须已安装，并且在 `PATH` 中可用。

## 演示

- `broadcast-demo`：把同一个 prompt 广播给多个 AI CLI。
- `mini-zellij`：一个由 rmux 驱动的轻量 Zellij 风格前端。
- `web-claude-demo`：浏览器和终端连接到同一个 rmux pane。
- `demo-orchestration`：Claude 通过 rmux 控制其他 agent。
- `terminal-playwright-demo`：面向终端应用的 Playwright 风格测试。

## 清理

大多数演示都支持清理命令：

```bash
cargo run -- cleanup
```

`demo-orchestration` 使用：

```bash
./launch.sh cleanup
```
