---
name: architecture-base-vs-ridl-variant-selection
description: base/ridl 变体选择必须由叶子二进制决定；记录 js_stdlib 的连接点作用与归档拆分修复
type: architecture
created: 2026-09-04
updated: 2026-09-04
sources: [deps/mquickjs-build/src/main.rs, deps/mquickjs-rs/build.rs, deps/mquickjs-ridl-glue/src/lib.rs, deps/mquickjs-sys/build.rs]
---

# base / ridl 变体选择：必须由叶子二进制决定

> **本文档取代先前的错误结论。** 曾有一版把 `cargo test --workspace` 链接失败
> 判定为"编译期注册架构的固有结果、无法规避"。**该结论错误**：
> 真正的问题是**变体选择被放在了全局 feature 上，而不是叶子二进制上**。
> 修复后 `cargo test --workspace` 全绿。

## 问题现象（修复前）

```
$ cargo test --workspace
rust-lld: error: undefined symbol: js_test_require_class_foo_constructor
        >>> referenced by mqjs_stdlib_impl.c
        >>>               mqjs_stdlib_impl.o:(js_c_function_table)
error: could not compile `mquickjs-rs` (lib test)
```

## 根因链

1. mquickjs 的类注册必须编译期完成，因此应用的 C stdlib 需要一张列出全部 RIDL
   模块 C 入口的表 `js_c_function_table`（由 `mquickjs_build.c` 的 `dump_cfuncs()` 生成，
   逐条直接引用函数名 → **强未定义符号**）。

2. 构建产出两个变体，差异**只**在 stdlib 对象与 ridl 专属对象：

   | 对象 | base vs ridl |
   |---|---|
   | `mquickjs.o` / `dtoa.o` / `libm.o` / `cutils.o` | **字节完全相同**（md5 一致） |
   | `mqjs_stdlib_impl.o` | 不同（ridl 的表引用 RIDL 模块符号） |
   | `mquickjs_ridl_register.o` / `mqjs_require.o` | 仅 ridl 有 |

3. **`js_stdlib` 是"变体专属对象"与"共享 rlib"之间唯一的连接点。**
   `context.rs` 的 `JS_NewContext(..., &js_stdlib)` 使任何链接 `mquickjs-rs` 的
   二进制都必须定义 `js_stdlib`，链接器因此必然拉入 `mqjs_stdlib_impl.o`，
   连带其全部 RIDL 引用。

4. 变体选择原先放在**全局 feature** 上（`deps/mquickjs-sys/build.rs`）：
   ```rust
   .join(if _ridl_extensions_enabled { "ridl" } else { "base" })
   ```
   `cargo test --workspace` 时 feature 统一（根 crate `default = ["ridl-extensions"]`）
   → `mquickjs-rs` 的每个测试目标（含 lib test）都链接 ridl stdlib →
   而它**无法**提供那些 RIDL 符号（不能依赖 RIDL 模块 crate，会成环）。

5. 顺带核实：**"在 mquickjs-rs 测试里调 `ridl_bootstrap!()`"不可行** ——
   其 OUT_DIR 无 `ridl_bootstrap.rs`；即便补上，生成内容引用它不依赖的 crate，
   编译不过；真加依赖则成环。

## 修复：把变体选择下沉到叶子二进制

### 1) 归档拆分（`deps/mquickjs-build/src/main.rs`）

```
libmquickjs_core.a       变体无关的引擎对象（可随 rlib 传播）
libmquickjs_stdlib_base.a   base 变体的 stdlib（+ 变体专属对象）
libmquickjs_stdlib_ridl.a   ridl 变体的 stdlib
libmquickjs.a            合并归档（向后兼容 selftest / 外部 consumer）
```

stdlib 归档**按变体命名**，避免同名归档需要靠 `-L` 顺序区分。

### 2) `mquickjs-rs`：只传播 core，自身 test 目标固定 base

`deps/mquickjs-rs/build.rs`：
- `rustc-link-lib=static=mquickjs_core`（变体无关，可传播）
- `rustc-link-arg` 指定 base stdlib —— **注意**：`rustc-link-arg` 只作用于
  本包自己的目标，**不作用于 lib test**（只有 `-tests` 变体才覆盖 lib test，
  实测 `rustc-link-arg-tests` 不覆盖 lib test），因此这里用无条件 `rustc-link-arg`。

### 3) base 变体导出符号为 weak

应用最终二进制会**同时**拿到 base（经 mquickjs-rs 的 rlib 元数据传播，
供其自身测试与 trybuild 这类**嵌套构建**）与 ridl（由应用链接）。
若都是 strong → `duplicate symbol: js_stdlib / js_date_constructor / js_date_now`。

因此令 **base 的这三个导出符号为 weak**，ridl 保持 strong：
- 单独链接 base：weak 定义可用 ✓
- 同时链接两者：strong 胜出，无重复定义 ✓

实现要点（易踩坑）：
- `js_date_*` 定义在 `deps/mquickjs-rs/mqjs_stdlib_impl.c`，用 `JS_STDLIB_LINKAGE` 宏前缀，
  由 `mquickjs-build` 在 base 变体下 `-DJS_STDLIB_LINKAGE=__attribute__((weak))`。
- `js_stdlib` 是**由宿主工具生成的文本**（`mquickjs_build.c` 的 `printf`），
  因此属性必须在**生成时**决定 —— 用 `MQUICKJS_ENABLE_RIDL_EXTENSIONS` 判断。
  **关键坑**：`mquickjs_build.c` 被单独编译成 `mquickjs_build.host.o`，
  该步骤原先**没有**传该宏（只传给了模板），导致两个变体都生成 weak。
  必须在该步骤也按变体传宏。
- `printf` 里用的是 `%s`，所以宏必须是**字符串字面量**
  （`JS_STDLIB_LINKAGE_STR`），不能直接是属性。

### 4) 应用侧自行链接 ridl stdlib

`deps/mquickjs-ridl-glue/src/lib.rs` 的 `emit_native_stdlib_link()`：
用 `-Wl,--whole-archive,<abs path>,--no-whole-archive`。

**为什么用 whole-archive 而不是 `-L` + `-l`**：由 `rustc-link-lib` 产生的 `-l`
会被放在 **rlib 之前**，静态归档只在"扫描到它时存在未定义符号"时才抽出成员，
因此 `mqjs_stdlib_impl.o` 不会被抽出 → `undefined symbol: js_stdlib`。
`--whole-archive` 无条件包含全部成员，消除对库顺序的依赖。

应用的 `build.rs` 调用该函数（根 crate、`apps/test_app`、cargo-generate 模板同）。

### 5) trybuild 的嵌套构建

trybuild 在 `target/tests/trybuild/` 下发起**独立的 cargo 构建**，那些 crate
不是本包目标。它们通过 `mquickjs-rs` 的 rlib 元数据拿到传播的 base stdlib
（这正是第 3 步必须让 base 为 weak 的原因）。

`mquickjs-rs` 为此导出 `native_test_link_args()`，供需要嵌套构建的测试参考。

## 结果

```bash
cargo test -p ridl-tool          # ✓
cargo test -p mquickjs-rs        # ✓
cargo test -p mquickjs-rs --features ridl-extensions   # ✓（修复前 101）
cargo test -p mquickjs-demo      # ✓
cargo test --workspace           # ✓ 全绿（修复前全面链接失败）
cargo run -- tests               # ✓ 26/26
cargo run -p ridl-builder -- selftest-gc-mark   # ✓
```

## 教训

**"哪个二进制需要 RIDL"是叶子的属性，不是全局 feature 的属性。**
把这类选择放在 feature 上，会在 feature 统一时把不属于该叶子的依赖强加给它。