# TAPL 中文翻译工程

本仓库用于逐章翻译 Benjamin C. Pierce 的 *Types and Programming
Languages*。项目规范见 `AGENTS.md`。

## 当前状态

- 已核验本地教材版本和文件性质。
- 已建立本地 PDF 页码与原书印刷页码映射。
- 已建立 LaTeX 工程骨架。
- 已使用可复现脚本切分全部 625 个 PDF 页面。
- 尚未开始任何章节的正文翻译。

## 关键文件

| 路径 | 用途 |
| --- | --- |
| `AGENTS.md` | 项目原则与协作规范 |
| `notes/source-inventory.md` | PDF 版本、元数据和质量风险 |
| `notes/page-map.md` | 本地 PDF 页码与原书印刷页码映射 |
| `notes/book-structure.md` | 分部、章节和前后置材料的全书关系 |
| `notes/progress.md` | 逐章翻译与人工确认状态 |
| `scripts/split_pdf.py` | 可复现的 PDF 切分与验证脚本 |
| `source/split/manifest.json` | 切分范围、页数和 SHA-256 清单 |
| `tex/main.tex` | LaTeX 主入口 |
| `tex/book-structure.tex` | 全书分部与章节顺序的唯一映射 |

## 复现 PDF 切分

安装脚本依赖后运行：

```sh
python3 -m pip install -r requirements.txt
make split
make verify-splits
```

`make split` 会核验原始 PDF 的 SHA-256、页数、章节首页标题和全书覆盖范围，
然后写出切分 PDF 与清单。`make verify-splits` 只读复核现有产物。

## 编译 LaTeX 骨架

安装带有中文支持的 TeX Live、XeLaTeX 和 latexmk 后运行：

```sh
make pdf
```

编译结果写入 `build/`。当前骨架不包含正文译文。

## Git 版本管理

仓库跟踪项目规范、LaTeX 源文件、脚本、页码映射、校对记录和切分清单。
原始教材 PDF、可重新生成的切分 PDF、临时文件和 LaTeX 编译产物默认不纳入
Git。切分文件继续保留在本地，并可通过 `make split` 重建。

每章建议使用独立提交记录翻译与校对修改；人工明确确认后可创建
`chapter-NN-approved` 格式的标签。
