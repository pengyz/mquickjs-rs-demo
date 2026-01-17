# 2026-01-17 fn 覆盖与 HandleScope 前置：状态快照

## 背景与目标
本轮目标原本是：
- fn 相关用例全覆盖（default/strict）
- strict 下非 variadic 多余参数报 TypeError
- callback/array/map 暂不支持：保持注释并加入 TODO

过程中发现：`any` 的 Rust 映射与 mquickjs 可达性/GC root 机制存在根本性设计问题，需要先实现机制型 HandleScope（对外 API），再回到 fn 覆盖。

## 已确认语义（用户确认）
- default：非 variadic 多余参数忽略
- strict：非 variadic 多余参数 TypeError
- double：follow 引擎，不限制 NaN/Infinity
- variadic：覆盖 any 与具体类型（...args: string）
- callback/array/map：暂不支持，保持注释并 TODO

## 当前关键结论：mquickjs 可达性与 GC root
### 结论
- mquickjs **不会扫描 C/Rust 机器栈**，因此 C/Rust 局部变量中保存的 JSValue **不会自动成为 GC root**。
- mquickjs 的 GC root 主要来自：
  - VM 值栈（ctx->sp .. ctx->stack_top）
  - GCRef 链（ctx->top_gc_ref / ctx->last_gc_ref）
- native 调用期间可能触发 GC（例如 JS_StackCheck / check_free_mem 会调用 JS_GC）。
- 因此：只要存在“创建 JSValue 后、写入可达位置前”的窗口期，且期间可能发生分配/GC，就必须用 GCRef/HandleScope 等机制保护。

### 证据（关键源码点）
- `deps/mquickjs/mquickjs.c: gc_mark_all(JSContext*, ...)`
  - 扫 VM 栈：`for (sp = ctx->sp; sp < (JSValue*)ctx->stack_top; sp++) ...`
  - 扫 GCRef：`for (ref = ctx->top_gc_ref; ref != NULL; ref = ref->prev) ...`
- `deps/mquickjs/mquickjs.c: JS_Call(...)`
  - C 函数 argv 指向 VM 帧：`fp + FRAME_OFFSET_ARG0`
- `deps/mquickjs/mquickjs.c: JS_StackCheck / check_free_mem`
  - 可能触发 `JS_GC(ctx)`
- `deps/mquickjs/mquickjs.h: JSGCRef + JS_PUSH_VALUE/JS_POP_VALUE`
  - 以及 `JS_AddGCRef/JS_DeleteGCRef`

## 当前工作树改动概览（未完成，存在 build break）
### 文件变更（相对 HEAD）
- `Cargo.toml`
  - 新增依赖：`ridl_test_g_fn_strict`（新的 strict 测试 crate）
- `tests/global/fn/test_fn/src/test_fn.ridl`
  - 扩展 default 覆盖：noArgs/echoBool/echoString/echoInt/echoDouble/echoIntOpt/echoAny/union/addInt/countAny/joinStrings
  - callback/array/map 仍注释并 TODO
- `tests/global/fn/test_fn/src/fn_impl.rs`
  - 实现上述 default singleton
  - 注意：曾出现临时 `unimplemented!()`（用户要求禁止）
- `tests/global/fn/basic.js`
  - 重写 default 覆盖用例（成功路径 + TypeError + default 多参忽略等）
- `tests/global/fn/strict.js`
  - 新增 strict JS 用例（strict 多参 TypeError + variadic）
- `tests/global/fn/test_fn_strict/`（新增）
  - `Cargo.toml`/`build.rs`/`src/lib.rs`/`src/strict_impl.rs`/`src/test_fn_strict.ridl`
- `deps/ridl-tool/templates/rust_glue.rs.j2`
  - strict + non-variadic：`argc > params_len => TypeError`（functions/singletons/interfaces/classes methods）
- `deps/ridl-tool/src/generator/mod.rs`
  - 补齐 TemplateFunction/TemplateMethod 的 `file_mode` / `has_variadic`
- `deps/ridl-tool/src/generator/filters.rs`
  - 变更：string 解码更偏向 Rust `String`（但当前状态仍有未收敛点）

### 当前主要阻塞/失败点
1) **any 返回类型设计不闭环**
- 之前生成：入参 any -> `Local<Value>`，返回 any -> `Global<Value>`，但 trait 不带 `&Scope`，导致 impl 无法实现。
- 讨论结论：返回 any 应改为 `Local<Value>`，并且必须引入机制型临时根（对外 API）。

2) **mquickjs-demo 生成的 ridl_runtime_support.rs 引用 strict ctx slot vtable 错误**
- 编译错误示例：引用 `ridl_test_g_fn::RIDL_TEST_FN_STRICT_CTX_SLOT_VT`，但正确应来自 `ridl_test_g_fn_strict`。
- 需要在应用聚合/生成逻辑中修正符号来源。

3) **禁止临时实现**
- `fn_impl.rs` 中出现过 `unimplemented!` 以绕过 `any` 返回（用户明确禁止）。

## 下一步计划（中断当前任务，先做 HandleScope 机制）
1) 回到“可用基线”
- 移除临时实现/修复当前明显编译错误，保证最少能构建通过（或回到上一个可用点）。

2) 实现 V8 风格 API（底层 mquickjs GCRef）
- `HandleScope`：批量临时根（基于 `JS_PushGCRef/JS_PopGCRef` 或等价）
- `EscapableHandleScope`：支持 `escape()` 将值提升到外层 handle scope
- `Global`：继续复用现有 `JS_AddGCRef/JS_DeleteGCRef` 实现

3) 将 HandleScope 接入 ridl glue codegen
- 所有创建/中间持有 JSValue 的路径都必须可达（机制保证，不依赖使用者纪律）。

4) 回到 fn 覆盖与 strict 多参 TypeError 完成与验证
- `cargo test`
- `cargo run -- tests`
