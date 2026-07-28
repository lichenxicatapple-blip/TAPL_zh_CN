# 前言原始图表资产

本目录中的图像由 `scripts/extract_preface_assets.py` 从以下切分文件可复现
地提取：

`source/split/frontmatter/03-preface_p005-p016.pdf`

- `chapter-dependencies.png`：本地前言第 4 页的图 P-1。
- `sample-syllabus-reference.png`：本地前言第 7 页的图 P-2，仅作为重排
  LaTeX 表格时的逐项核对依据。

CHM 转换件以自下而上的方向存储这两张位图，并在 PDF 页面绘制时翻转。
提取脚本会执行一次垂直翻转，使生成文件符合普通图像坐标方向。不得手工覆盖
生成图片；源 PDF 或提取方式变化时应重新运行脚本并核对输出 SHA-256。
