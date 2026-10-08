# SDK 正名与工作区结构修正计划

> 状态：✅ 已完成（2026-10-08 实施并全量验证通过；实施中额外修复 4 处
> ridl-builder CWD 假设与 ridl-tool 空聚合产物缺失，见知识库
> decision_sdk_repositioning）  
> 日期：2026-09-15（首版）/ 2026-10-08（修订）  
> 结论来源：项目现状评估 + 子 agent 独立复核报告

## 背景与定位结论

项目已不是 demo。代码分布证明：

| 组件 | 代码量 | 角色 |
|------|--------|------|
| `deps/ridl-tool` | ~8900 行 | 产品：RIDL 解析器 + 三端代码生成器 |
| `deps/mquickjs-rs` | ~5300 行 | 产品：安全绑定库（Context/GC/异步桥/no_std） |
| 根包 `mquickjs-demo` | 仅 270 行 | 示例应用 + JS 测试 runner |

最新定位：**mquickjs-rs —— 面向嵌入式场景的 Rust JS 引擎 SDK**（运行时库 + IDL 工具链 + 一致性测试套件）。

名实不符的现状：仓库名 `mquickjs-rs-demo`、根包名 `mquickjs-demo`，但内容是 SDK。
本计划修正工作区结构使其与新定位一致。

## 目标结构

```
（前）                          （后）
mquickjs-rs-demo/               mquickjs-rs-demo/
├── Cargo.toml  [package]       ├── Cargo.toml  [workspace] 虚拟清单 + resolver="2"
├── build.rs                    ├── justfile                    （命令加 -p）
├── mquickjs.build.toml         ├── mquickjs.build.toml        （profile 指向改写）
├── mquickjs.ridl.toml          ├── apps/
├── src/                        │   ├── demo/                  （原根包整体迁入，包名不变）
├── tests/                      │   │   ├── Cargo.toml / build.rs
│   ├── *.rs  ← 根包集成测试    │   │   ├── mquickjs.ridl.toml
│   ├── global/... (RIDL 模块) │   │   └── src/ + tests/
│   ├── module/...              │   └── test_app/              （已存在）
│   └── _diag/                  ├── tests/                     （仅 RIDL 测试模块与 JS 语料）
└── ...                         └── ...（deps/、ridl-builder/ 等不变）
```

**关键决策：根包改虚拟清单，原根包整体迁入 `apps/demo/`，包名保持 `mquickjs-demo` 不变。**

理由：
- 包名不变 → `mquickjs_demo::` 库引用（7 处）、`app_id = "mquickjs_demo"` →
  `target/ridl/apps/mquickjs_demo` 聚合路径、ROM class id 命名全部零改动；
  改名将连锁以上所有面，中期收益为零。
- 包整体迁移（src + tests/*.rs + build.rs + mquickjs.ridl.toml 同行）→
  包内部相对引用零改动。
- `deps/` 不动（见 P3）。

## P1：结构迁移（单一可编译节点）

> 实施纪律：P1.1–P1.5 全部落地并通过编译/测试后才算一个完整节点。
> 不允许"文件已移走但引用未修"的中间 commit 存在。

### 1.1 文件迁移

```bash
mkdir -p apps/demo/src apps/demo/tests
git mv src/*.rs apps/demo/src/
git mv build.rs apps/demo/build.rs
git mv mquickjs.ridl.toml apps/demo/mquickjs.ridl.toml
git mv tests/async_cancellation.rs apps/demo/tests/
git mv tests/async_stream_integration.rs apps/demo/tests/
git mv tests/base_build_no_ridl_headers.rs apps/demo/tests/
git mv tests/gc_root_cycle.rs apps/demo/tests/
git mv tests/gc_traced.rs apps/demo/tests/
git mv tests/js_smoke.rs apps/demo/tests/
git mv tests/no_c_option.rs apps/demo/tests/
```

不迁移：`tests/global/`、`tests/module/`、`tests/_diag/`、`tests/README.md`、
`tests/TODO.md`（测试语料/文档，不是根包集成测试）。

### 1.2 根 Cargo.toml 虚拟化

- 删除 `[package]`、`[lib]`、`[[bin]]` 段与原依赖。
- `[workspace]` 增补 `apps/demo`，**删除死条目 `"."`**，**显式声明 `resolver = "2"`**
  （虚拟清单默认回退 resolver=1，特性统一行为会变）。
- 新建 `apps/demo/Cargo.toml`：继承原 `[package]`（name/version/edition 不变），
  **21 条路径依赖全部改写**：
  - `deps/*` ×3 → `../../deps/*`
  - `ridl-modules/stdlib` → `../../ridl-modules/stdlib`
  - `tests/global/**` ×16 → `../../tests/global/**`
  - `apps/test_app` → `../test_app`

### 1.3 引用修正清单（grep 驱动，逐项落实）

**测试文件 CWD 修复（迁移后集成测试进程 CWD = apps/demo/，仓库根资源需重新解析）：**

| 文件 | 修正 |
|------|------|
| `apps/demo/tests/js_smoke.rs` | `CARGO_MANIFEST_DIR/tests` → 经 ancestors 上溯解析仓库根，收集 `tests/` 与 `ridl-modules/` |
| `apps/demo/tests/no_c_option.rs` | `target/mquickjs-build/...` 探测与 fallback 的 `deps/mquickjs/...` 均改为仓库根解析 |
| `apps/demo/tests/base_build_no_ridl_headers.rs` | `target/<profile>/mquickjs-build` 与 `deps/mquickjs` 同上 |

仓库根解析实现：从 `CARGO_MANIFEST_DIR` 沿 ancestors 上溯，以
`justfile` + `mquickjs.build.toml` 共存（或含 `[workspace]` 的 Cargo.toml）为锚点；
保持各文件既有的非空/存在性护栏断言。

**配置文件：**

| 文件 | 修正 |
|------|------|
| `mquickjs.build.toml` | 三个 profile `app_manifest = "Cargo.toml"` → `"apps/demo/Cargo.toml"`。
  注：唯一活跃消费方 `deps/mquickjs-sys/build.rs` 解析后弃用其值（仅用 profile 名拼产物目录），
  此改动本身不破坏构建链路；改写是为 P1.4 的默认发现规则提供真实 SoT |

**命令面（`cargo run -- tests` 在多成员 workspace 下报 "could not determine which binary"）：**

| 文件 | 修正 |
|------|------|
| `justfile`（根） | `test-js`/`run`/`aggregate` 加 `-p mquickjs-demo`；`build` 配方依赖 P1.4 新规则 |
| `apps/test_app/justfile` | prepare 配方按 P1.4 新规则核对（`cd ../.. && cargo run -p ridl-builder -- prepare`
  在新规则下默认解析到 default profile 的 app，行为需重验证；若 test_app 需自身为默认，改用显式 `--cargo-toml` 绝对路径） |
| `templates/app/justfile` | 与 test_app 同步，否则新脚手架落地即坏 |
| `AGENTS.md` | `cargo run -- tests` → `cargo run -p mquickjs-demo -- tests`（从仓库根运行） |
| `README.md` / `QUICKSTART.md` | 命令（QUICKSTART 9 处，含 L223 迁移前已坏的
  `cargo run -- tests/apps/my-app` 路径一并修正）与目录结构图更新 |
| `tests/README.md` | L5 命令与 7 个 .rs 迁出后的目录说明 |
| `docs/knowledge/architecture_base_vs_ridl_variant_selection.md`、
  `docs/knowledge/RULES-KNOWLEDGE-MEMORY.md` | 活跃知识库，命令同步更新 |
| `.claude/hooks/` | 已核实无根 `src/` 引用，无需改（no-op 项保留记录） |

**其他：**

| 项 | 处理 |
|----|------|
| `ridl-builder/src/config.rs` | 死文件（main.rs 未声明 `mod config;`）——删除，在 commit message 说明 |
| 历史文档 `docs/planning/`、`docs/legacy/` | 按惯例不改写历史，仅活跃文档更新 |

### 1.4 ridl-builder 应用发现重设计（复核后修订版）

原方案的"向下扫描 apps/*"作废：`apps/test_app` 同样含 `mquickjs.ridl.toml`，
会触发歧义错误，与验证步骤自相矛盾。

**新规则（优先级从高到低）：**
1. 显式 `--cargo-toml`（保持现有绝对路径要求不变）；
2. 从 CWD **向上**查找最近 `mquickjs.ridl.toml`（多应用原约定，apps/*/ 内运行各归各）；
3. 向上未命中时，继续**向上**定位 workspace 级 `mquickjs.build.toml`，
   取 `default` profile 的 `app_manifest`（相对该文件目录解析）作为默认应用。

理由：`mquickjs.build.toml` 头注释自称 "SoT for RIDL module selection"，
但当前 app_manifest 值无任何活跃消费方（mquickjs-sys 解析后弃用；config.rs 是死文件）。
此改动使其成为真实 SoT，天然消除多应用歧义，无硬编码，符合 AGENTS.md 通用机制要求。

同步项：`ridl-builder` 帮助文本更新；
`docs/knowledge/reference_ridl_builder_prepare_command.md` 更新。

## P2：文档与知识沉淀

1. `README.md`：定位段重写为 SDK 三组成（运行时库 / 工具链 / 测试套件），
   架构图补 `apps/` 层，目录说明含 deps/ 命名约定。
2. `tests/TODO.md`：条目已过时（P0 项均已修复，js_fields/literals/types 现已全绿），
   归档为历史记录（文首加"已过时"声明），避免误导。
3. 知识库新增：
   - `decision_sdk_repositioning.md`（决策：demo → SDK 正名，虚拟清单理由）
   - `architecture_workspace_layout.md`（架构：目录职责约定，含 deps/ 命名约定说明）

## P3：明确不做项（含理由）

| 项 | 理由 |
|----|------|
| `deps/` → `crates/` 改名 | 波及全部 workspace 路径、path 依赖、build.rs、hooks、文档；
  零功能收益。以 README/知识条目说明约定替代。 |
| runner 从 demo 拆出独立 crate | runner 本质是"内嵌引擎跑 JS 的应用"（main.rs 持有
  ridl_bootstrap 与 Context），与 demo 天然同体；拆出属过度设计。 |
| 仓库目录/远端改名 `mquickjs-rs-demo` → `mquickjs-rs` | 需 GitHub 侧操作，列为可选手动步骤，
  不在本次实施范围。 |

## 验证清单（全部通过才算完成）

1. `cargo build`（workspace 全量编译，无 resolver 警告）
2. `cargo test -p mquickjs-demo`：7 个集成目标 **32 个测试函数**全过
   （10+6+1+6+1+7+1；`no_c_option`/`base_build_no_ridl_headers` 是 CWD 修复的直接验证面）
3. `cargo test -p ridl-tool`（357）与 `cargo test -p mquickjs-rs`（153）：无回归
4. `cargo run -p mquickjs-demo -- tests`（从仓库根运行）：26/26 JS 用例通过
5. `cargo run -p ridl-builder -- prepare`（从仓库根运行，无参数）：经 P1.4 新规则
   默认解析到 apps/demo
6. `cargo build -p test_app` 与 `cd apps/test_app && cargo run -p ridl-builder -- prepare`：
   多应用路径不受影响
7. `just build`、`just test-js`（若存在对应配方）：命令面可用
8. `templates/app` 脚手架文件与 test_app 一致性核对
9. grep 复查：justfile、apps/*/justfile、templates/、docs/knowledge/、tests/README.md、
   顶层 `*.md` 无残留 `cargo run -- tests` 与旧根包路径引用
   （历史 planning/legacy 文档除外）

## 风险与回滚

- 风险 1：ridl-builder 新发现规则与 mquickjs.build.toml 的 app_manifest 不一致
  → prepare 默认应用选错（注意：prepare 不直接读 build.toml，断裂点是"发现规则"
  与"SoT 值"脱节）。缓解：验证步骤 5/6 覆盖两种运行位置。
- 风险 2：集成测试 CWD 修复不彻底 → 验证步骤 2 必败（护栏断言会立刻暴露）。
- 风险 3：resolver 未显式声明 → 特性统一行为漂移。缓解：P1.2 显式 `resolver = "2"`，
  验证步骤 1 检查无警告。
- 整体回滚：git revert 单节点 commit。

## 实施顺序

P1.1 + P1.2 + P1.3 + P1.4 → 验证清单 1–9 → 单一可编译 commit
（含 build.toml/justfile/测试路径/ridl-builder 全部修正）→ P2 文档与知识
（独立 commit）→ 复查。commit 前按仓库规则征询用户。
