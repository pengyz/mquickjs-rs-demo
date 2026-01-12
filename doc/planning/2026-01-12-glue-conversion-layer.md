# 设计：由生成器负责的 Glue 转换层（支持 singleton / class）

日期：2026-01-12

## 1. 目标

我们希望 RIDL 生成的 `glue.rs` 成为 QuickJS/mquickjs C ABI 与 Rust 实现之间的**转换/适配层**：

- 对外导出符合 mquickjs/QuickJS 回调约定的 `extern "C"` 函数（引擎通过这些函数调用到 Rust）。
- 按照 RIDL 接口声明，把 `JSValue` 参数转换为 Rust 参数（含参数个数检查、类型检查）。
- 对 `singleton` / `class` 这类“有接收者”的调用，在 glue 层**解析 Rust 实例**：
  - `singleton`：从每个 JSContext 对应的 `CtxExt` slot 中取得实例（使用我们的 RIDL runtime 机制）。
  - `class`：从 `this_val` 上通过 `JS_GetOpaque` 取回 Rust 指针。
- 调用 `impl.rs`（用户手写实现层）中提供的 Rust 函数/trait 方法。
- 将 Rust 返回值再转换成 `JSValue` 返回给引擎。
- 当转换/分发失败时抛出 JS 异常（TypeError 等）。

> 分层约定：
> - `glue.rs` = 转换层（C ABI + JSValue<->Rust + receiver 解析 + 错误抛出）
> - `impl.rs` = 纯 Rust 逻辑实现（原则上不直接处理 `JSValue` 转换/抛异常）

非目标：
- 不在本文档中确定 RIDL 语言的全部语义。
- 不在本文档中决定聚合集成路径（Plan A vs Plan B）；本文只讨论 glue 生成。

## 2. 约束（项目规则）

- **禁止硬编码 singleton/module 名称**去做聚合/注册/初始化的白名单或特判。
- C API 注册必须发生在编译/链接期驱动的路径中，避免运行时“扫描发现模块”。
- 新增 RIDL module 不应要求修改中心化白名单。
- 生成代码需稳定、确定（函数/slot 顺序稳定）。

## 3. 现状（截至 2026-01-12）

- `deps/ridl-tool/templates/rust_glue.rs.j2` 已经具备：
  - free function 包装（基础类型转换）
  - singleton 方法包装（通过 `ridl_get_erased_singleton_slot_by_name` 按 name-key 取 slot）
  - `js_throw_type_error` 等基础错误抛出 helper
- runtime 支持已存在：
  - `mquickjs_rs::ridl_runtime::{ErasedSingletonSlot, RidlErasedSingletonVTable}`（create/drop）
  - `mquickjs_rs::ridl_ext_access::{ridl_get_erased_singleton_slot, ridl_get_erased_singleton_slot_by_name}`
  - `ContextHandle::from_js_ctx(ctx)`：从 `JSContext` 获取 `ContextInner`，进一步取 `ridl_ext_ptr()`

缺口：
- `class` 绑定路径仍不完整（class_id、`JS_GetOpaque`、finalizer/析构）。
- 转换逻辑目前部分散落在 generator 的字符串片段里，需要结构化/可复用。
- 复杂类型（对象/数组/结构体等）的转换策略尚未标准化。

## 4. 目录与文件形态（对齐 stdlib 示例）

### 4.1 模块内代码的“手写 vs 生成”边界

以 `ridl-modules/stdlib` 为例：

- **用户手写实现**放在模块 crate 自己的源码中，例如：
  - `ridl-modules/stdlib/stdlib_impl.rs`（手写）
  - 在 `ridl-modules/stdlib/src/lib.rs` 中通过 `#[path = "../stdlib_impl.rs"] mod stdlib_impl;` 引入
  - 然后在 `pub mod impls { ... }` 中把需要给 glue 用的 Rust API re-export 出来（如 `create_console_singleton`）

- **生成器输出**统一落到 `OUT_DIR`，由模块 crate 通过 `include!(concat!(env!("OUT_DIR"), "..."))` 引入，例如：
  - `OUT_DIR/<module>_glue.rs`：转换层（C ABI wrapper + 转换 + receiver 解析）
  - `OUT_DIR/<module>_symbols.rs`：注册表/符号
  - `OUT_DIR/<module>_impl.rs`：接口声明/trait/默认 stub（用于 glue 稳定引用），但**不作为用户手写实现文件**

> 结论：本文档里不再使用 `generated/<module>_impl.rs` 这种“生成到源码目录下”的表述。
> impl（业务实现）应当位于模块源码中（`src/` 或类似 stdlib 的 crate 根文件），生成物只在 `OUT_DIR`。

### 4.2 每个模块的建议结构

- `src/lib.rs`：模块入口；include 生成物、导出 glue symbols；拼装 `impls` 模块（供 glue 调用）
- `src/impls.rs`（可选）：手写实现的统一导出层（也可像 stdlib 用 `#[path]` 引入 crate 根实现）
- `<module>_impl.rs`（手写实现，可放在 `src/` 或 crate 根）
- `OUT_DIR/*`：所有 generator 生成文件

## 5. 类型映射与转换语义

### 5.1 基础类型（v1 glue）

| RIDL | impl.rs 侧 Rust 类型 | JS 表示 | glue 转换 |
|---|---|---|---|
| `bool` | `bool` | boolean | tag 检查 / `JS_ToBool`（按现有绑定选用） |
| `int` | `i32` | number | `JS_IsNumber` + `JS_ToInt32` |
| `double` | `f64` | number | `JS_IsNumber` + `JS_ToNumber` |
| `string` | `*const c_char`（或后续包装为 `&str`） | string | `JS_IsString` + `JS_ToCString` |
| `any` | `JSValue` | any | 透传 |
| `void` | `()` | undefined | 返回 `JS_UNDEFINED` |

字符串生命周期：
- 必须明确 `JS_ToCString` 返回值是否需要 `JS_FreeCString`。
- 如果当前绑定使用 `JSCStringBuf`（短缓冲）策略，需要在文档中写清楚可用范围，避免泄漏/悬垂。

### 5.2 Optional

`T?` -> Rust `Option<T>`。

建议语义：
- 参数缺失 OR `undefined` OR `null` => `None`
- 否则 => `Some(converted)`

### 5.3 Varargs

`...args:T`：glue 收集 `argv[idx..argc]` 并逐个转换为 `Vec<T>`。
转换失败时抛 TypeError，错误信息带上下标（例如 `args[3]`）。

### 5.4 复杂类型（后续扩展）

对象/数组/结构体等：
- 短期内统一按 `any(JSValue)` 透传，先把 glue 框架稳定下来。
- 后续在确认依赖/抽象后再逐步引入强类型转换（例如 `Value` wrapper 或 serde-json）。

## 6. receiver 解析（singleton / class）

### 6.1 singleton

glue 需要：
1) `ContextHandle::from_js_ctx(ctx)`
2) `ext_ptr = h.inner.ridl_ext_ptr()`
3) `slot_ptr = ridl_get_erased_singleton_slot_by_name(ext_ptr, name)`
4) 检查 `slot.is_set()`
5) 取出 slot 内存放的指针并转换为预期的 dyn-trait holder
6) 调用 trait 方法

约束：
- singleton key 来自 RIDL 声明，不得手写特判。
- 使用 name-key lookup，避免跨 crate slot_index 耦合。

### 6.2 class

glue 需要：
1) 拿到 class_id（每 module / 每 class）
2) `ptr = JS_GetOpaque(this_val, class_id)`
3) 若为空 => 抛 TypeError（invalid this）
4) cast 为 `*mut T`
5) 调用 impl 方法

所有权与析构：
- constructor glue 创建 `Box<T>` 并将指针绑定到 JS 对象的 opaque
- 注册 finalizer，在 GC 时 drop `Box<T>`

待确认：
- class_id 的存放和初始化时机。

建议方案：
- module 的 `initialize_module()` 注册 class，并把 class_id 存到 module crate 内部的 `static mut`。
- 保证使用 class 之前模块初始化已完成（由聚合路径负责调用）。

## 7. 错误处理

glue 层负责抛 JS 异常并返回 thrown value（QuickJS 约定）。

最小集合：
- 参数缺失/类型不匹配：TypeError
- ctx user_data 缺失、ridl_ext 缺失、ctx-ext vtable 缺失：TypeError
- singleton 未初始化：TypeError
- class invalid this：TypeError

建议把错误 helper 统一在 glue 公共部分（例如 `js_throw_type_error(ctx, msg)`）。

## 8. generator 的数据模型与模板组织

### 8.1 IR（用于模板渲染的数据）

需要在 AST -> 模板模型中表达：
- 函数/方法列表、参数列表（normal/optional/vararg）、返回类型
- receiver kind：free / singleton / class
- class：类名、构造函数、方法、属性、class_id 符号名等

### 8.2 filters（生成转换片段）

filters 需要生成：
- 参数提取与转换片段
- 调用参数列表
- 返回值转换片段
- receiver 解析片段（singleton/class 两套）

要求：filters 产出确定、可单测、可复用。

## 9. 测试策略

### 9.1 generator 单测（ridl-tool 内）

- 对代表性 RIDL 输入做“生成结果检查”（可做 snapshot 形式）：
  - free functions（基础类型）
  - singleton methods
  - optional / varargs
  - class receiver 解析（JS_GetOpaque、finalizer 片段）

### 9.2 repo 集成测试（JS）

- `tests/*.js` 作为 JS smoke（已接入 runner）：
  - 增加覆盖 singleton dispatch / class dispatch 的用例
  - runner 输出文件名与失败原因，便于 CI 排查

### 9.3 负向用例

- 错误参数类型应抛 TypeError
- class 方法使用错误 this 应抛 TypeError
- 未做 context init 的场景应抛 TypeError

## 10. 迁移计划

1) 先保持现有 v1 glue（基础类型 + singleton）稳定。
2) 引入 class glue（可加 feature gate 或 RIDL 语法开关）。
3) 逐步扩展复杂类型转换。
4) 确保任何 singleton constructor 不会是 `todo!()/panic!()` 的占位（应通过测试/构建期约束尽早暴露）。

## 11. 待决策问题

1) class_id 生命周期：存放位置、初始化顺序、以及如何确保 `initialize_module()` 在使用前执行。
2) string 转换的生命周期规则：是否必须显式 `JS_FreeCString`。
3) object/array/map 等复杂类型：短期用 `JSValue(any)` 过渡是否接受；长期是否引入 `Value` wrapper。
4) 若将来支持 async：glue 层如何表达 Promise/JobQueue 语义。
