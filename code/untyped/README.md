# `tapl-untyped`

本 crate 对应 TAPL 第 5--7 章：

- de Bruijn 索引与命名上下文；
- 第 6.2 节的移位和替换；
- 图 5-3 的按值调用小步求值；
- 第 7 章实现中的上下文长度一致性检查、变量名提示和打印。

Rust 版本用 `Result` 报告负索引、上下文长度不一致等实现不变量错误；
“没有求值规则适用”则由 `Option` 表示。内部数据结构不保存原书 OCaml
实现中的源文件位置信息，但解析错误仍报告字节位置。

## 运行与验证

以下命令从仓库根目录开始执行；先进入 `code/`，使 Rust 自动采用该目录
`rust-toolchain.toml` 固定的 Rust 1.97.1。命令行程序支持作者测试文件
所用的自由变量声明 `x/;`：

```console
cd code
cargo run -p tapl-untyped -- ../source/official-code/extracted/untyped/test.f
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

也可以回到仓库根目录运行 `make rust-check`。当前 6 个自动化用例直接验证
作者 `untyped/test.f` 的解析、求值与打印，以及移位截点、捕获规避替换、
按值调用先求参数、负索引与上下文长度错误、未绑定变量解析错误。

## 与作者 OCaml 实现的差异

本 crate 对应书中的核心数据结构与形式语义，不是作者命令行工具的完整复刻：

- AST 不保存官方实现的 `info` 源位置；解析错误只报告字节偏移。
- 解析器不支持官方具体语法中的 `import`、嵌套块注释和符号标识符。
- `_` 在本 crate 中按普通标识符处理，因此其绑定可以被引用；官方实现把它
  用作匿名绑定子。
- 本 crate 拒绝重复的顶层自由变量声明；官方工具允许同名声明再次扩展
  上下文。
- 打印器始终显式保留应用与抽象的括号；官方工具依据结合性和优先级省略
  不必要的括号。
- Rust 用 `Result`/`Option` 表达不变量错误和“无规则适用”，而 OCaml
  实现使用异常。

这些差异不改变本 crate 所实现的移位、替换和按值调用规则，但会影响它接受
的输入文本与输出外观；因此不能把它当作作者解析器和打印器的逐字替代品。
