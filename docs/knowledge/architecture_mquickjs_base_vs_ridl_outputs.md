---
name: mquickjs-base-vs-ridl-outputs
description: base/ridl 两套 QuickJS 输出与归档拆分；变体由**叶子二进制**选择（不是全局 feature）
type: architecture
created: 2026-09-03
updated: 2026-09-04
sources: [docs/knowledge/architecture_base_vs_ridl_variant_selection.md, deps/mquickjs-build/src/main.rs]
---

# base / ridl 两套输出与归档拆分

`ridl-builder prepare` 构建两个 QuickJS 变体：
- **base**：无 RIDL 扩展
- **ridl**：含 RIDL 扩展（其 stdlib 的 `js_c_function_table` 引用应用的 RIDL 模块 C 入口）

## 归档（修复后）

| 归档 | 内容 | 可传播性 |
|---|---|---|
| `libmquickjs_core.a` | 变体无关的引擎对象（`mquickjs.o`/`dtoa.o`/`libm.o`/`cutils.o`，两变体 **md5 相同**） | 经 rlib 元数据传播 |
| `libmquickjs_stdlib_base.a` | base 的 stdlib + 变体专属对象 | 按需 |
| `libmquickjs_stdlib_ridl.a` | ridl 的 stdlib + 变体专属对象 | 按需 |
| `libmquickjs.a` | 合并（向后兼容 selftest / 外部 consumer） | — |

stdlib 归档**按变体命名**，避免同名归档要靠 `-L` 顺序区分。

## 选择规则：**由叶子二进制决定，不是全局 feature**

> 早先的实现用 feature `ridl-extensions` 在 `mquickjs-sys` 里全局选择变体。
> **那是错的**：`cargo test --workspace` 会统一 feature，把 ridl stdlib
> 强加给 `mquickjs-rs` 自身的测试目标，而它无法提供那些 RIDL 符号
> （不能依赖 RIDL 模块 crate，会成环），导致全面链接失败。

现行规则：

| 叶子 | 链接 |
|---|---|
| 应用（`mquickjs-demo`、`apps/test_app`、模板生成的 app） | ridl stdlib，经 `--whole-archive`（`mquickjs_ridl_glue::emit_native_stdlib_link()`） |
| `mquickjs-rs` 自身目标（含 lib test） | base stdlib，经其 `build.rs` 的 `rustc-link-arg` |
| trybuild 的嵌套构建 | base stdlib（经 rlib 元数据传播） |

base 的 `js_stdlib` / `js_date_constructor` / `js_date_now` 导出为 **weak**，
ridl 保持 strong —— 这样应用同时拿到两者时不会 `duplicate symbol`。

## 何时使用

- 核心引擎层 / 库自身测试：依赖 base（不引用任何 `js_*` RIDL 符号）
- 应用层：链接 ridl stdlib（其 `build.rs` 调 `emit_native_stdlib_link()`）

**编译/链接失败时的排查顺序**：
1. `undefined symbol: js_stdlib` → 该叶子没有链接任何 stdlib
2. `undefined symbol: js_test_*` → 该叶子拿到了 ridl stdlib 却没有对应的 RIDL 模块 crate
3. `duplicate symbol: js_stdlib` → base 的 weak 属性丢失（检查
   `mquickjs_build.c` 的变体宏与 `mquickjs_build.host.o` 编译步骤是否传了宏）

详见 [`architecture_base_vs_ridl_variant_selection.md`](architecture_base_vs_ridl_variant_selection.md)。