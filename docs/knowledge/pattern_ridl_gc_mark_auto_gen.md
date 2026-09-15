---
name: ridl-gc-mark-auto-gen
description: 【已作废】RIDL 曾自动生成 user class gc_mark —— 该机制有致命 ABI 缺陷且已冗余，已整体移除
type: pattern
created: 2026-09-03
updated: 2026-09-04
status: retracted
sources: [docs/knowledge/gotcha_mquickjs_gc_mark_signature.md, deps/ridl-tool/templates/rust_glue.rs.j2]
---

# ️ 本条模式已作废（RETRACTED）

**原模式**：RIDL 为含 `Traced<T>` 字段的 user class 自动生成 `gc_mark` 实现，
遍历 opaque 中的 `Traced<T>` 字段并 `mark_value`。

**该机制有两个致命问题，已整体移除。**

## 问题一：ABI 不匹配（活的 SIGSEGV）

生成的 Rust 定义是 **4 参** `(_ctx, _obj, opaque, mf)`，
而引擎契约是 **3 参** `(ctx, opaque, mf)`（`mquickjs.h` 的 `JSCMark`；
调用点 `mquickjs.c` 的 `c_mark_table[...](s->ctx, p->u.user.opaque, &mf)`）。

寄存器错位后 `opaque` 实参收到 `&mf`（栈地址、非空），被当作
`Box<dyn Trait>` fat pointer 解引用 → 虚表跳转 → **SIGSEGV**。

只要 GC 时该类对象**可达**就会崩溃。既有测试之所以没崩，是因为它们都
**先 delete 再 GC** —— 对象不可达 ⇒ `gc_mark_all` 不会遍历到它 ⇒ 回调从不被调用。

复现：`tests/gc_traced.rs::traced_node_reachable_during_gc_invokes_gc_mark`
（修复前为 `signal: 11, SIGSEGV`）。

知识库 [`gotcha_mquickjs_gc_mark_signature.md`](gotcha_mquickjs_gc_mark_signature.md)
**早已记录正确签名并指出草稿写错了**，但模板一直未跟上。

## 问题二：已冗余

`Traced<T>` 现基于引擎的 **`JSGCRef`**（见 `deps/mquickjs-rs/src/traced.rs`），
该链由引擎在 mark 与重定位两个阶段自动扫描。class 回调纯属多余。

## 现状

- 三个模板（`rust_glue.rs.j2` / `mquickjs_ridl_register.h.j2` / `mquickjs_ridl_api.h.j2`）
  与 `rust_api.rs.j2` 都不再生成任何 class gc_mark；
- 注册表中的 gc_mark 槽位恒为 `NULL`；
- trait 里的 `gc_mark` 默认方法也一并移除；
- 回归守卫：`deps/ridl-tool/tests/gc_mark_generation_test.rs`、
  `deps/ridl-tool/tests/gcmark_render_test.rs`（断言"不再生成"）。

**若将来因故需要恢复该机制**：必须严格按 `mquickjs.h` 的 3 参签名实现，
并补一个"对象在 GC 时可达"的测试。参见
[`gotcha_mquickjs_gc_mark_signature.md`](gotcha_mquickjs_gc_mark_signature.md)。