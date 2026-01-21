<!-- planning-meta
status: 未复核
tags: build, context-init, engine, hook, ridl, types
replaced_by:
- docs/ridl/overview.md
- docs/ridl/context-init.md
- docs/build/pipeline.md
-->

> 状态：**未复核**（`build` `context-init` `engine` `hook` `ridl` `types`）
>
> 现行口径/替代：
> docs/ridl/overview.md
> docs/ridl/context-init.md
> docs/build/pipeline.md
>
> 关键结论：
> - （待补充：3~5 条）
# 计划：在现行聚合/薄 vtable/两文件布局下落地 RIDL Class（真 class）（2026-01-14）

状态：草案（已确认关键选择；未确认执行，不进入实现）

## 0. 背景与目标

### 背景（现状已具备）
- RIDL 聚合：`mquickjs_ridl_register.h` 由 `deps/ridl-tool` 生成并被 mquickjs ROM 构建消费。
- Rust 侧模块布局：模块 crate 通过 `mquickjs_rs::ridl_include_module!()` 在 crate root include `api+glue`。
- singleton：已切换为 thin vtable（`RidlErasedSingletonVTable`）+ ctx-ext slot（`ErasedSingletonSlot`），并提供 `RidlSlotWriter`。
- class：已有模板与部分 C 聚合结构（`mquickjs_ridl_class_defs.h.j2`，以及 `rust_api.rs.j2`/`rust_glue.rs.j2` 内部 include 的 class 片段），但存在契约不一致与缺口。

### 本计划的目标（验收口径）
1. RIDL `class` 支持 `new` 构造、实例方法、实例属性 getter/setter、finalizer drop。
2. `proto property`：per-JSContext 共享状态（走 ctx-ext），并支持 `proto readonly`。
3. class-id 管线端到端：`mqjs_ridl_class_id.h`（mquickjs-build 生成）→ `mquickjs-sys` 提供 include_dir → `mquickjs-rs` bindgen `-include mqjs_ridl_class_id.h`，并提供稳定 Rust 常量路径（**自解析**）。
4. 符号保活：class 的 ctor/method/getset/finalizer 以及 class_def/proto_funcs 的 keep-alive 确保不被链接裁剪。
5. 测试：Rust 单测（ridl-tool 生成快照/语法）+ JS 集成用例（`cargo run -- tests`）覆盖 class 关键路径与负例。

非目标（本期不做）
- 复杂类型（struct/enum/map/union）的完整转换体系（继续最小集/现有能力）。
- runtime 动态注册 C API（严格禁止）。

---

## 1. 现有实现与差异点清单

### 1.1 “两文件布局” vs 旧模板路径
- 现行 glue 模板（`templates/rust_glue.rs.j2`）已切换为 `crate::api` + `crate::impls` + `ridl_module_context_init(RidlSlotWriter)`，且避免 `crate::generated::*`。
- 但 class glue 片段（当前在 `rust_glue.rs.j2` include 的 `rust_class_glue.rs.j2`）仍引用 `crate::generated::api::...`，与现行两文件布局冲突；需将其并入/对齐到 `glue.rs` 的统一风格。

### 1.2 class receiver 验证与 opaque 获取不一致风险
- 设计期望：`JS_GetClassID + JS_GetOpaque`（并校验 class_id）。
- 现模板在 `finalizer` / `JS_GetOpaque` 形态上可能与本 fork 的实际 FFI 不一致；实现期必须以 `mquickjs-rs` bindgen 输出为准。

### 1.3 class ctor 用户 hook 设计不理想
- 现模板要求每个 class 必须提供 `crate::impls::new_<class>(ctx, this_val, argv_vec)`（JS 泄漏到 impl）。
- 需要收敛为“glue 负责转换，impl 只接收纯 Rust 参数”。

### 1.4 class-id 常量路径需要统一
- 当前仓库已在 `deps/mquickjs-rs/build.rs` 自解析 `mqjs_ridl_class_id.h` 生成 `OUT_DIR/ridl_class_id.rs`。
- 本计划明确：对外稳定路径应基于“自解析输出 + re-export”，避免 bindgen 输出受版本/生成细节影响。

### 1.5 proto property 的运行时存储模型未落地
- AST/grammar 已支持 `PropertyModifier::Proto` 与 `proto` 语法。
- 但 glue/聚合/ctx-ext 尚未为 proto property 提供 per-ctx 存储与访问契约（slot？命名？drop 时机？）。

### 1.6 C 聚合侧 class def vs Rust keep-alive 的契约需确定
- C 模板 `mquickjs_ridl_class_defs.h.j2` 生成 `js_<module>_<class>_class(void)` keep-alive stub。
- Rust aggregated_symbols 当前未必引用到 `*_class`（以及 ctor/method/finalizer）；需要补齐。

---

## 2. 关键选择（已确认）

1) class-id 常量：**自解析**（以 `mquickjs-rs/build.rs` 读取 `mqjs_ridl_class_id.h` 生成的 Rust 常量为 SoT）。

2) class 实例承载：接受与 singleton 同款 **thin-pointer** 模式：`Box<Box<dyn Trait>>` 或等价薄指针容器（严禁 fat pointer 直接进 `*mut c_void`）。

3) v1 必须包含：**proto property**（per-JSContext 共享）。

---

## 3. 稳定契约（生成器/运行时/模块）

### 3.1 生成物（模块 crate / 聚合）
- 每个模块 crate（RIDL module）生成并在 crate root include：
  - `pub mod api`：纯 Rust trait/struct surface（不接触 JSValue/ctx-ext）。
  - `pub mod glue`：`#[no_mangle] extern "C" fn js_*` + 参数转换/抛异常 + receiver/slot/opaque 获取。
- 聚合产物：
  - `mquickjs_ridl_register.h`：包含 class defs + `JS_PROP_CLASS_DEF` 注入全局。
  - `ridl_symbols.rs`（或 app-side aggregated_symbols）：强引用所有 `js_*` 导出与 class keep-alive stub。

### 3.2 class-id 常量访问（拟定）
- `mquickjs_rs::ridl_class_id::RIDL_CLASS_<module>_<class>`（由 mquickjs-rs 统一 re-export；生成器只依赖此路径）。

### 3.2.1 产物收束约束（拟定）
- class 的 trait 必须生成到模块 `api.rs`。
- class 的 C ABI 胶水（ctor/method/get/set/finalizer）必须生成到模块 `glue.rs`。

### 3.3 impl 层稳定入口（拟定）
- class ctor：`crate::impls::<class>_constructor(<rust params>) -> Box<dyn crate::api::<Class>Class>`
- class methods/getset：`dyn <Class>Class`
- proto backing（per-ctx）：`crate::impls::<class>_proto_create() -> Box<dyn crate::api::<Class>Proto>`（每个 ctx 创建一次）

---

## 4. 生命周期与存储

### 4.1 class 实例
- ctor glue：
  - `obj = JS_NewObjectClassUser(ctx, CLASS_ID)`
  - `inst = <impl ctor>(converted_args...)`
  - 分配薄指针承载：`Box::into_raw(Box::new(inst)) as *mut c_void`（其中 `inst: Box<dyn Trait>`，外层再 box 一次形成 thin 指针）
  - `JS_SetOpaque(ctx, obj, opaque_ptr)`
- finalizer glue：
  - 仅做 drop（禁止 JS API），确保 exactly once。

### 4.2 proto property（per-ctx 共享）
- 存储位置：ctx-ext（与 singleton 同源），但不复用 singleton slot。
- 访问机制（冻结）：扩展 ctx-ext vtable 提供按 key 查找：
  - 运行时新增：`RidlCtxExtVTable::get_proto_by_name(ext_ptr, name_ptr, name_len) -> *mut c_void`
  - 返回：非空 = proto backing（thin-pointer：`*mut Box<dyn <Class>Proto>`）；空 = 未找到（glue 抛 `TypeError("missing proto state")`）。
  - vtable 不负责分配/抛异常；proto backing 统一在 `ridl_context_init` 创建并由 ctx teardown 统一 drop。
- key 规范（冻结）：
  - `proto:<module_ns>::<class_name>`（ASCII）
  - `module_ns` 优先 RIDL `module foo.bar`，否则为 `<crate_name>`（cargo 包名 normalization：非 `[A-Za-z0-9_]` -> `_`）
  - 全局唯一性：`module_ns + class_name` 不可重复；冲突生成器报错。
- proto backing 粒度（已确认）：每个 class 一份（`<Class>Proto` 包含多个 proto property 的 get/set）。

---

## 5. 实施阶段划分（确认执行后才进入实现）

### Phase 0：接口确认与探测（实现期第一步）
- 确认本 fork 的 FFI：
  - `JS_NewObjectClassUser` / `JS_SetOpaque` / `JS_GetOpaque` / `JS_GetClassID` 的函数签名。
- 明确 class-id 常量 re-export 的最终路径（`mquickjs_rs::ridl_class_id::*`）。

交付物：可编译的最小探测/对齐提交（如需）。

### Phase 1：mquickjs-rs（class-id 稳定导出）
- 以 `OUT_DIR/ridl_class_id.rs` 为 SoT：
  - 在 `mquickjs-rs` crate 中新增模块 `ridl_class_id`，`include!(concat!(env!("OUT_DIR"), "/ridl_class_id.rs"))`。
  - 生成器与 glue 全部只依赖 `mquickjs_rs::ridl_class_id::RIDL_CLASS_*`。

交付物：`cargo test` 通过。

### Phase 2：ridl-tool（模板与符号保活）
- 在 module 的两主模板中落地 class（产物收束为 `api.rs` + `glue.rs`，不新增输出文件）：
  - `rust_api.rs.j2`：扩展 `IDLItem::Class` 渲染，把 `<Class>Class`/`<Class>Proto` trait 输出到 `api.rs`。
  - `rust_glue.rs.j2`：扩展 `IDLItem::Class` 渲染，把 ctor/method/get/set/finalizer 的 `js_*` 入口输出到 `glue.rs`。
  - 现存 `rust_class_api.rs.j2`/`rust_class_glue.rs.j2` 仅作为“模板片段”保留或被内联（实现细节），不形成额外生成文件。
- class glue 对齐点：
  - 不再引用 `crate::generated::api`，统一走 `crate::api`。
  - ctor 改为纯 Rust 参数，不向 impl 传 `JSValue/argv`。
  - receiver 校验：`JS_GetClassID` 比较 `mquickjs_rs::ridl_class_id::RIDL_CLASS_*`。
  - opaque：thin-pointer 反序列化为 `*mut Box<dyn Trait>` 并解引用。
- 扩展 `ridl_symbols.rs` keep-alive：
  - 引用 ctor/method/getset/finalizer
  - 引用 `js_<module>_<class>_class`（C keep-alive stub）

交付物：ridl-tool 单测（snapshot/片段断言）覆盖 class 生成。

### Phase 3：proto property（ctx-ext vtable 扩展 + 聚合输出）
- `mquickjs-rs`：扩展 `RidlCtxExtVTable` 增加 `get_proto_by_name`。
- 聚合产物：生成 proto state 字段、name-key 映射与 drop。
- glue：对 `proto property` getter/setter 走 `ctx -> ext_ptr -> get_proto_by_name -> dyn Proto`。

交付物：新增 JS 用例验证“同 ctx 多实例共享”。

### Phase 4：集成测试
- 新增 `tests/*.js`：
  - `new` 两个实例，调用方法验证实例态。
  - `proto readonly property` 跨实例读取一致。
  - 负例：错误 this/错误参数类型抛 TypeError。
- 运行：`cargo test` + `cargo run -- tests`。

---

## 6. 验收标准

- `cargo test` 全绿。
- `cargo run -- tests` 全绿（并新增 class 用例至少 1 个文件，含负例）。
- `mquickjs_ridl_register.h` 中包含 class 的全局导出与 class defs。
- 链接产物中 class 入口符号不被裁剪（依赖 keep-alive 机制）。

---

## 7. 风险与回滚策略

风险
- FFI 签名与模板假设不一致（尤其 `JS_GetOpaque` 变体）。
- proto property 需要扩展 ctx-ext vtable/聚合输出，改动面较大。
- 若 thin-pointer 承载做错，可能 UB。

回滚
- 若 proto 链路阻塞：先落地 class ctor+method（proto 延后），但本计划已要求 v1 做 proto，需你确认是否允许降级。

---

## 8. 状态
- [ ] 未开始实现（等待“确认执行实现”）
