# 测试重构推进过程中的问题与待办（Follow-ups）

> 目标：先跑通（全量 JS 用例可执行并通过），过程中发现的缺陷/不一致先记录，不在本阶段强行一次性修完。

## P0：阻塞跑通的问题（必须逐个清零）

### 1) 生成命名规则非 Rust idiomatic（全小写化/压平）
- 现象：
  - `TestDiagnostics` -> `TestdiagnosticsSingleton`
  - `TestFn` -> `TestfnSingleton`
  - `getName` -> `getname`
  - ctor 符号期待：`ridl_create_testdiagnostics_singleton`（无下划线分隔）
- 影响：手写 impl 必须跟随生成产物，否则无法编译/链接。
- 待办：定义并实现稳定的命名规范（Trait/struct/方法 snake_case 等），并提供迁移策略。

### 2) v1 glue 对参数类型的支持不完整
- 现象：`compile_error!("v1 glue: unsupported parameter type for ...")`
  - 例如 `int?`、`string?`、`(A|B|null)?` 等会触发。
- 影响：很多 nullable/union 相关端到端用例无法直接落地，只能先绕开。
- 待办：补齐 v1 glue 对 optional/nullable/union 等类型的 JSValue 转换与 runtime check。

### 3) mode 语法/语义不完整
- 现象：parser 仅接受 `mode strict;`，但 strict 又限制 any；显式 `mode default/loose` 不被识别。
- 现状 workaround：通过“省略 mode_decl”走默认 FileMode（以覆盖 any）。
- 待办：
  - 明确 mode 的合法取值集合与语义
  - 支持显式 `mode default;`（或其它名称）
  - 更新文档与测试。

### 4) singleton 的 var/proto var 成员解析在聚合链路失败
- 现象：ridl-builder 聚合报错 `expected singleton_member`（当 singleton 内出现 `var ... = literal;`）。
- 影响：js-only fields 的端到端测试无法落地。
- 待办：对齐聚合解析入口/grammar 与 ridl-tool 语法，使 singleton_member 接受 var/proto var。

### 5) enum/struct（含 msgpack struct）在聚合链路不通过
- 现象：聚合失败，提示 `expected EOI, definition, or module_decl`（在 enum 行）。
- 影响：struct/enum 的端到端测试暂缓。
- 待办：确认聚合链路使用的解析入口/语法版本，并对齐到 ridl-tool 的完整 grammar。

## P1：测试组织与基础设施

### 6) 测试目录与 runner 扫描
- 现状：runner 未传参时默认扫描 `tests/` 与 `ridl-modules/`。
- 待办：进一步明确“框架用例 vs 功能模块用例”的选择规则（本次先不做筛选），并完善 runner 的扫描/过滤策略。

### 7) 测试模块与功能模块的同构组织
- 约定：功能模块（如 stdlib）与框架测试模块在组织上同构（`src/*.ridl` + `tests/*.js`），差异仅在存放位置/用途。
- 待办：补齐 README/约定文档，并将旧根 tests 的用例完全迁移。

## P2：后续扩展

### 8) module_mode 测试套件
- 现状：module_mode 尚未支持，本阶段专注 global_mode。
- 待办：module_mode 实现后，按同样语法域维度补齐覆盖，并解决 GLOBAL vs MODULE 差异用例。

---

## 本文档维护规则

- 任何在“先跑通”过程中发现的系统性问题，优先在此记录。
- 每个问题尽量包含：现象、影响、当前 workaround、后续待办。
