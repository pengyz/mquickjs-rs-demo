# RIDL 文档索引

本目录存放 RIDL 的现行语义口径、运行时初始化、require 行为、以及代码生成边界等文档。

> 说明：`docs/planning/` 下存在大量按工作维度组织的讨论稿/方案稿；它们不作为现行规范。

## 推荐阅读顺序（现行口径）

1. `overview.md`（待补充）：RIDL 语义总览
2. `context-init.md`（待补充）：以 `ridl_context_init(ctx)` 作为唯一 correctness gate
3. `require-materialize.md`（待补充）：require + ROMClass materialize/writeback 语义
4. `codegen-outputs.md`（待补充）：聚合产物与职责边界

## 现有文档（待梳理/部分可能过时）

- `syntax-and-extension.md`
- `module-design.md`
- `../legacy/stdlib-extension-mechanism.md`（已过时）
- `thin-vtable-glue-impl-split.md`
- `../legacy/glue-generator-templates-and-type-conversion.md`（部分过时）
- `../legacy/rust-glue-evolution.md`（历史/部分过时）
