# broadcast-demo

<!-- rmux-demo-media:start -->
<p>
  <a href="https://github.com/Helvesec/rmux-demos/tree/main/broadcast-demo">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="../assets/readme/demo-broadcast-header-dark.svg">
      <img src="../assets/readme/demo-broadcast-header.svg" alt="Broadcast Demo" width="650">
    </picture>
  </a><br>
  <a href="https://rmux.io/#demo-broadcast">
    <img src="../assets/readme/demo-broadcast-preview.png" alt="播放 Broadcast Demo 视频" width="720">
  </a>
</p>
<!-- rmux-demo-media:end -->

一个面向 AI CLI 的 Ratatui 竞技场。

应用会创建五个隐藏的 rmux pane，并在底部显示一个干净的 prompt。按下 `Enter` 后，所有选中的 agent 都会收到同一个 prompt。

## 要求

- `PATH` 中可用的 `rmux`
- `PATH` 中至少有一个支持的 AI CLI：`claude`、`codex`、`gemini`、`vibe` 或 `grok`

如果只安装了一个 AI CLI，demo 会把它复用到五个 pane 中。

## Safety Warning

> [!WARNING]
> For testing purposes, this demo may start AI CLIs with approval or sandbox bypass flags. Be careful with the commands you run, and only use this demo in directories you trust.

## 运行

```bash
cargo run -- check
cargo run
```

## 控制

- 在底部 prompt 中输入。
- 按 `Enter` 广播。
- 点击某个 pane，只发送给该 agent。
- 点击底部 prompt，回到广播模式。
- 按 `Esc` 或 `Ctrl-C` 退出。

## 清理

```bash
cargo run -- cleanup
```
