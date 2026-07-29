# `tapl-arith`

本 crate 对应 TAPL 第 3、4 章：

- 图 3-1、3-2 的抽象语法和值；
- 小步求值规则；
- 练习 3.5.17 所述的大步求值；
- 第 4 章 OCaml 实现中的 `isnumericval`、`isval`、`eval1` 与 `eval`。

`step` 返回 `Option<Term>`，用 `None` 表示没有求值规则适用；这与原实现
抛出 `NoRuleApplies` 的宿主语言处理方式不同，但不改变对象语言的求值
关系。命令行程序接受一个文件，或从标准输入读取以分号分隔的表达式。

```console
cargo run -p tapl-arith -- ../source/official-code/extracted/arith/test.f
```

