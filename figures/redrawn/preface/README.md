# 图 P-1：章节依赖关系

- 可维护源文件：`chapter-dependencies.tex`
- 生成文件：`chapter-dependencies.pdf`
- 构建命令：`make preface-figures`
- 语义校验：`scripts/verify_preface_dependency_figure.py`

重绘依据是作者官网发布的正常排版前置材料
`https://www.cis.upenn.edu/~bcpierce/tapl/frontmatter.pdf` 第 xvi 页，以及本项目
从原始教材工作副本提取的
`figures/original/preface/chapter-dependencies.png`。

图中每条箭头从后面的章节指向其所依赖的前面章节。黑色实线表示主要依赖，
灰色线表示后面的章节只有一部分依赖目标章节。当前语义集合固定为 32 个节点、
42 条实线依赖和 3 条灰色部分依赖。

不得手工编辑生成的 PDF；修改布局或样式时应编辑 TikZ 源文件，运行构建命令，
并重新进行语义校验和书稿页面视觉检查。
