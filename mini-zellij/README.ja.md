# mini-zellij

<!-- rmux-demo-media:start -->
<p>
  <a href="https://github.com/Helvesec/rmux-demos/tree/main/mini-zellij">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="../assets/readme/demo-zellij-header-dark.svg">
      <img src="../assets/readme/demo-zellij-header.svg" alt="Zellij Demo" width="650">
    </picture>
  </a><br>
  <a href="https://rmux.io/#demo-zellij">
    <img src="../assets/readme/demo-zellij-preview.png" alt="Zellij Demo の動画を見る" width="720">
  </a>
</p>
<!-- rmux-demo-media:end -->

rmux 上に作った小さな Zellij 風ターミナルワークスペースです。

UI は Ratatui です。pane は `ratatui-rmux` で描画される本物の rmux pane です。

## 必要なもの

`rmux` が `PATH` から実行できる必要があります。

## 実行

```bash
cargo run -- check
cargo run
```

## 操作

- pane をクリックして focus します。
- 入力は focus された pane に送られます。
- `Ctrl-b %` で縦 split。
- `Ctrl-b "` で横 split。
- `Ctrl-b d` で detach。
- もう一度 `cargo run` すると reattach します。
- `Ctrl-q` または `Ctrl-c` で終了してクリーンアップします。

## クリーンアップ

```bash
cargo run -- cleanup
```
