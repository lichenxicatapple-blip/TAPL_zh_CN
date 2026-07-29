# `tapl-untyped`

本 crate 对应 TAPL 第 5--7 章：

- de Bruijn 索引与命名上下文；
- 第 6.2 节的移位和替换；
- 图 5-3 的按值调用小步求值；
- 第 7 章实现中的上下文长度一致性检查、变量名提示和打印。

Rust 版本用 `Result` 报告负索引、上下文长度不一致等实现不变量错误；
“没有求值规则适用”则由 `Option` 表示。内部数据结构不保存原书 OCaml
实现中的源文件位置信息，但解析错误仍报告字节位置。

命令行程序支持作者测试文件所用的自由变量声明 `x/;`：

```console
cargo run -p tapl-untyped -- ../source/official-code/extracted/untyped/test.f
```

