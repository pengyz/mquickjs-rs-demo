---
name: decision-abandon-ui-app-framework
description: 决定不上马 slint UI 层与"类快应用应用框架"方向，附四路对抗性复核结论
type: decision
created: 2026-09-04
sources: [docs/knowledge/gotcha_mquickjs_gc_compaction_and_finalizer.md]
---

# 决策：放弃 UI 层与"类快应用应用框架"方向

## 结论

**不上马** slint UI 渲染层、「类快应用应用框架」。相关产物已删除：

- `deps/mquickjs-ui/`（UI 渲染层 stub）
- `tests/global/ui/test_ui/`（UI RIDL 模块与测试）
- `docs/planning/embedded-app-framework.md`（四层架构规划）

## 决策依据（四路独立对抗性复核）

复核分别从**目标定位 / 技术架构 / 执行优先级 / 生态竞争位**四个角度独立开展，
四路结论收敛，无一路支持该方向。

### 1. 目标定位不成立

| 问题 | 证据 |
|---|---|
| 热更新与编译期注册根本矛盾 | `docs/build/pipeline.md`：注册必须编译期；而原规划文档对比表写"热更新：**支持**"，无任何解释。任何新 native 接口都要重跑 `ridl-builder prepare` + 重编固件 |
| 目标场景官方运行时已在位 | `open-vela/frameworks_runtimes_quickapp` —— OpenVela 上游自带的 QuickApp runtime |
| 规划文档示例在自己引擎上跑不了 | 原 `embedded-app-framework.md` 的 Layer 3 示例用了 `class` / 箭头函数 / 模板字符串；mquickjs 为 ES5 子集（`mquickjs.c` 中 `TOK_ARROW` 出现 0 次） |
| 无真实需求方 | 列了 4 个"适用场景"，无一有具名团队 |

### 2. 技术接缝不成立

- **Layer 1 选 slint 错误**（三重）：
  - 授权：slint 为 GPLv3 / 桌面免版税 / **嵌入式商业付费**，设备厂商需按出货量付费
  - 技术：slint 是声明式 + 编译期代码生成 + 自带绑定系统，**没有 DOM 式运行时建树 API**；
    用 `createElement/appendChild` 驱动它等于维护三棵树，slint 核心价值全程闲置
  - 事实：`Cargo.toml` 中 `slint = "1.5"` 当时被注释掉，**从未编译过一次**
- **缺事件循环**：现有异步是宿主手动泵取（`drain_completions`），非事件驱动。
  UI 的本质是事件驱动并发（点击/触摸/动画帧/定时器）——这是完整的调度器工程，规划中未提及。
- **引擎档位与框架档位矛盾**：mquickjs 定位 ~10kB RAM 的脚本引擎；
  带 UI 的应用框架需要渲染引擎 + framebuffer，两者在硬件光谱两端。

### 3. 执行层：属过早抽象

`mquickjs-ui` 是在渲染引擎未定、接口映射未验证、事件模型缺失的前提下，先固化了三样
最不可能存活的东西（DOM 数据结构、DOM 接口词汇、`println!` 假渲染），
然后给自己发了"Layer 1 完成"的勋章。

### 4. 生态位

作为"嵌入式应用框架"无活路：目标场景官方运行时已在位；ES5 + 无 npm 使
"用 JS 写 UI"的价值主张不成立；单人维护对抗的是华为 ArkUI 与 LVGL（MIT）的成熟社区。
"类快应用"路线的历史成绩单（Firefox OS / Tizen / webOS / KaiOS）不乐观。

## 保留的资产

以下为复核认定的**真实技术资产**，不受本决策影响：

- `ridl-tool`：RIDL 解析 / 校验 / 编译期代码生成
- `mquickjs-rs`：FFI 绑定、构建编排、`Root`/`Traced` GC 集成（**已修复压缩安全缺陷**）

## 若要重新考虑此方向，前置条件

项目定位仍在讨论中。若将来重新考虑 UI/应用框架，至少须先满足：

1. 目标硬件与目标团队具名（谁写应用、跑在什么设备上）
2. 明确回答"编译期注册下应用如何分发"
3. 渲染方案通过可失败的一日 spike（含授权评估）
4. 事件循环/调度器设计先行，而非事后补

## 相关

- 技术缺陷修复见 `gotcha_mquickjs_gc_compaction_and_finalizer.md`