<!-- planning-meta
status: 未复核
tags: engine, require, ridl, tests
replaced_by:
- docs/ridl/overview.md
- docs/ridl/require-materialize.md
-->

> 状态：**未复核**（`engine` `require` `ridl` `tests`）
>
> 现行口径/替代：
> docs/ridl/overview.md
> docs/ridl/require-materialize.md
>
> 关键结论：
> - （待补充：3~5 条）
# RIDL module+require V1：实施计划（按层切分）

状态：实施中（用户已要求开始实现）

> 目标：在不引入“复杂功能耦合”的前提下，把 module namespace + require(version select) 的 V1 走通；测试优先，逐层推进。

---

## 0. 总体策略（避免混淆）

- **按层推进**：parser → generator(IR/模板) → C 注入 → 集成。
- **每层都加测试**：parser 单测覆盖语法边界；generator 单测覆盖输出稳定性；Rust 单测覆盖 glue 侧行为（如有）；JS 集成覆盖行为。
- **每次只引入一个可观察行为**：例如 Phase1 只改 parser 与 parser test，不动 generator。

---

## 1. Phase1（parser）：冻结 module 语法与版本号

### 1.1 需求点
- `module` 声明必须携带版本：`module <base>@<version>;`
- 不允许空格（`@` 前后均不可有空白）
- 版本格式：`MAJOR[.MINOR[.PATCH]]`（三段规范化用于比较）

### 1.2 代码改动
- `deps/ridl-tool/src/parser/grammar.pest`
  - 使 `module_decl` 中 `@` 与 `version` 成为 **必选**，且 `@` 后不允许 `WS`。
  - 扩展 `version` 规则支持最多三段（当前最多两段）。
- `deps/ridl-tool/src/parser/mod.rs`
  - `parse_module_decl()`：version 变为必选；解析后保存原始版本字符串（或保存规范化三段，视后续 generator 需要）。

### 1.3 parser 单测（必须新增/调整）
- 已存在的 module 相关用例需更新（例如 `test_module_without_version*` 应改为期望失败）。
- 新增：
  - `module system.network@1;` / `@1.2;` / `@1.2.3;` 均可
  - `module system.network@1.2.3.4;` 失败
  - `module system.network @1.2;`（空格）失败
  - `module system.network@ 1.2;`（空格）失败
  - `module system.network@>1.2;` 失败（module 声明不接受比较符，比较符仅用于 require spec）

交付标准：`cargo test -p ridl-tool` 通过。

---

## 2. Phase2（generator）：module object 与 require-table（仅生成，不注入 require）

### 2.1 需求点
- module 模式：同一个 RIDL 文件内若声明 module，则导出不再进 global，而是收敛到 module object 上。
- module object：
  - object class（`func_name = NULL`）
  - 导出 `fn` / `class` 作为 object props（非 proto）
  - `singleton` 在 module 下报错
- require-table：按 base + version 三元组 + module_class_id（`JS_CLASS_*` 常量）生成。

### 2.2 代码改动（预期）
- `deps/ridl-tool/src/generator/*`
  - IR 增加 module 维度聚合：按 module full-name 分组。
  - 生成 `mquickjs_ridl_register.h` 时：
    - 生成每个 module 的 object class_def + props 列表
    - 生成 require-table 静态数据（但暂不生成 require() 函数）

### 2.3 generator 单测
- 对一个小 RIDL 输入（内含 module + fn + class）
  - 断言输出中存在：
    - `func_name = NULL` 的 class_def（或模板等价输出）
    - object props 中存在对应 fn/class 的条目
    - require-table entry 的 base/version 拆分正确
- 对 module 内 singleton：断言 generator 报错。

交付标准：`cargo test -p ridl-tool` 通过，且输出顺序稳定（快照或字符串包含断言）。

---

## 3. Phase3（C 注入）：require() 查表 + 版本选择

### 3.1 需求点
- global 注入 `require(spec)`
- spec 解析：
  - 无版本：`base` → 取最高
  - 精确：`base@1.2`
  - 约束：`base@>1.2` / `>=` / `<` / `<=`
  - 无空格
- 找不到：`TypeError("require <spec> failed: module not found.")`

### 3.2 代码改动（预期）
- `deps/mquickjs/...` 中 stdlib 注入点（例如 `mqjs_stdlib.c` 的 global object defs）
  - 添加 `require` 的 `JS_CFUNC_DEF`。
- require() 的实现放在 C 侧，并引用 generator 生成的 require-table。

### 3.3 测试
- C 侧单测如不方便，则以 JS 集成测试覆盖（Phase4）。

---

## 4. Phase4（集成）：新增 module+version 用例 + JS 测试

- 新增一个 tests crate（独立于 stdlib）：至少提供两个版本的同 base module（例如 `system.network@1.0` 与 `system.network@1.2`）。
- JS 集成测试覆盖：
  - `require("system.network")` 选最高版本
  - `require("system.network@1.0")` 精确
  - `require("system.network@>1.0")` 选 1.2
  - `require("system.network@<1.2")` 选 1.0
  - `require` 每次返回新实例
  - not found 的错误消息精确匹配

交付标准：
- `cargo test`
- `cargo run -- tests`

---

## 5. 回滚/隔离策略

- Phase1 不改生成器；Phase2 不改 C 注入；Phase3 才引入 require()。
- 任一 phase 出现 bug，优先在该层用单测定位并修复，避免跨层调试。
