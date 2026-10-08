# tests/TODO 六项问题处置计划（TDD + 对抗复核）

> 状态：✅ 已对抗复核（2026-10-08，判定"修订后通过"，全部前置条件已吸收）  
> 日期：2026-10-08  
> 依据：探索 agent 全量核查 + 对抗复核 agent 报告  
> 流程：复杂架构项（Phase C/D）经对抗复核后实施；全部 TDD。

## 核查结论摘要

| TODO 项 | 现状 | 处置 |
|---------|------|------|
| 1. v1 glue optional/nullable/union | ✅ 已修复 | A1：补 `bool?` 用例 |
| 2. singleton var/proto var | ❌ 未修复 | C：**仅 plain var**（复核否决 proto var，见下） |
| 3. enum/struct/msgpack 端到端 | ❌ 未修复 | D：3a 命名类型端到端；3b msgpack 另立 |
| 4. union/nullable/string | ✅ 主路径已修复 | B：varargs 两处残留 |
| 5. class glue argc/ctor | ✅ 已修复 | A2：补带参 ctor 用例 |
| 6. runner 兼容层 | ✅ 已修复 | A3：三处收尾 |

**范围裁定（复核确认维持）**：
- 3b msgpack 序列化另立特性（generator 零序列化概念，`SerializationFormat`
  无消费方——非 TODO 修复而是特性开发）。
- Union/Optional varargs 维持编译期明确拒绝（B2 仅升级诊断；后续若实现约
  1-1.5 天，被拒的是语义分歧而非工作量）。

## Phase A：局部修复（0.5 天）

### A1. `bool?` 端到端用例（TODO 1 收尾）
- test_types.ridl 增 `echoBoolOpt(b: bool?) -> bool?`；impl 透传；
  basic.js 先加断言（null→null、true/false 透传）。
- 生成路径已存在（`filters.rs:468-475`），预期直接绿；补的是覆盖缺口。

### A2. 带参 constructor 端到端（TODO 5 收尾）
- test_class.ridl 的 User 增 `constructor(name: string)`；impl 提供
  `user_constructor(name: String)`；basic.js 断言 `new User('x').name === 'x'`。
- 复核确认无破坏：现网 User 全部经 `TestClass.makeUser` 创建，无 `new User()`
  调用点（basic.js:7）；register.h 的 `JS_CLASS_DEF` length 与 ctor 无关。

### A3. runner 收尾（TODO 6）
- TDD：`collect_js_files` 增"跳过 `_` 前缀目录"规则——先写失败测试
  （临时树含 `_diag`，断言不被收集；`tests/_diag/exports_diag.js` 现被
  实际收集执行）→ 实现 → 绿。
- `main.rs:13-27` 默认根改用 `workspace_root()`（消除 CWD 假设）。
- 删 `group_key_for_path` 的 `ridl-modules/tests` 死分支（`test_runner.rs:120-123`）。
- 刷新 `tests/README.md`（删软链接时代描述）。

## Phase B：varargs 残留（1-1.5 天）

### B1. string varargs：失配 + **悬垂指针**（复核加重的真实缺陷）
- 失配：glue `Vec<*const c_char>`（`filters.rs:1570-1584`）vs trait
  `Vec<String>`（`rust_api.rs.j2:27-28,63-64`）。
- **Soundness**：`JS_ToCString` 短串返回调用方栈缓冲（`mquickjs.c:2106-2127`），
  现 glue 逐迭代栈 `JSCStringBuf` 的指针被 push 存活到循环外即悬垂；长串返回
  GC 堆指针受压缩影响。修 glue 逐元素 CStr→String 转换同时消除两者。
- TDD：① generator 单测断言转换代码生成；② **编译级**——新增
  test_varargs 模块（`fn joinAll(sep: string, ...parts: string) -> string`）
  让生成物真实编译；③ e2e：`joinAll('-','a','b','c')==='a-b-c'`、空变参、
  非串参 TypeError。stdlib 的 `...args: any` 路径不受影响
  （Any 分支为 `Vec<Local<Value>>`，已一致）。

### B2. Union/Optional varargs 诊断升级
- `filters.rs:1714-1718` 的 `_` 臂 compile_error 按类型分支给出可诊断文案
  （Optional/Map/Callback/Union 各自说明）；generator 测试固化文案。

## Phase C：singleton plain var（TODO 2，2-3 天，已按复核重设计）

### C.0 范围裁定（复核击穿后的重设计）
- **只做 plain `var`；`proto var` 在 singleton 上维持编译期明确拒绝**。
- 理由（复核实证）：singleton 经 `JS_OBJECT_DEF(name, props)` 注册为
  class_id=NULL 的匿名 class（`register.h.j2:287-288`、`mquickjs_build.h:93`），
  **没有 class id 与可寻址 proto**；现有 validator 本就禁止 singleton proto
  成员（`validator/mod.rs:556-577`）。支持 proto var 需 class-id 全链改造
  （class id 分配、ROM JS_CLASS_DEF、require 表 ensure_class_ids、romclass map），
  属 4-6 天独立特性，另立。
- 现有 test_js_fields/literals 的 .ridl **从未包含** js_fields 语法（"恢复"
  无从谈起）；class js_fields 的真实端到端断言已存在于
  `tests/module/class_members/fields_and_proto.js`。C.5 为**新建**用例。

### C.1 语法
- `grammar.pest:58-59` `singleton_member` 增补 var_member 规则（复用 class
  侧现有规则；proto var_member 被 validator 拒绝而非 grammar 排除——错误
  信息更友好）。

### C.2 字面量类型集（复核修正）
- **`{i32, bool, string, null}`**（端到端真实可用集）。
- 已知既有地雷（另立缺陷单，不在本轮）：class 侧 validator 允许
  I64/F32/F64（`validator/mod.rs:470-527`）但 glue 模板对其 `unreachable!()`
  panic（`rust_glue.rs.j2:503-514`）、C 侧 proto var 同样缺型
  （`register.c.j2:18-44`）。

### C.3 安装机制（复核指认的正确落点）
- **不经 Rust glue**（glue 只做 ctx-slot 分发，拿不到 singleton 的 JSValue）。
- 落点：`mquickjs_ridl_register.c.j2` 的 `JS_RIDL_StdlibInit`（112-124 行）——
  `JS_GetGlobalObject` → `JS_GetPropertyStr(name)` → 逐字段 `JS_SetPropertyStr`
  （writable own 属性），失败 `return -1`。仅覆盖 GLOBAL 模式（module 模式
  singleton 今天未注册到任何 JS 可达位置，维持现状并在文档说明）。
- **emission 转义**：init_literal 存的是解码后原文（`parser/mod.rs:534-539`），
  模板裸嵌会因 `"`/换行生成非法源码——新增 Rust 与 C 两个转义 filter。

### C.4 全部联动点（复核清单）
- `TemplateSingleton` 三处构造点：`generator/mod.rs:359-381`、
  `generator/mod.rs:1150-1174`、`template_modules.rs:32-34`（现显式置空）。
- `singleton_aggregate.rs:102`、`class_ref_rewrite.rs:45`（Singleton 分支
  的 ClassRef 改写否决）。
- **validator**（方案初版遗漏）：singleton js_fields 的名称冲突（与
  methods/properties）、重复名、类型/字面量约束（复用 422-554 逻辑）、
  proto var 拒绝文案升级；错误定位对齐 `js_fields_error_location_test.rs`。

### C.5 测试（TDD）
- ridl-tool 单测：解析、validator 四类错误定位、转义 filter、三构造点透传。
- **新建** singleton var 端到端模块：var 各类型（i32/bool/string/null）、
  转义字符串、读写回传断言。
- 行为不变性：erased-slot 不受影响（js 字段挂 JS 对象，槽位装
  `Box<Box<dyn Trait>>`）；经 singleton 方法返回的 class 实例不安装
  class js_fields（`rust_glue.rs.j2:446` 既有注释）维持。

## Phase D：enum/struct 命名类型端到端（TODO 3a，3-4 天，已补全拒绝矩阵）

### D.1 范围
- enum/struct 作为方法参数/返回的命名类型；顶层 Rust 类型生成已存在
  （`rust_api.rs.j2:122-144`）；msgpack 另立；跨模块命名类型引用 v1 不支持
  （生成类型 crate 局部，可诊断错误）。

### D.2 机制（复核指认）
- 查表 = **渲染前 named-type override pass**（对齐
  `apply_union_rust_ty_overrides` 范式，`mod.rs:5-38`），不改 filter 签名
  （askama filter 无状态带不了聚合上下文）。
- `TemplateStruct` 字段 rust_ty 的 JSValue 静默回退（`mod.rs:1119-1121`）
  改为**硬错误**。

### D.3 转换语义
- struct 参数：JS plain object → 逐字段提取（递归复用
  emit_single_param_extract）；缺失字段 strict 报错；多余字段忽略。
- struct 返回：逐字段注入 JS object。
- enum：纯 C-like（`grammar.pest:34,93` 无载荷语法，复核确认）；
  **JS 字符串形态 = RIDL 原始变体名（如 "RED"）**，与 Rust PascalCase
  变体名（`rust_api.rs.j2:127`）双向映射由生成器产出；非法变体名报错。

### D.4 拒绝矩阵（复核补全的 4 个静默错误面 + 边界）
- struct 字段允许：`基元 / string / 嵌套 struct / array<允许集>`；
  **明确拒绝**：`Traced<T>`、union、**ClassRef**（class 同名改写劫持，
  `class_ref_rewrite.rs:30-34`）、**map**、**`T?` 字段**——全部带诊断。
- **union 成员含命名类型 → 生成期报错**（现状静默丢弃非基元成员，
  `union_types.rs:150-153,195-218`，`string | Address` 会静默变 `UnionString`）。
- validator 补：跨 kind 唯一性（struct/enum/class 重名——现
  `validate_duplicate_definitions` 是空实现 `validator/mod.rs:168-171`）+
  Custom 引用必须解析到本模块命名类型（现显式跳过 `:236-240`）。
- `SerializationFormat` 字段在 3a 忽略（plain struct 默认 Json，仅记录）。

### D.5 测试（TDD）
- 新增 `tests/global/struct_enum/test_struct_enum` 模块：struct Address +
  enum Color + 使用二者签名的方法；先 JS 断言（红）→ 注册表与转换（绿）。
- ridl-tool 单测：override pass、拒绝矩阵全项、唯一性、Custom 解析失败。

## 实施顺序与验证门槛

A（0.5 天）→ B（1-1.5 天）→ C（2-3 天）→ D（3-4 天）；每 Phase 全量回归
（三包 cargo test + JS 全量用例）；C/D 每子任务小步提交。完成后：
tests/TODO.md 归档声明改为逐项处置记录（链接本计划）；知识库沉淀——
C：扩写 `gotcha_quickjs_rom_ridl_mechanism.md`（匿名 class 无 proto、ROM
props 无 bool kind、类型支持矩阵、转义 emission）；D：pattern（named-type
override pass 范式）+ gotcha（union 静默丢成员、TemplateStruct 静默回退——
后者即使 D 不做也值得记）。

## 风险（复核后剩余）

- C 的 `JS_RIDL_StdlibInit` 安装时序：字段安装早于用户脚本执行——需测试
  固化"模块方法首次调用前字段已可读"。
- D 的 override pass 与 union 覆写的执行顺序（先 union 后 named-type？）
  ——实现时以测试钉死。
- B1 glue 改动回归 stdlib any varargs——已在验证门槛内。
