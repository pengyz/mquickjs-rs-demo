<!-- planning-meta
status: 未复核
tags: ridl, tests
replaced_by:
- docs/ridl/overview.md
-->

> 状态：**未复核**（`ridl` `tests`）
>
> 现行口径/替代：
> docs/ridl/overview.md
>
> 关键结论：
> - （待补充：3~5 条）
# RIDL 关键字：`property` → `prop`（不兼容）规划

## 背景与动机
当前 RIDL 中用于声明宿主绑定属性的关键字为 `property`。在实际编写时该关键字偏长，且与 `proto property`、`readonly/readwrite` 等组合后更显冗长。

鉴于：
- RIDL 仍处于开发期；
- 项目可接受 breaking changes；

我们计划将关键字 **`property` 改为 `prop`**，并且 **不提供 `property` 兼容解析**（即旧语法直接报错）。

> 注意：本规划仅定义目标与实施步骤，本轮不立即实施。

## 目标
- RIDL 语法中：
  - 用 `prop` 替代 `property` 表示宿主绑定属性。
  - 删除/禁用 `property` 关键字。
- 生成/语义保持不变：`prop` 仍然是 getter/setter（或 read-only）语义，走现有宿主绑定路径。

## 非目标
- 不改变 `var`（JS-only 字段）语义。
- 不引入省略关键字的隐式规则（避免歧义）。
- 不在本次改动中调整其它关键字（如 `singleton`、`class`、`proto`）。

## 影响范围（需要全量检索与替换）
### 1) RIDL 语法/解析器
- pest grammar：
  - `property` token/规则替换为 `prop`。
  - 相关规则（readonly/readwrite/proto variants）同步更新。
- parser：
  - `Rule::*prop*` 分支更新。
  - 错误提示：遇到旧 `property` 应报错并提示使用 `prop`。

### 2) AST / Validator
- AST 通常无需变更（仍为 Property 节点），但：
  - 若当前 AST/Rule 命名含 `property`，需要评估是否重命名以保持一致性（可选）。
- validator 规则不变，仅确保新语法路径覆盖完整。

### 3) generator
- 只要 AST 不变，生成逻辑通常不变。
- 若模板/代码中有关键字文本匹配（不应有），需要移除。

### 4) RIDL modules
- `ridl-modules/*/src/*.ridl`：全部将 `property` 替换为 `prop`。

### 5) tests
- parser/validator 单元测试里所有 `property` 输入样例需替换。
- JS integration tests（`tests/*.js`）通常不涉及关键字，但若引用 RIDL 模块，可能因生成变更间接受影响。

### 6) docs
- `README.md`、`docs/`、`doc/` 中所有示例与语法说明更新。

## 实施步骤（建议拆分 PR/提交）
1. 修改 pest grammar：`property` → `prop`。
2. 修改 parser：匹配新规则；对旧关键字报错（不兼容）。
3. 更新 ridl-modules 内所有 RIDL 文件。
4. 更新 tests（parser/validator + 集成 smoke）。
5. 运行全量验证：
   - `cargo run -q -p ridl-builder -- prepare`
   - `cargo test -q`
   - `cargo run -- tests`
6. 更新 docs（如本仓库文档要求需中文，必要时新增专门说明）。

## 验证清单
- 新语法 `prop` 在 class/singleton 中可正常解析与生成。
- `property` 输入必须稳定报错，且错误信息清晰。
- 现有 RIDL 模块生成结果一致（仅语法关键字变化）。
- 所有 Rust tests 与 JS integration tests 通过。

## 风险与缓解
- 风险：一次性替换面广，容易漏改。
  - 缓解：实施前先用全仓库 grep 列出所有 `property` 出现位置，并在 CI/本地验证中确保无残留。
- 风险：外部使用者（若有）被 breaking。
  - 缓解：由于开发期且明确不兼容，直接报错并给出迁移提示即可。

## 状态
- 规划已完成，等待进入实施。
