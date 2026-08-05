# TAPL 中文翻译工程

本仓库用于逐章翻译 Benjamin C. Pierce 的 *Types and Programming
Languages*。项目规范见 `AGENTS.md`。

## 当前状态

- 原始教材的版本、页码映射和全部 625 个 PDF 页面的可复现切分已核验。
- 前言及第 1--7 章已经人类总审确认。
- 第 8--14 章已形成连续草稿并完成独立初审，正在等待人类总审。
- 第 15 章及以后尚未获准开始正文翻译。

## 关键文件

| 路径 | 用途 |
| --- | --- |
| `AGENTS.md` | 项目原则与协作规范 |
| `notes/source-inventory.md` | PDF 版本、元数据和质量风险 |
| `notes/page-map.md` | 本地 PDF 页码与原书印刷页码映射 |
| `notes/book-structure.md` | 分部、章节和前后置材料的全书关系 |
| `notes/progress.md` | 逐章翻译与人工确认状态 |
| `notes/review-policy.md` | 独立初审、修订与人类批准制度 |
| `notes/reviewers.md` | 审校角色及上下文边界 |
| `notes/reviews/_template/` | 可复制的章节审校与批准模板 |
| `scripts/split_pdf.py` | 可复现的 PDF 切分与验证脚本 |
| `scripts/init_review.py` | 针对确定 commit 建立章节审校包 |
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

## 编译 LaTeX 书稿

运行以下命令可编译完整书稿入口：

```sh
make pdf
```

书稿固定输出为 `output/pdf/tapl-zh.pdf`；后续章节会继续加入这一份产物，
不按分部生成不同 PDF。`build/main.pdf` 仅是编译过程中的中间副本。项目支持
XeLaTeX（latexmk）或 Tectonic。

## Git 版本管理

仓库跟踪项目规范、LaTeX 源文件、脚本、页码映射、校对记录和切分清单。
原始教材 PDF、可重新生成的切分 PDF、临时文件和 LaTeX 编译产物默认不纳入
Git。切分文件继续保留在本地，并可通过 `make split` 重建。

每章建议使用独立提交记录翻译与校对修改；人工明确确认后可创建
`chapter-NN-approved` 格式的标签。

章节初稿提交后，使用下列命令创建绑定该提交的独立审校包：

```sh
make init-review UNIT=preface TARGET=<commit>
make init-review CHAPTER=1 TARGET=<commit>
```

翻译角色不得审查自己的译稿。独立审校角色只填写
`notes/reviews/chapter-NN/` 中的报告，不直接修改章节 LaTeX；最终批准权
属于人类总审。
