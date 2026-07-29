# TAPL 中文译本 Rust 配套实现

这里保存译本为原书实现章节提供的 Rust 参考实现。原书正文中的 OCaml
代码仍是翻译对象；Rust 工程是单独标明的译者增补，不替代原实现。

当前工作区包括：

- `arith`：对应第 3、4 章的布尔与算术表达式语言；
- `untyped`：对应第 5--7 章的纯无类型 lambda 演算。

工程固定使用 `rust-toolchain.toml` 指定的稳定版工具链，不依赖第三方
crate。从仓库根目录先进入本目录，再验证整个工作区：

```console
cd code
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

作者的原始 OCaml 程序和测试输入保存在
`../source/official-code/`，来源与校验值见该目录的 README。
