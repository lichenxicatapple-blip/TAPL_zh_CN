# 第 8--14 章 Rust 参考实现

本 crate 是第二部分的可执行 Rust 配套实现。它覆盖：

- 第 8 章的布尔值、自然数、类型检查与求值；
- 第 9、10 章的简单类型 lambda 演算、上下文、抽象和应用；
- 第 11 章的 `Unit`、顺序执行、`let`、类型指称、积、记录、变体、
  一般递归与列表；
- 第 13 章的显式存储、分配、解引用、赋值与别名；
- 第 14 章的 `error`/`try` 和携带值的 `raise`/处理器。

实现采用具名变量和显式环境的大步解释器，而原书 OCaml 检查器使用
de Bruijn 索引和小步求值。这个选择让一份紧凑的 Rust 工程能够同时展示
闭包、存储和异常传播；形式规则、求值次序与可观察结果仍按相应章节实现。
它不是原书检查器具体语法的逐字符移植，也没有实现解析器、类型缩写和漂亮
打印器。

`Type::Bottom` 用来紧凑表示普通 `error` 的任意结果类型，与作者
`fullerror` 检查器的做法一致。携带值的异常使用抽象的 `Type::Exn`；
`Term::Exception` 表示把程序选择的具体表示封装进这个抽象类型。这是原书
没有提供官方 OCaml 实现的图 14-3 扩展。

从仓库根目录运行：

```console
cd code
cargo run -p tapl-simple
cargo test -p tapl-simple
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
```

测试覆盖正常求值、静态拒绝、闭包、记录、变体、列表、不动点、引用别名、
普通异常和求值步数上限。作者官方 `tyarith`、`simplebool`、`fullsimple`、
`fullref` 与 `fullerror` 快照保存在 `source/official-code/`，用于核对
行为；其中 `fullerror` 缺失的 `try` 求值分支已按作者勘误补全在本实现中。
