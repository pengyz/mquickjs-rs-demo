<!-- planning-meta
status: 未复核
tags: build, engine, ridl, tests
replaced_by:
- docs/ridl/overview.md
- docs/build/pipeline.md
-->

> 状态：**未复核**（`build` `engine` `ridl` `tests`）
>
> 现行口径/替代：
> docs/ridl/overview.md
> docs/build/pipeline.md
>
> 关键结论：
> - （待补充：3~5 条）
# 2026-01-13：重构 ridl-builder，复用 ridl-tool 的聚合生成逻辑（register.h 等）

## 背景与问题
当前 `ridl-builder` 在 `ridl-builder/src/aggregate.rs` 中以“拼字符串”的方式生成：
- `target/ridl/aggregate/ridl-manifest.json`
- `target/ridl/aggregate/mquickjs_ridl_register.h`

与此同时，`ridl-tool` 已经存在成熟的模板化生成能力（例如 `deps/ridl-tool/templates/mquickjs_ridl_register_h.rs.j2`），但聚合场景没有通过模板生成，而是由 ridl-builder 重新实现一份 C 头文件生成逻辑。

这造成了**重复实现与漂移风险**：
- 最近的 bug：`console.enabled` 的 readonly property 在 Rust glue 里被生成了 getter，但 `ridl-builder` 聚合的 `mquickjs_ridl_register.h` 没有生成 `JS_CGETSET_DEF("enabled", ...)`，导致运行时 `console.enabled` 为 `undefined`。

用户选择方案：**以 ridl-tool 模板为准（选项 1）**，允许宏名变更（从 `MQUICKJS_RIDL_REGISTER_H` 迁移到模板当前的 `MJS_RIDL_REGISTER_H` 及其宏层级），并同步修正消费方。

## 目标
1. `ridl-builder` 不再拼字符串生成 `mquickjs_ridl_register.h`；聚合 register.h 由 `ridl-tool` 通过模板生成。
2. `ridl-builder` 尽量只负责：模块发现/选择 + 调度工具链（ridl-tool / mquickjs-build）。
3. 迁移后保证：
   - singleton readonly properties（如 `console.enabled`）在聚合 register.h 中正确生成 `JS_CGETSET_DEF`。
   - `cargo test --test js_smoke` 通过。
   - `cargo run -- tests`（JS 集成用例）通过。

## 现状盘点：ridl-builder 中的重复生成点
### 1) ridl-manifest.json
- 当前位置：`ridl-builder/src/aggregate.rs::write_manifest` 通过 `push_str` 拼 JSON。
- 风险：格式/字段扩展时缺少 schema/serde 约束。
- 处理建议：可以保留（它本质是 ridl-builder 的“选择快照”）；或改为 `serde_json` 输出以减少手工拼接。

### 2) mquickjs_ridl_register.h（重点）
- 当前位置：`ridl-builder/src/aggregate.rs::write_ridl_register_h`。
- 重复点：`ridl-tool` 已存在 `mquickjs_ridl_register_h.rs.j2` 模板；拼接版与模板版会持续漂移。
- 本次重构：完全移除拼接生成；改为调用 ridl-tool 聚合生成。

## 设计方案
### 总体策略
- 在 `ridl-tool` 内新增一个面向“聚合输出”的生成入口（子命令或库 API），输入为：
  - `ridl-manifest.json`（包含模块 crate_dir + ridl_files）
  - 或（更简单）直接传 `--ridl-file <path>` 多次
- `ridl-tool` 负责：
  - 解析所有 RIDL 文件
  - 合并 AST（functions/singletons/interfaces/classes）为一个“聚合 module”
  - 使用现有模板渲染 `mquickjs_ridl_register.h`
- `ridl-builder` 负责：
  - 生成 manifest（模块选择快照）
  - 调用 ridl-tool 生成聚合 register.h 到稳定目录
  - 调用 mquickjs-build 消费该 header

### 生成入口形式（推荐）
新增 ridl-tool 子命令：
- `ridl-tool aggregate --manifest <path> --out <dir>`
输出：
- `<out>/mquickjs_ridl_register.h`（模板生成）
（后续可扩展：class defs header、其他 C/Rust 聚合产物）

理由：
- ridl-builder 不依赖 ridl-tool 内部 Rust API（避免耦合）
- 输出契约明确、便于调试

### 宏名迁移（用户选择 1）
- 以 `mquickjs_ridl_register_h.rs.j2` 当前宏体系为准。
- 需要同步检查与修改消费方：
  - `mquickjs-build` 是否 include 的 header guard/宏名有假设
  - `deps/mquickjs` 的 stdlib 模板是否依赖特定宏名（目前使用 `JS_RIDL_EXTENSIONS`，模板里也提供）
  - `mquickjs-build` 在 `-DMQUICKJS_RIDL_DECLARE_ONLY` 的隔离机制是否仍成立：
    - 若模板使用不同宏名，需要在 ridl-tool 模板中保留同等语义的 declare-only 开关，或同步调整 mquickjs-build。

## 实施步骤（拆分执行）
1. 盘点 ridl-builder 中所有“拼字符串生成产物”，确认是否有 ridl-tool 模板可替代。
2. ridl-tool：实现 `aggregate` 子命令（或等价 API），从 manifest 读取 RIDL 文件列表，聚合后用模板生成 register.h。
3. ridl-builder：
   - 保留 manifest 生成（可选：改为 serde_json）
   - 删除 `write_ridl_register_h` 拼接实现，改为调用 ridl-tool aggregate 输出到 `target/ridl/aggregate/`。
4. 消费方适配：
   - 调整 mquickjs-build / 其他 include 点，兼容模板宏名与 declare-only 语义。
5. 测试：
   - ridl-tool：新增单测/集成测试，断言聚合生成的 register.h 含 `JS_CGETSET_DEF("enabled"`。
   - 全链路：`cargo run -p ridl-builder -- prepare --profile framework`、`cargo test --test js_smoke`、`cargo run -- tests`。

## 风险与回滚策略
- 风险：宏名迁移导致 mquickjs-build 或 C 模板 include 失配。
- 缓解：
  - 优先保证 `JS_RIDL_EXTENSIONS` 这个 hook 宏仍存在且语义一致。
  - declare-only 机制需要在模板侧显式保留，否则会再次出现 C 编译期找不到声明/重复定义问题。

## 验收标准
- ridl-builder 不再拼接生成 `mquickjs_ridl_register.h`。
- 聚合 register.h 由 ridl-tool 模板生成。
- `console.enabled` 为 boolean，`default_id_any` 可用。
- `cargo test --test js_smoke` 通过。
- `cargo run -- tests` 通过。
