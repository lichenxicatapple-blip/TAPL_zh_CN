# TAPL 作者官方配套资料快照

本目录保存本项目核对译稿与开发 Rust 参考实现时使用的作者官方资料。

来源：

- 作者网站：<https://www.cis.upenn.edu/~bcpierce/tapl/>
- 官方实现目录：<https://www.cis.upenn.edu/~bcpierce/tapl/checkers/>
- 官方勘误：<https://www.cis.upenn.edu/~bcpierce/tapl/errata.txt>

获取日期：2026-07-30；第三部分补充资料获取于 2026-08-10；第五部分补充资料
获取于 2026-08-21。

`errata.txt` 文件内标明其内容更新至 2024-12-31。当前快照保存了第 4 章
对应的 `arith`、第 7 章对应的 `untyped`、第 5 章示例所使用的
`fulluntyped`，以及第二部分核对所需的 `tyarith`、`simplebool`、
`fullsimple`、`fullref` 和 `fullerror`，以及第三部分第 15--17 章核对所需的
`fullsub`、`bot` 和 `rcdsubbot`。第五部分另保存了第 22 章所需的
`recon`、`reconbase`、
`fullrecon` 与练习骨架 `letexercise`，第 23--25 章所需的 `fullpoly`，以及第
26--28 章所需的 `fullfsub`、`fullfomsub` 与 `purefsub`。原始压缩包与
解压内容都保留在仓库中；解压内容只作为原书 OCaml 行为和示例的核对依据，
不作为本项目 Rust 代码的构建输入。

`letexercise` 特意把练习 22.7.1 要求读者完成的分支保留为断言占位，因而不属于
可以直接运行通过的作者成品；自动验证清单不会把它误报为已通过实现。其余列入
`scripts/check_official_ocaml.sh` 的项目均从这里复制到构建目录后编译并运行。

## SHA-256

```text
bcd6f38a3fae1665a99e3cbe0a5bb10a0027bd77b352352165f6a308b60ef604  arith.tar.gz
c6959c97e3f71e8fde4d8384e5f7883391b86811b2619af8241c60fade4c5b6f  untyped.tar.gz
e3985d1e3bafd07d79bd30f122ba58ad14439bc7e42eb355435af3cf71532ab3  fulluntyped.tar.gz
2257ca261392b4e65ec8899430a9c2de10ffdcf897dc1d4b75bd1322c8816153  tyarith.tar.gz
a40e87feb851926527a076f13c07a882fcedf7236aad4a47302d4604f36ea335  simplebool.tar.gz
d635d8ad439ca3e3a8159020df9d157efe949a5a518c8c814a79cb52df383d46  fullsimple.tar.gz
75a6f65cd69fc1c704f69efbcf1bda81ddb34d3dc3ff6b2307f812073032159d  fullref.tar.gz
5b50212c48168168ee7db34b47d91a286b83c84243aa5f966a6974e9d0cfc7a8  fullerror.tar.gz
651d2e3f3707e5cf03cb997e2413de444445852589cda48089e01b6487efa790  fullsub.tar.gz
aa0bc0c7ae5a933ce337c83f46072bb9631b22b6664a00c3e22baa8d9978948e  bot.tar.gz
43a3b0df69940cf02973fc8894545e1003df3362d34dffd7e98385b9d4f95e87  rcdsubbot.tar.gz
a04ed3b1f668496e3f7feac32f2a77c0d134fca928f0283cf01659f734cb5161  recon.tar.gz
2733e1da406d855282bb1b49c49ac4591096712d46a2e6d2ee64998f1045830a  reconbase.tar.gz
c1071c2dbe59ad47bbdb61e10d5cc72a10e5ea0ca99f302257c2df29f69a2fdd  fullrecon.tar.gz
14b656c84d13bf732af052132084ec5e77997f73226411f9ddf93cd80bdb9282  letexercise.tar.gz
bf6355ee46102e2a5e6cccf93677ea0704f5bbf0d798c853e7f41faea77ea1b1  fullpoly.tar.gz
591eee4e4ef47dd62a982773dcc37d1abdd8bb2932101498714385ae609ac288  fullfsub.tar.gz
06cc5a1b36afd11df1eaeb86ed6937fc34bed5f4bf0566e164a5e9c2fbbebb90  fullfomsub.tar.gz
544c55b3792d1b6b6924d5a469706dc70ba5ec7f34d371dbcf8f6afaa4e1286c  purefsub.tar.gz
91a5c89d43f3aa4d9119728b07eb39f7ab07a76dbd5cb9b1deddd6c46d02be60  errata.txt（上游原始 CRLF 字节）
324527b7a62e1d6f82c139f62e095b230fac0a45bc1b8f0747bd62272fb4e29c  errata.txt（仓库 LF 规范化文件）
```

Git 按仓库文本规范把 `errata.txt` 的 CRLF 行尾转换为 LF，因此上游下载文件
与仓库内快照内容相同、字节哈希不同；上面同时记录两个校验对象。
