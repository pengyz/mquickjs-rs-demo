---
name: decision-sdk-repositioning
description: 项目定位从 demo 正名为嵌入式 JS SDK，工作区根改虚拟清单、应用归位 apps/
type: decision
created: 2026-10-08
sources: [docs/planning/2026/2026-09-15-sdk-repositioning.md]
---

**决策**：项目定位从"demo"正名为 **mquickjs-rs 嵌入式 JS 引擎 SDK**（运行时库 +
IDL 工具链 + 一致性测试套件）。工作区根 `Cargo.toml` 改为**虚拟清单**
（`resolver = "2"`，无 `[package]`），原根包整体迁入 `apps/demo/`，包名保持
`mquickjs-demo` 不变。

**为什么包名不变**：改名会连锁 `app_id`（mquickjs.ridl.toml）→
`target/ridl/apps/<app_id>` 聚合路径 → ROM class id 命名 → 7 处
`mquickjs_demo::` 库引用，中期收益为零。

**配套变更**：
- ridl-builder 应用发现改为两级：向上 `mquickjs.ridl.toml` 优先，兜底
  `mquickjs.build.toml` 的 `default` profile `app_manifest`（后者原是死配置，
  现成为真实 SoT）
- 删除死文件 `ridl-builder/src/config.rs`；修复 `write_cargo_env_config`/
  `build_mquickjs`/`prepare`/`selftest` 四处 CWD 假设（改经
  `find_workspace_root()` 解析）
- ridl-tool 空聚合（零 RIDL 模块应用）补齐 register.c / class_ids.h / api.h
  空产物生成（此前 "missing AggregateIR" 直接报错）

**明确不做**：`deps/` 改名 `crates/`（波及面大零功能收益，以 README 约定说明替代）；
runner 从 demo 拆出（runner 内嵌引擎，与 demo 天然同体）；仓库目录/远端改名
（GitHub 侧手动步骤）。

相关：[[workspace-layout]]
