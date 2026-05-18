# terminal-playwright-demo

面向真实终端应用的 Playwright 风格测试。

runner 会打开两个终端：一个显示实时测试清单，另一个显示在真实 rmux pane 中渲染的模拟网页。

## 运行

```bash
cargo run -- check
cargo run -- smoke
cargo run
```

runner 会输入 `rmux`，点击 `[ Run ]`，等待 quiet state，然后断言：

```text
Result: Hello rmux
```

## 清理

```bash
cargo run -- cleanup
```
