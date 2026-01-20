# 规划：ridl-tool module 使用聚合构建的全局 class id（A3）

日期：2026-01-19

## 背景与问题
当前测试（`tests/global/require/test_require`）在 JS 侧执行 `new Foo()` 后，得到的实例不是 Foo 实例，导致 `foo.value` 缺失。

定位结论：
- 聚合构建（`ridl-builder prepare`）会为 app 内所有模块分配**全局单调递增**的 `JS_CLASS_USER + N`。
- 但各模块 crate 在 `build.rs` 里执行 `ridl-tool module ... $OUT_DIR`，module 模式生成的 Rust glue 使用的是**模块内局部**的 `class_id`（默认从 0 起），于是 `JS_NewObjectClassUser(JS_CLASS_USER + 0)` 创建了错误 class（通常是 module object class），与运行时全局布局不一致。

这属于“module 模式生成物与聚合运行时布局错配”。

约束（用户确认）：
- `ridl-manifest.json` 仅用于审计，不作为生成输入。
- 当前阶段允许模块 crate 继续保留 `build.rs`，用于生成本模块的 `api.rs/glue.rs`；用户不手写 include/copy。

## 目标
1. ridl-tool 的 **module 模式**生成的 Rust glue 不再使用局部数值 `JS_CLASS_USER + <local_id>`。
2. module glue 统一引用**聚合构建输出的全局 class-id 定义**（符号/常量），从而与运行时一致。
3. 恢复并通过：
   - `cargo test`
   - `cargo run -- tests`（包含 `tests/global/require/test_require/tests/basic.js`）

## 方案（A3）概述
### 核心想法
把“class id 的数值”从 module glue 中移除，让 module glue 引用一个稳定的“全局 class id 常量”。

聚合构建已经生成全局 class id 的 C 头：
- `mquickjs_ridl_api.h`（含 `#define JS_CLASS_* (JS_CLASS_USER + N)`）

实现上：
- 由 `mquickjs-rs/build.rs` 解析 `mquickjs_ridl_api.h`，生成 Rust 常量模块 `mquickjs_rs::ridl_js_class_id`。
- module glue 生成时使用 `mquickjs_rs::ridl_js_class_id::JS_CLASS_*` 作为 class id 偏移。

## 问题1（typeof Foo 异常）的结论（固定）
我们在定位过程中遇到过第二条现象：在“从对象属性读取 class ctor”的特定路径上，`typeof Foo` 曾出现不一致（表现为 `typeof Foo === "object"`，而不是 `"function"`）。

最终结论：
- 该问题**不是 RIDL 直接改变了 JS 语义**导致的。
- RIDL 触发/组合出了一个标准库中默认不覆盖的场景：**把一个 class ctor（在 ROM 表达中对应 JS_PROP_CLASS_DEF / ROMClass）作为另一个对象的 property 值暴露到 JS**，从而走到了 mquickjs 的 getprop/OP_get_field 快路径。
- 该快路径在当时缺少对“ROMClass -> ctor(JS Function) 物化”的处理，导致 `typeof` 观察到的值可能不是 function。
- 我们已在 `deps/mquickjs/mquickjs.c` 中补齐该快路径的 ROMClass->ctor materialize 逻辑，使得该场景下 `typeof` 稳定为 function。

因此，问题1应定性为：**mquickjs 标准库/ROM 表达在该组合场景上的未覆盖路径 bug**，RIDL 只是触发条件之一；该问题已修复。

## 已实施的改动（摘要）
1) **ridl-tool**
- module 模式生成的 Rust glue：
  - 从 `JS_CLASS_USER + <local_id>` 改为引用 `mquickjs_rs::ridl_js_class_id::JS_CLASS_*`。

2) **mquickjs-rs**
- build.rs 新增解析 `include_dir/mquickjs_ridl_api.h`：
  - 生成 `OUT_DIR/ridl_js_class_id.rs`。
- `src/lib.rs` 新增：
  - `pub mod ridl_js_class_id { include!(.../ridl_js_class_id.rs) }`。

3) **验证**
- `cargo run -- tests/global/require/test_require/tests/basic.js` 通过。
- `cargo test` 通过。

## 测试矩阵
- `cargo test`
- `cargo run -- tests`
- 重点回归：`cargo run -- tests/global/require/test_require/tests/basic.js`

## 风险与回滚策略
- 风险：`mquickjs_ridl_api.h` 在未运行 `ridl-builder prepare` 前可能不存在；为此 `mquickjs-rs/build.rs` 在缺失时生成空文件，避免硬失败。
- 回滚：保留原有 module glue 的生成方式（使用局部 id）并回退对 class id 常量的引用；但会重新引入错配风险。
