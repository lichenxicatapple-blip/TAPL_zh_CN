# 原文工作副本

根目录中的教材 PDF 是受保护的原始输入，不在这里复制或覆盖。

`split/` 中的 48 个 PDF 均由 `scripts/split_pdf.py` 生成：

- `frontmatter/`：转换目录、封底、版权页和前言；
- `parts/`：各分部页；
- `chapters/`：第 1-32 章；
- `appendices/`：附录 A 和附录 B；
- `backmatter/`：参考文献、索引和转换版本附带的插图目录。

页码边界定义在 `scripts/split_pdf.py` 中，机器可读清单和产物 SHA-256 见
`split/manifest.json`。不要手工修改生成的 PDF；需要调整时应修改脚本并
重新生成、验证。
