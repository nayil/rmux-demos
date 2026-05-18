# broadcast-demo

AI CLI のための Ratatui アリーナです。

このアプリは 5 つの隠れた rmux pane を作り、下部に 1 つの prompt を表示します。`Enter` を押すと、選択された agent が同じ prompt で走り始めます。

## 必要なもの

- `PATH` から実行できる `rmux`
- `PATH` から実行できる対応 AI CLI のいずれか: `claude`, `codex`, `gemini`, `vibe`, `grok`

AI CLI が 1 つだけでも、その CLI を 5 つの pane で再利用します。

## Safety Warning

> [!WARNING]
> For testing purposes, this demo may start AI CLIs with approval or sandbox bypass flags. Be careful with the commands you run, and only use this demo in directories you trust.

## 実行

```bash
cargo run -- check
cargo run
```

## 操作

- 下部の prompt に入力します。
- `Enter` で broadcast します。
- pane をクリックすると、その agent だけを対象にします。
- 下部の prompt をクリックすると broadcast mode に戻ります。
- `Esc` または `Ctrl-C` で終了します。

## クリーンアップ

```bash
cargo run -- cleanup
```
