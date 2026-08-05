# TAPL 中文译本 Rust 配套实现

这里保存译本为原书实现章节提供的 Rust 参考实现。原书正文中的 OCaml
代码仍是翻译对象；Rust 工程是单独标明的译者增补，不替代原实现。

当前工作区包括：

- `arith`：对应第 3、4 章的布尔与算术表达式语言；
- `untyped`：对应第 5--7 章的纯无类型 lambda 演算；
- `simple`：对应第 8--14 章，覆盖简单类型、主要简单扩展、引用和异常；
- `book-snippets`：保存书稿中紧跟 OCaml 代码展示的 Rust 对照片段；这些
  片段属于工作区中的实际源码，会随整个工作区一起编译和测试。

工程固定使用 `rust-toolchain.toml` 指定的稳定版工具链，不依赖第三方
crate。从仓库根目录先进入本目录，再验证整个工作区：

```console
cd code
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

书稿不复制维护另一套 Rust 源码。`scripts/extract_rust_snippets.py` 会从
`book-snippets` 中带稳定标记的区域生成 LaTeX 所需片段，并检查每个片段
是否都在书稿中引用。

作者的原始 OCaml 程序和测试输入保存在
`../source/official-code/`，来源与校验值见该目录的 README。
