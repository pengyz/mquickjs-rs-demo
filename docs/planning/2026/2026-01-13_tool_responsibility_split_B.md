<!-- planning-meta
status: 未复核
tags: build, context-init, engine, ridl
replaced_by:
- docs/ridl/overview.md
- docs/ridl/context-init.md
- docs/build/pipeline.md
-->

> 状态：**未复核**（`build` `context-init` `engine` `ridl`）
>
> 现行口径/替代：
> docs/ridl/overview.md
> docs/ridl/context-init.md
> docs/build/pipeline.md
>
> 关键结论：
> - （待补充：3~5 条）
# 计划：方案B - 明确 ridl-tool / mquickjs-build / ridl-builder 的职责边界（2026-01-13）

> 目标：把“RIDL 语言处理 / 工程聚合 / C 产物构建”三类职责拆开，避免当前链路在 B3（预构建）下出现的注入断链、符号保活失效与产物不一致问题。
>
> 关键约束：mquickjs **不支持运行时 C API 注入**；stdlib 扩展必须在 **编译期** 固化进 `js_stdlib` ROM 表。

## 0. 现状问题（为什么需要重划）

当前仓库中存在 3 个事实同时成立：

1) stdlib 注入是编译期：`mqjs_stdlib_template.c` 通过 `#include "mquickjs_ridl_register.h"` 展开 `JS_RIDL_DECLS/JS_RIDL_GLOBAL_PROPS`，由 `mqjs_ridl_stdlib` 生成 `mqjs_ridl_stdlib.h`（ROM 表）。
2) RIDL 选择集合来自 Rust 工程（SoT = App manifest / registry source），并且需要聚合产物（register.h、symbols keep-alive、ctx-ext slot 定义等）。
3) B3（预构建）将 mquickjs-build 脱离 build.rs，使得“生成聚合产物（OUT_DIR）”与“编译 C 产物（target/mquickjs-build/…）”可能不同步。

因此必须把“谁决定集合 / 谁产出聚合 / 谁消费聚合”定义成一个稳定的流水线。

## 1. 概念分层：三段式流水线

### 1.1 RIDL 语言层（Language layer）
**输入**：单个 RIDL 文件（或一个 crate 内的 RIDL 文件列表）
**输出**：模块级（module-local）代码生成物

- Rust glue（`<module>_glue.rs`）
- Rust API（`<module>_api.rs`）
- symbols（`<module>_symbols.rs`）
-（可选）class defs / C 侧片段（非聚合，仅模块自身可生成的片段）

> 该层不应理解 Cargo 依赖图，不负责“最终注册集合”。

**建议归属**：`ridl-tool`（模块模式，module-only）。

### 1.2 工程聚合层（Aggregation layer）
**输入**：registry source（App manifest）+ 依赖图解析结果 + 模块级产物（或 RIDL 列表）
**输出**：聚合级（aggregate）产物

- `mquickjs_ridl_register.h`（聚合头：`JS_RIDL_DECLS/JS_RIDL_GLOBAL_PROPS`）
- `ridl_bootstrap.rs`（Rust 侧集中入口：确保模块 symbols/glue 被拉入）
- `ridl_context_init.rs` / `ridl_ctx_ext.rs` / `ridl_slot_indices.rs`（ctx-ext 相关聚合）
- `ridl-manifest.json`（用于记录本次构建收集到的 RIDL modules 列表；作为可复现的集合快照，不要求下游工具读取它）

> 该层需要理解“最终模块集合”，因此必须理解 Cargo/workspace 的 registry source。

**建议归属**：`xtask`（或上层 app 的 build.rs；但为了 B3 稳定与避免嵌套 cargo，建议迁入 xtask）。

### 1.3 C 产物构建层（C build layer）
**输入**：mquickjs 源码目录 + 聚合头 `mquickjs_ridl_register.h` +（可选）class defs header + 编译选项
**输出**：C 静态库与可被 Rust 消费的构建输出描述

- `libmquickjs.a`
- `include/`（`mquickjs.h`、`mquickjs_atom.h`、`mqjs_ridl_stdlib.h`、`mqjs_ridl_class_id.h` 等）
- `mquickjs_build_output.json`（给 sys/rs crate 提供 link/include 路径）

**建议归属**：`mquickjs-build`。

> 该层不应解析 Cargo 依赖图；它只“消费”聚合层产物。

## 2. 职责重划：工具拆分与接口

### 2.1 ridl-tool（建议目标）
- ✅ parse/typecheck RIDL
- ✅ 生成模块级 Rust glue/api/symbols
- ❌ 不再负责 resolve Cargo deps（或仅保留为可选库能力，主路径不走）
- ❌ 不再负责聚合 `mquickjs_ridl_register.h`（主路径迁到 ridl-builder）

> 备注：保留 `resolve/generate` 命令可以作为兼容，但方案B主路径应让 ridl-builder 成为 SoT 编排者。

### 2.2 ridl-builder（方案B 的“权威编排入口”）
新增（或调整）命令，使其成为 B3 的稳定入口：

- `ridl-builder ridl-resolve`：
  - 读取 registry source（app manifest，来自 `mquickjs.build.toml` 的 profile 或 CLI 参数）
  - 解析依赖图（只取 path deps 且 `src/*.ridl` 非空）
  - 产出稳定目录下的 `ridl-manifest.json`

- `ridl-builder ridl-generate`：
  - 调用 ridl-tool 生成各模块产物到稳定目录（不落到 OUT_DIR）
  - 生成聚合产物（`mquickjs_ridl_register.h`、`ridl_bootstrap.rs`、ctx-ext 文件）

- `ridl-builder build-mquickjs`：
  - 调用 `mquickjs-build build --ridl-register-h <stable_ridl_dir>/mquickjs_ridl_register.h --out <stable_out_dir>`（mquickjs-build 不强依赖 manifest）

- `xtask prepare`（一键）：
  - build-tools
  - ridl-resolve + ridl-generate
  - build-mquickjs

### 2.3 mquickjs-build（保持纯 C build）
- ✅ 读取 `--plan`（或直接 `--ridl-register-h <path>`）以复制聚合头到 include 目录
- ✅ 生成 `mqjs_ridl_stdlib.h`（ROM 表）
- ✅ 编译并打包 `libmquickjs.a`
- ❌ 不解析 Cargo 依赖图

> 输入接口建议：长期可将 `--plan` 收敛为 `--ridl-register-h` + `--class-defs-h` 等更直接的输入，plan 仅由聚合层使用。

## 3. 产物路径与一致性（解决 B3 断链的关键）

方案B要求：聚合产物与 C build 产物都落到 **稳定路径**（例如 `target/mquickjs-build/<profile>/<target>/<mode>/...`），不依赖 Cargo 的 `OUT_DIR` hash 目录。

- xtask 产出：
  - `target/mquickjs-build/<profile>/ridl/plan.json`
  - `target/mquickjs-build/<profile>/ridl/mquickjs_ridl_register.h`
  - `target/mquickjs-build/<profile>/ridl/ridl_bootstrap.rs` 等

- mquickjs-build 产出：
  - `target/mquickjs-build/<profile>/<target>/<mode>/{include,lib,build,...}`
  - `mquickjs_build_output.json`

- mquickjs-sys/build.rs：
  - 仅验证 `mquickjs_build_output.json` 存在，并导出 include/lib dir 给 mquickjs-rs。

## 4. class-id / atoms 的权威来源（避免循环依赖）

### 4.1 atoms
- `mquickjs_atom.h` 是 `mqjs_ridl_stdlib` 的生成产物（stdlib ROM 表强相关）。
- 核心头 `mquickjs.h` 不依赖 atoms 头，这使得 core bindgen 可以与 stdlib 构建解耦。
- atoms 的权威来源：**mquickjs-build**（通过 `mqjs_ridl_stdlib -a` 导出）。

### 4.2 class-id（方案A：权威来自 mquickjs）
- mquickjs 不允许运行时动态扩展内置 class（无 `JS_NewClassID` 路径），因此 class-id 必须由编译期固化的 stdlib/class 定义决定。
- `mqjs_ridl_class_id.h` 已由 `mqjs_ridl_stdlib -c` 导出，内容来自 mquickjs 构建工具对 class 表的最终编号（`class_idx`）。
- 为避免 `ridl-builder` 复刻 mquickjs 内部编号规则、产生强绑定与维护负担：
  - class-id 的权威来源固定为：**mquickjs-build**（`mqjs_ridl_stdlib -c`）。
  - `ridl-builder` 不分配 class-id 数值；它只负责聚合输出（register.h / class defs 等）作为 mquickjs-build 的输入。
- Rust 侧 glue 如需 class-id：通过“前置流水线”先由 mquickjs-build 生成 `mqjs_ridl_class_id.h`，再由聚合层把该头转换为 Rust 常量模块并供各模块引用（不通过每个模块的 build.rs 读取 OUT_DIR）。

## 5. 符号保活（必须保持策略不变）

方案B不改变符号导入策略：
- 仍由“Rust 聚合产物（ridl_bootstrap.rs / aggregated_symbols.rs / ensure_symbols）”负责把 RIDL 模块的 glue/symbols 强制拉入最终链接闭包。
- C stdlib ROM 表只负责把扩展项以编译期方式固化到 `js_stdlib`。

关键是：聚合层产物必须与 C build 使用的 register.h 同源、同版本、同集合。

## 5. 迁移步骤（分阶段，保证可回归）

### Phase 1：定义稳定目录与 xtask 一键入口
1) 在 `mquickjs.build.toml` 定义 profile -> app manifest（现有已具备）
2) xtask 增加 `prepare`：
   - build-tools
   - ridl-resolve/generate 到稳定目录
   - build-mquickjs 使用同一 plan

### Phase 2：让 app build.rs 退化为“零聚合/或仅 include 稳定产物”
- app build.rs 不再运行 ridl-tool；改为 `include!` 稳定目录下的聚合 Rust 文件（或通过 env 指定路径）。

### Phase 3：ridl-tool 命令面收敛（可选）
- 保留 `module` 作为核心；`resolve/generate` 变为兼容路径或库 API。

## 6. 验收标准
- 执行：`cargo run -p xtask -- prepare` 后，`cargo run -- tests` 至少能看到 stdlib 中 `console` 与 demo 函数被注入（不再出现“is undefined”）。
- 新增 RIDL module：只需加到 app manifest deps（path）并放入 ridl 文件；无需改工具/代码。
- 全流程不在 build.rs 内执行 cargo 子进程；避免锁竞争与显著变慢。

## 7. 待确认问题
1) “聚合层”是否必须解析完整依赖图（cargo metadata）还是仅解析 `[dependencies]` 的 path deps（当前 ridl-tool 是后者）。
2) 聚合产物中 slot index 的稳定性要求：是否需要把 slot index 也作为 plan 的一部分固定输出。
3) class/proto 相关产物：mquickjs-build 输入应是 `register.h + class_defs.h` 还是 register.h 内嵌 class defs。
