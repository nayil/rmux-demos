# terminal-playwright-demo

<!-- rmux-demo-media:start -->
<p>
  <a href="https://github.com/Helvesec/rmux-demos/tree/main/terminal-playwright-demo">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="../assets/readme/demo-playwright-header-dark.svg">
      <img src="../assets/readme/demo-playwright-header.svg" alt="Playwright Demo" width="650">
    </picture>
  </a><br>
  <sub><em>≃ 1,495 lines</em></sub><br>
  <a href="https://rmux.io/#demo-playwright">
    <img src="../assets/readme/demo-playwright-preview.png" alt="Playwright Demo の動画を見る" width="720">
  </a>
</p>
<!-- rmux-demo-media:end -->

本物のターミナルアプリ向けの Playwright 風テストです。

デモは 2 つのターミナルを開きます。1 つはライブで動くテストランナー、もう 1 つは本物の rmux pane に描画された simulated web page です。

## 必要なもの

`rmux` が `PATH` から実行できる必要があります。

## 実行

```bash
cargo run -- check
cargo run -- smoke
cargo run
```

runner は `rmux` と入力し、`[ Run ]` をクリックし、terminal が quiet になるのを待って、次を期待します。

```text
Result: Hello rmux
```

## クリーンアップ

```bash
cargo run -- cleanup
```
