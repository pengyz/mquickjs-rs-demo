---
name: gotcha-workspace-test-link-failure
description: 【已作废】原判定为"架构固有约束"—— 结论错误，已由 architecture_base_vs_ridl_variant_selection 取代
type: gotcha
created: 2026-09-04
updated: 2026-09-04
status: retracted
sources: [docs/knowledge/architecture_base_vs_ridl_variant_selection.md]
---

# ️ 本条结论已作废（RETRACTED）

**原结论**：`cargo test --workspace` 的链接失败是"编译期注册架构的固有结果"，
无法通过只链接部分对象规避，逐包运行才是唯一受支持方式。

**该结论错误。** 真正的问题是**变体选择被放在了全局 feature 上，而不是叶子二进制上**。

修复后 `cargo test --workspace` **全绿**（543 个测试）。

**请阅读更正后的条目**：
[`architecture_base_vs_ridl_variant_selection.md`](architecture_base_vs_ridl_variant_selection.md)

要点速览：
- `js_stdlib` 是"变体专属对象"与"共享 rlib"之间唯一的连接点
- 引擎对象（`mquickjs.o` 等）在两变体中**字节完全相同**，因此归档可拆分为
  `core` + 变体专属 `stdlib`
- 归档按变体命名 + base 导出符号为 weak + 应用侧 `--whole-archive` 链接 ridl
  → 叶子各自选择变体，互不干扰

**保留本文件的原因**：避免其他文档/记忆继续引用这条错误结论。
（注意：当时的统计"30 个未解析符号"也不准确，实测为 139 个。）