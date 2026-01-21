<!-- planning-meta
status: 未复核
tags: context-init, engine, ridl
replaced_by:
- docs/ridl/overview.md
- docs/ridl/context-init.md
-->

> 状态：**未复核**（`context-init` `engine` `ridl`）
>
> 现行口径/替代：
> docs/ridl/overview.md
> docs/ridl/context-init.md
>
> 关键结论：
> - （待补充：3~5 条）
# 按 RIDL 文件为 module 维度组织生成器数据模型

日期：2026-01-15

## 背景与问题
当前 ridl-tool 在生成 `mquickjs_ridl_register.h` / `mqjs_ridl_stdlib.h` / `ridl_symbols.rs` 等聚合产物时，存在“丢失文件维度（module 维度）”的结构性风险：

- generator 在聚合阶段把所有 RIDL 文件的 AST items 扁平化为 `all_classes/all_functions/all_singletons/...`。
- module name 语义上属于 RIDL 模块（本 repo 规则：一个 `.ridl` 文件就是一个 module；文件内最多一个 `module xxx;`；缺省 GLOBAL）。
- 扁平化后如果不严格把 module_name 复制到每一种语法元素，会导致符号命名、JS_CLASS_*、keepalive、ctx-slot vtable ident 等在不同产物间不一致，引发链接错误。

本次实现目标是把“module”升级为生成器的一等数据结构，避免靠散落的字段复制维持语义。

## 规则约束（来自 AGENTS.md 与本次确认）
- 每个 `.ridl` 文件视为一个 module。
- 文件内最多一个 `module <path>` 声明。
- 未声明 module 时，module_name 缺省为 `GLOBAL`。
- 禁止硬编码 module 前缀/白名单；所有命名必须从数据流推导。

## 设计方案

### 数据结构
在 ridl-tool 生成器侧新增：

```rust
struct TemplateModule {
  /// 该 RIDL 文件对应的 module_name（normalize 后；缺省 GLOBAL）
  module_name: String,
  module_decl: Option<ModuleDeclaration>,
  file_mode: FileMode,

  interfaces: Vec<TemplateInterface>,
  functions: Vec<TemplateFunction>,
  singletons: Vec<Singleton>,
  classes: Vec<TemplateClass>,
}
```

同时保留：
- `TemplateClass.module_name`（以及必要时在其它模板元素上携带 module_name），但它由 `TemplateModule` 统一赋值，避免“手工到处复制”。

### 生成流程调整

#### 1) 聚合入口：`generate_register_h_and_symbols`
- 从“扁平 all_* 收集”改为：逐个 ridl 文件解析 -> 构造 `TemplateModule` -> 收集 `modules: Vec<TemplateModule>`。
- 原来需要全局视图的产物：
  - `mquickjs_ridl_register.h`：仍然是集成头文件（一次 include 全部模块），但输入改为 `modules`。
  - `mquickjs_ridl_class_defs.h`：同上。
  - `ridl_symbols.rs`：同上。

#### 2) class id 分配
- JS_CLASS_* 的数值分配仍然需要全局顺序。
- 方案：在 generator 内对 `modules` 做 `flatten_classes()` 得到带 module_name 的 class 列表，然后稳定排序（例如按 `(module_name, class.name)`），用于分配 index。
- 注意：宏名仍然是 `JS_CLASS_<MODULE>_<CLASS>`（MODULE 为 GLOBAL 时就是 GLOBAL），保持可读与稳定。

### 模板调整
把涉及 class/function/singleton 的模板输入从 `classes/functions/...` 改为 `modules`，并在模板内使用 `for module in modules { for class in module.classes { ... } }`。

其中：
- C/Rust 符号命名必须统一使用 `module.module_name` 或 `class.module_name`。
- keepalive `aggregated_symbols` 仍按 class/method 生成，但从 modules 遍历展开。

## 迁移步骤（实施顺序）
1. 新增 `TemplateModule` 结构与构造逻辑。
2. 改造 `generate_register_h_and_symbols` 使用 modules 作为主输入。
3. 改造三份模板：
   - `mquickjs_ridl_register_h.rs.j2`
   - `mquickjs_ridl_class_defs.h.j2`
   - `aggregated_symbols.rs.j2`
4. 在 generator 内实现 `flatten_*` 用于 class id 分配等全局需求。
5. 运行验证：
   - `cargo run -p ridl-builder -- prepare`
   - `cargo test`
   - `cargo run -- tests`

## 验收标准
- 不再需要 template-level 的“module_name 前缀”来影响符号命名；所有命名从 `TemplateModule.module_name` 数据流推导。
- `mqjs_ridl_stdlib.h` 中引用的 js_* 符号与 Rust glue 导出、keepalive 引用完全一致。
- `cargo test` 与 `cargo run -- tests` 全部通过。

## 备注（已知非阻塞警告）
- 目前仍有 C warnings（函数指针签名/ finalizer 签名不匹配）属于独立问题，不作为本计划的 blocker。
