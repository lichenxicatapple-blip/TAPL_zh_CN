# `tapl-arith`

本 crate 对应 TAPL 第 3、4 章：

- 图 3-1、3-2 的抽象语法和值；
- 小步求值规则；
- 练习 3.5.17 所述的大步求值；
- 第 4 章 OCaml 实现中的 `isnumericval`、`isval`、`eval1` 与 `eval`。

`step` 返回 `Option<Term>`，用 `None` 表示没有求值规则适用；这与原实现
抛出 `NoRuleApplies` 的宿主语言处理方式不同，但不改变对象语言的求值
关系。命令行程序接受一个文件，或从标准输入读取以分号分隔的表达式。

## 运行与验证

以下命令均从仓库根目录执行，工具链固定为 `rust-toolchain.toml` 中的
Rust 1.97.1：

```console
cargo run -p tapl-arith -- source/official-code/extracted/arith/test.f
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

也可以用 `make rust-check` 一次执行格式、Clippy 与测试检查。测试覆盖：

- 数值、布尔值和卡住项的值判断；
- 图 3-1、3-2 的小步规则与多步求值；
- 练习 3.5.17 的大步求值；
- 作者 `arith/test.f` 中的示例输入；
- 解析错误及没有求值规则适用的边界情况。

## 与作者 OCaml 实现的差异

本 crate 复现的是本章所需的抽象语法与求值关系，不追求复制官方工具的全部
具体语法和用户界面：

- Rust AST 省略了仅用于源位置诊断的 `info` 字段。
- 官方解析器还接受任意非负十进制整数和 `import`；本 crate 的解析器只把
  `0` 作为数值字面量，也不实现 `import`。
- 官方具体语法要求 `succ`、`pred`、`iszero` 的复合操作数加括号；本
  crate 同时接受无括号的前缀嵌套形式，如 `succ succ 0;`。
- 官方打印器把数值压缩为十进制；本 crate 保留构造形式，因此作者示例
  `succ (pred 0);` 会打印为 `succ (0)`，而不是 `1`。
- `step` 的 `None` 与官方实现的 `NoRuleApplies` 只是在宿主语言错误处理
  方式上不同，不改变对象语言的求值规则。

这些差异不影响书中抽象语义规则的对应关系，但意味着本命令行程序不是官方
解析器和打印器的逐字替代品。
