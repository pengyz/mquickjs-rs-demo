<!-- planning-meta
status: 未复核
tags: build, engine, ridl
replaced_by:
- docs/ridl/overview.md
- docs/build/pipeline.md
-->

> 状态：**未复核**（`build` `engine` `ridl`）
>
> 现行口径/替代：
> docs/ridl/overview.md
> docs/build/pipeline.md
>
> 关键结论：
> - （待补充：3~5 条）
# 构建体系拆分方案（mquickjs-sys / mquickjs-rs）

## 目标
- 拆分 mquickjs-sys（纯 FFI + bindgen）与 mquickjs-rs（扩展/包装），解决符号裁剪与集成复杂度。
- 统一在 mquickjs-rs 内生成并链接 RIDL 扩展，减少上层手工 include/ensure。

## 方案概述
- mquickjs-sys：
  - 角色：仅编译原生 mquickjs 核心 C 代码，运行 bindgen 生成 FFI（mquickjs.h）。
  - 产物：核心 `.a` + bindings，不运行 RIDL 标准库生成，不包含 glue/symbols。
- mquickjs-rs：
  - 角色：依赖 mquickjs-sys，负责 RIDL 模块（作为子模块/子 crate）生成 glue/impl/symbols，以及运行 `mqjs_ridl_stdlib` 生成 C 标准库扩展表，将扩展与核心 `.a` 链接为最终库。
  - 符号保活：在库内部通过 `--whole-archive` 或内部 `ensure_symbols`/`#[used]` 引用，避免链接器裁剪，不要求上层调用。
  - 路径：生成头/符号文件放在 `generated/` 或 `mquickjs-rs/`，通过 `-I` 指向，不再复制到 mquickjs 目录。
- RIDL 模块归属：收编到 mquickjs-rs（workspace 子成员），统一构建/链接，避免上层手工集成。
- atoms/ClassID：
  - sys 仅对 `mquickjs.h` bindgen，不处理标准库头；如 Rust 需要 atoms/ClassID，可在 mquickjs-rs 中额外 bindgen 标准库头或手写常量映射。

## 动机
- 避免当前符号缺失/重复定义（js_*）问题。
- 简化上层（如 mquickjs-demo）使用：仅依赖 mquickjs-rs，得到可用的扩展。
- 让 sys 保持纯净，可复用且更符合 Rust best practice。

## 初步实施步骤（拟）：
1) 引入 mquickjs-sys crate：build.rs 编译核心 C，bindgen mquickjs.h；无 RIDL。
2) mquickjs-rs 依赖 sys，统一处理 RIDL 生成（glue/impl/symbols + mqjs_ridl_stdlib），链接核心 `.a` 与扩展对象；内部保活符号。
3) 调整 include 路径：`-I../generated`/`-I../mquickjs-rs`，不再复制到 mquickjs 源。
4) 若需要 atoms/ClassID，mquickjs-rs 中单独 bindgen 标准库头或手写常量。
5) 验证：cargo build 通过；js_sayhello 等符号在最终二进制可见。

## 状态
- 方案已讨论确认，待按步骤落地。
