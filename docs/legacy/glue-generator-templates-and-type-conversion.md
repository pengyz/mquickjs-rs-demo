# Glue 生成器：模板开发与类型转换设计（部分过时）

> 部分过时：本文记录了早期 glue 模板/分层设计与演进计划，包含旧命名与旧产物结构。
>
> 现行口径请以以下文档为准：
> - `docs/ridl/codegen-outputs.md`
> - `docs/ridl/context-init.md`
> - `docs/build/pipeline.md`


> 本文档讨论 RIDL 工具（`deps/ridl-tool`）如何生成 Rust glue 转换层代码：包括模板如何组织、如何从 RIDL AST/IR 生成 wrapper、类型如何从 `JSValue` 转到 Rust 再转回 `JSValue`，以及 singleton/class 的 receiver 获取策略。

## 1. 背景与目标

### 1.1 分层定位

模块内建议分为三层（明确依赖方向）：
- `api.rs`（生成）：只包含 trait/类型声明（未来复杂类型也在这里声明），不包含 JSValue 转换。
- `impl.rs`（手写）：依赖 `api.rs`，实现 trait/业务逻辑，不依赖 glue。
- `glue.rs`（生成）：依赖 `api.rs + impl.rs`，负责 C ABI、JSValue<->Rust 转换、receiver 获取与错误处理。

- **glue.rs（转换层）**
  - 对外提供符合 mquickjs/QuickJS C API 约定签名的 `extern "C"` 入口函数。
  - 将 `JSValue` 参数按 RIDL 声明转换为 Rust 参数，并做参数个数/类型检查。
  - 对 singleton / class 需要从运行时环境中获取 Rust 实例（receiver）：
    - singleton：从 `CtxExt`（每个 JSContext 持有的扩展）中取 slot
    - class：从 `this_val` 通过 `JS_GetOpaque` 取回指针
  - 调用 impl 层的 Rust 函数/trait 方法。
  - 将 Rust 返回值转换为 `JSValue`。
  - 在出错时抛 JS 异常（TypeError 等）。

- **impl.rs（实现层）**
  - 用户手写的纯 Rust 实现逻辑。
  - 不负责 JSValue 转换与异常抛出（除非接口明确使用 `any/JSValue` 透传）。

### 1.2 目标

- generator 负责生成可维护、可扩展的 glue 转换层。
- 支持 free function / singleton method / class method 三类调用形态。
- 支持基础类型（bool/int/double/string/void/any）以及 optional/varargs。
- 生成代码具有：稳定顺序、清晰的错误信息、可测试性。

### 1.3 非目标

- 本文不决定聚合集成路径（Plan A/B），只讨论单模块内生成 glue。
- 本文不一次性完成所有复杂类型（map/array/struct）的强类型转换；短期先标注为 `any(JSValue)` 透传，并给出明确演进计划。

## 2. 现状与参考实现

### 2.1 生成器与模板位置

- 模板目录：`deps/ridl-tool/templates/`
  - 关键模板：`rust_glue.rs.j2`、`symbols.rs.j2`、`rust_module_api.rs.j2`、`rust_ctx_ext.rs.j2` 等。
  - 规划新增模板：`rust_api.rs.j2`（仅生成 trait/类型声明；不生成实现 stub）。

### 2.2 runtime 辅助能力

- `ContextHandle::from_js_ctx(ctx)`：从 JSContext 获取到我们 Rust 侧 context。
- `mquickjs_rs::ridl_ext_access::*`：按 slot index 或 name-key 获取 erased singleton slot。
- `mquickjs_rs::ridl_runtime::*`：erased slot / vtable（create/drop）。

### 2.3 stdlib 模块布局（重要约定）

以 `ridl-modules/stdlib` 为例：

- 用户手写实现：`ridl-modules/stdlib/stdlib_impl.rs`
- `ridl-modules/stdlib/src/lib.rs` 通过 `#[path = "../stdlib_impl.rs"]` 引入手写实现，并在 `pub mod impls { ... }` 中导出供 glue 调用的函数/构造器。
- 生成物落在 `OUT_DIR`，由 `include!(concat!(env!("OUT_DIR"), "..."))` 引入。

结论：
- **impl（实现层）应当是用户手写**，存在于模块 crate 源码中（`src/` 或 crate 根文件）。
- generator 应生成一个纯 Rust 的 **api 层**（建议文件名 `OUT_DIR/<module>_api.rs`）：
  - 放置 singleton/class 对应的 trait/interface 声明
  - 放置未来复杂类型声明（struct/enum/type alias 等）与必要的辅助类型
- glue 层依赖 `api + impls`；用户实现层只依赖 `api`，不依赖 glue。

## 3. 生成目标：需要生成哪些内容

每个 module crate 生成（并被 `src/lib.rs` include）：

1) `OUT_DIR/<module>_glue.rs`
- 所有 JS 回调入口（free/singleton/class）
- 参数转换、receiver 获取、错误抛出、返回转换

2) `OUT_DIR/<module>_symbols.rs`
- export 列表/注册元数据（函数列表、class 列表、singleton 列表）

3) `OUT_DIR/<module>_api.rs`
- trait/interface 声明（例如 `ConsoleSingleton` / class interface）
- 未来复杂类型声明（struct/enum/type alias），以及为 glue/impl 双方共享的类型定义
- （可选）free function 的 Rust 侧签名类型（仅声明，不生成 `todo!` stub）

4) `OUT_DIR/ridl_module_api.rs`（已有约定）
- `initialize_module()`：注册符号到引擎
- `ridl_module_context_init()`：初始化该 module 的 ctx-ext slot（singleton constructor）

## 4. generator 内部模型（IR）

### 4.1 抽象：调用形态

对每个可导出的 callable（函数/方法），在 IR 中应表达：

- `CallKind`：
  - `FreeFunction`
  - `SingletonMethod { singleton_key, trait_name }`
  - `ClassMethod { class_name, rust_type, class_id_symbol }`

- `Signature`：
  - params: `[Param]`
  - return: `ReturnType`

- `Param`：
  - name
  - type
  - mode: `Normal | Optional | VarArg`

### 4.2 抽象：类型

短期支持类型集合（v1）：

- `bool` -> `bool`
- `int` -> `i32`
- `double` -> `f64`
- `string` -> `*const c_char`（或后续演进为 `String`/`&str`）
- `any` -> `JSValue`
- `void` -> `()`
- `T?` -> `Option<T>`
- `...T` -> `Vec<T>`

IR 需要保留 “原始 RIDL 类型”与“映射到 Rust 类型”的双信息，以便模板生成。

## 5. 模板组织与开发方式

### 5.1 总原则

- 模板保持 **小而可组合**，避免在一个 `.j2` 中堆大量 if/else。
- 复杂逻辑尽量下沉到 Rust generator 侧（filters/helper），模板只负责拼装。
- 生成代码要稳定（排序规则固定），便于 diff 与回归。

### 5.2 推荐拆分（从现有 rust_glue.rs.j2 演进）

建议将 glue 模板拆成以下逻辑片段（可先逻辑拆分再物理拆分）：

- `glue/common`：
  - `js_throw_type_error` 等异常抛出 helper
  - `missing ctx user_data`、`missing ridl_ext` 等通用错误

- `glue/receiver_singleton`：
  - 通过 name-key 从 ctx-ext 获取 singleton slot，并拿到实例

- `glue/receiver_class`：
  - `JS_GetOpaque(this_val, class_id)` 获取实例

- `glue/convert_arg`：
  - 按 RIDL type 生成参数提取与转换片段

- `glue/convert_ret`：
  - 按 RIDL type 生成返回值转换片段

- `glue/wrapper_free_function`、`glue/wrapper_singleton_method`、`glue/wrapper_class_method`

实际落地时可以先保留一个 `rust_glue.rs.j2`，但 generator 代码层面要按上述模块化去组织过滤器。

### 5.3 filters/helper 设计

在 `deps/ridl-tool/src/generator/` 中提供可单测的 helper：

- `emit_arg_extract(param, idx)` -> `String`
- `emit_arg_check(param, idx)` -> `String`
- `emit_varargs_collect(param, start_idx)` -> `String`
- `emit_receiver(call_kind)` -> `String`
- `emit_call_expr(call_kind, args...)` -> `String`
- `emit_return_convert(ret_type, expr)` -> `String`

模板只需要：
- 迭代 functions/methods
- 对每个调用拼接上述片段

## 6. 类型转换规则（v1）

> 注意：下面描述“应该如何”，最终需要和当前 mquickjs ffi API 具体函数对齐。

### 6.1 bool

- 检查：应识别 JS boolean（可用 tag 检查或 `JS_IsBool` 类 API）
- 转换：转为 Rust `bool`
- 错误：TypeError（`invalid bool argument: <name>`）

### 6.2 int（i32）

- 检查：`JS_IsNumber`
- 转换：`JS_ToInt32`
- 错误：TypeError

### 6.3 double（f64）

- 检查：`JS_IsNumber`
- 转换：`JS_ToNumber`
- 错误：TypeError

### 6.4 string

- 检查：`JS_IsString`
- 转换：`JS_ToCString`（配合 `JSCStringBuf` 或等价 helper）

**生命周期（mquickjs 与 QuickJS 不同）**

mquickjs 的字符串转换走 `JS_ToCString(ctx, v, &mut JSCStringBuf)` 这套 API：
- 项目内 glue/runtime 侧应统一使用 `JSCStringBuf` 路径进行字符串读取。
- 由于 mquickjs **没有** `JS_FreeCString` 这一接口，glue 层不应生成/依赖显式 free 的逻辑。
- 由 `JSCStringBuf` 约束可用生命周期：转换得到的 `*const c_char` 仅在当前 glue 调用栈内有效，不应跨调用保存。

因此：RIDL 的 `string`（v1）建议映射为 impl 层的“只读入参”（例如 `*const c_char` 或立即拷贝为 `String`），并禁止在 impl 中缓存该指针。

### 6.5 any

- 不做转换，直接按 `JSValue` 透传给 impl。

### 6.6 void

- impl 返回 `()`，glue 返回 `JS_UNDEFINED`。

### 6.7 Optional（T?）

建议语义：
- 参数缺失 OR 值为 `undefined` OR 值为 `null` => `None`
- 否则 => `Some(converted)`

### 6.8 Varargs（...T）

- glue 将 `argv[start..argc]` 收集到 `Vec<T>`。
- 单元素转换失败抛 TypeError，并指明下标：`nums[3]`。

## 7. receiver 获取与调用分发

### 7.1 Free function

- 无 receiver。
- glue 直接调用 `crate::impls::foo(...)`。

### 7.2 Singleton

- glue 从 ctx-ext 取实例：
  1) `ContextHandle::from_js_ctx(ctx)`
  2) 获取 `ridl_ext_ptr`
  3) `ridl_get_erased_singleton_slot_by_name(ext_ptr, singleton_key)`
  4) 检查 `slot.is_set()`
  5) 拿到实例指针并调用 trait 方法

要求：
- **禁止硬编码** singleton key 白名单。
- 使用 name-key 查找，避免 slot index 跨 crate 耦合。

### 7.3 Class

- glue 通过 `JS_GetOpaque(this_val, class_id)` 获取 `*mut T`。
- 若为空，抛 TypeError（invalid this）。

#### class_id 的生成与初始化

约定（v1 固化）：
- C 侧 user class id 使用 `JS_CLASS_USER + i` 的编译期常量宏（类似 mquickjs example.c），并生成独立头文件 `mqjs_ridl_user_class_ids.h`。
- Rust 侧同源生成 `ridl_js_class_id.rs`（供 `mquickjs_rs::ridl_js_class_id` include）。
- 排序规则采用与 `ridl-manifest.json` 相同的稳定排序（便于审计），但 `ridl-manifest.json` 本身不作为输入。

命名规范：
- module name：
  - 全局注册：固定为 `GLOBAL`（C 符号中使用 `global`）。
  - 有 module 声明：使用 module path normalize（非 `[A-Za-z0-9_]` 替换为 `_`，含 `-` -> `_`）。
- class id 宏：`JS_CLASS_{GLOBAL|MODULE}_{CLASS}`（全大写）。
- C 侧符号命名域（统一 normalize + 小写，避免与全局函数/其他符号冲突）：
  - class：`js_<module>_class_<class>_...`
  - singleton：`js_<module>_singleton_<singleton>_...`
  - 全局函数：`js_<module>_fn_<function>_...`

初始化：
- 每个 module 的 `initialize_module()` 负责注册 class 并在 JS 侧导出符号（global 或 require 返回对象）。
- 聚合路径保证在 JS 侧使用 class 前已经执行 initialize。

#### 析构

- glue 需要生成 finalizer：将 opaque 指针转回 `Box<T>` 并 drop。

## 8. 错误处理规范

- glue 统一用 helper：`js_throw_type_error(ctx, msg)`。
- 错误消息建议包含：
  - missing argument
  - invalid <type> argument: <name>
  - missing ctx user_data (call ridl_context_init)
  - singleton not initialized
  - invalid this

要求：
- 错误路径必须返回一个 `JSValue`（QuickJS throw value）。

## 9. 与聚合/注册路径的接口

- 每个 module crate 导出：
  - `initialize_module()`：注册函数/class/singleton 的 JS 侧符号
  - `ridl_module_context_init()`：向 ctx-ext 写入 singleton slot（构造实例）

- 聚合层（app 或 framework）负责：
  - 统一安装 ctx-ext vtable
  - 调用所有 module 的 `initialize_module()`
  - 在创建 Context 时调用聚合后的 `ridl_context_init`（填充 slot）

## 10. 测试策略

### 10.1 generator 单测（ridl-tool）

- 对典型 RIDL 输入生成 glue，并对关键片段做断言（或 snapshot）：
  - free function 基础类型转换
  - singleton receiver 获取片段
  - varargs/optional 片段
  - class receiver 获取与 finalizer 片段

### 10.2 repo 集成测试（JS）

- `tests/*.js`：验证导出函数/错误行为/receiver 行为。
- runner 需逐文件跑并输出 PASS/FAIL（当前已具备）。

### 10.3 负向用例

- 参数缺失/类型错误 => TypeError
- singleton 未初始化 => TypeError
- class invalid this => TypeError

## 11. 演进路线

1) 固化 v1（基础类型 + singleton）并清理重复转换代码。
2) 引入 class glue（class_id、JS_GetOpaque、finalizer）。
3) 逐步扩展复杂类型转换（优先从数组/对象的只读访问开始）。

## 12. 待讨论问题（请评审）

1) `string` 的内存生命周期：mquickjs 没有 `JS_FreeCString`，统一使用 `JSCStringBuf`，并将返回指针视为“仅在当前调用栈有效”。
2) class_id 的存放方案是否接受（module static + initialize_module 初始化）？已确认：接受。
3) complex types 的演进：短期全部走 `any(JSValue)` 是否 OK？已确认：接受。

### 12.1 complex types：当前策略与未来演进

当前策略（必须明确标注）：
- RIDL 中出现的 object/array/map/struct 等复杂类型，在 v1 阶段统一映射为 `any(JSValue)`。
- glue 层不做结构化解析，仅做最小校验（例如 `JS_IsObject`/tag 检查）可选；默认直接透传。

未来演进计划（建议）：
1) 阶段 1：保持 `any(JSValue)` 透传，补齐只读辅助能力与错误模型（保证 glue 框架稳定）。
2) 阶段 2：引入轻量 wrapper（如 `Value`/`Object`/`Array`，基于现有 mquickjs-rs 封装）提供安全访问接口。
3) 阶段 3：为 RIDL struct/enum 生成显式序列化/反序列化代码，并补充完整的回归测试用例。
