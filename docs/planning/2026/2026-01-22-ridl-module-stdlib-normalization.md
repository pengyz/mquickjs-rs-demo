# 归一化：stdlib 初始化阶段完成 RIDL module/class 初始化（Phase1 proto var）

- 状态：进行中
- 日期：2026-01-22

## 1. 背景与问题

当前 module 模式 Phase1（proto var）在运行时存在时序冲突，导致 proto var 丢失或需要复杂幂等逻辑：

- proto var 若在 `context_init` 期间写入 class prototype（`ctx->class_proto[class_id]`），随后 require 触发的 ROMClass materialize 阶段会通过 `proto->props = rc->proto_props` 覆盖 property list 指针，导致先前写入丢失。
- require 作为延迟初始化触发点，使得初始化路径分散在：stdlib_init / context_init / require / 生成 glue，多处交织，必须对引擎执行时序非常熟悉才能保证不重置、不丢失。

目标是把“结构性初始化”归一到 Context 创建阶段一次性完成，运行期 require 只负责创建 module 实例。

## 2. 目标语义

1) module/class 初始化在 Context 创建阶段一次性完成：
- class proto/ctor 初始化（proto_props 绑定）
- module exports（ROMClass -> ctor function）挂载到 module prototype（proto exports）
- proto var 一次性安装到 class prototype

2) require 退化为：仅创建并返回 module 实例（不 materialize exports，不安装 proto vars）。

3) module 模式隔离：
- `globalThis` 只提供 `require` 与 `__ridl_modules`。
- module entries（`name@version`）不再注入到 `globalThis`。

4) proto var 语义：shared mutable like static。
- 重复 require 不得重置用户对 prototype 上 proto var 的修改。

## 3. 已有事实（代码证据）

### 3.1 stdlib 注入点
`mqjs_stdlib_template.c` 会展开 `JS_RIDL_EXTENSIONS` 到 `js_global_object[]`，作为 stdlib ROM 表的 global_object_offset 一层条目来源。

### 3.2 已新增 C API（deps/mquickjs）
子模块 `deps/mquickjs` 已新增：
- `JSValue JS_MaterializeROMClass(JSContext *ctx, JSValue val);`
- `int JS_MaterializeModuleClassExports(JSContext *ctx, JSValue module_obj);`

其中 `JS_MaterializeModuleClassExports` 当前语义为：扫描 module 实例 own props 和一层 prototype props，遇 ROMClass 则 materialize，并写回到 module 实例 own property。

### 3.3 require table
runtime glue `mquickjs_ridl_register.c` 提供 `js_ridl_require_table`，并保证：
- `ensure_class_ids[0]` 为 module_class_id

## 4. 归一化方案（最小侵入）

选择：
- exports 采用 proto exports（挂到 module prototype）。
- 调用点选择：B（放外层 glue，在创建 ctx 后立刻调用一次 stdlib 后初始化函数）。

### 4.1 总体时序

Context 创建：
1) `JS_NewContext2` 内部执行 `stdlib_init`（现有机制）
2) 外层 glue 在获得 `JSContext* ctx` 后，调用一次 `JS_RIDL_StdlibInit(ctx)`：
   - 遍历 `js_ridl_require_table`
   - 对每个 module：materialize exports 到 module prototype
   - 对所有 class：一次性安装 proto vars

运行期：
- `require(spec)` 仅 new module instance 并返回

### 4.2 mquickjs 侧最小修改

（已完成，见实际实现与最新 docs/ridl/* 口径）

### 4.3 runtime glue：新增 `JS_RIDL_StdlibInit`

新增一个导出函数（位置可选）：

- 推荐放在 `deps/mquickjs-rs` 的 C glue 中（与 require.c 同级或同编译单元），以便：
  - 可见 `js_ridl_require_table`
  - 可调用 `JS_MaterializeModuleClassExportsToProto`
  - 可调用生成的 `ridl_install_proto_vars`

逻辑：
- 对每个 require entry：
  - `mid = ensure_class_ids[0]`
  - `module_proto = ctx->class_proto[mid]`（必要时 ensure）
  - `JS_MaterializeModuleClassExportsToProto(ctx, module_proto)`
- 安装 proto vars：
  - 遍历所有 entry 的 `ensure_class_ids[1..]`（去重）
  - 对应 class_id 的 proto 上写入 literal 初值

### 4.4 ridl-tool 生成支持

- 生成 `ridl_install_proto_vars(ctx, ensure_class_ids, ...)` 的通用实现：
  - 以 IR 收集的 proto var 列表为输入
  - 为每个 `(class_id, name, literal)` 写入 `ctx->class_proto[class_id]` 的 property
- 移除在 `context_init` 阶段的 proto var 安装块（避免早装被覆盖）。

## 5. 正确性论证要点

- 归一化后，proto var 写入发生在：
  - `stdlib_init_class` 绑定 proto_props 完成之后
  - 且 `JS_MaterializeROMClass` 不再进行晚覆盖
  因此不会被覆盖丢失。

- require 不再触发 materialize/proto-var 安装，因此重复 require 不会重置用户修改。

- exports 挂在 module prototype，require 返回的新实例通过原型链可访问 ctor。

## 6. 测试计划

- module Phase0：不污染 globalThis、require 返回新实例、导出可用
- module Phase1：
  - 首次 require：`MFoo.prototype.px === 100`
  - 用户修改：`MFoo.prototype.px = 7`
  - 再次 require：仍为 7（不重置）

## 7. 风险与权衡

- exports 从“实例 own”变为“prototype”会改变反射/枚举行为；按当前优先级（RIDL 语义优先）可接受。
- 调用点放外层 glue：要求所有 ctx 创建路径都执行一次 `JS_RIDL_StdlibInit`。

