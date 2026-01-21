# RIDL 构建流水线闭环方案：Driver（ridl-builder） + Glue（mquickjs-ridl-glue）

日期：2026-01-14

状态：可执行方案（待实现）

> 目标：在不违反“build.rs 不运行/不构建任何工具”的硬约束下，实现 RIDL 产物的可复现选择（SoT）+ 可追溯审计 + 多 root crate 支持，并提供半自动（当前）与全自动（未来）两种工作流。

---

## 1. 背景与问题

当前工程需要在编译期完成 QuickJS C API 注册（运行期无法动态注册）。因此必须在构建阶段生成并链接/包含一套稳定的聚合产物（register.h + 若干 rs 侧聚合文件），同时保证：

- 模块选择不可硬编码白名单；新增模块仅放入模块目录即可被纳入（在 root 的 direct deps 中）。
- build.rs 不得构建/运行任何 Rust 工具（避免 cargo 锁冲突、ETXTBSY、rust-analyzer 并发等问题）。
- 支持 workspace 下多 root crate，且每个 root 的聚合产物相互隔离。

过去通过 `--intent build|test`（静态规则）选择 `[dependencies]` 或 `+ [dev-dependencies]`，存在两类问题：

1) intent 需要人工指定，容易与实际执行的 `cargo build/test` 不一致。
2) Cargo 的真实编译计划受 features/target/profile/cfg 影响，静态规则可能偏离“本次构建真正会编译哪些依赖”。

---

## 2. 设计原则（硬约束）

1) **SoT=direct deps**：RIDL 模块候选集仅来自 root crate 的 **direct dependencies**（不递归闭包）。
2) **RIDL module 判定**：仅当某依赖 crate 的 `src/` 下存在至少一个 `*.ridl` 文件，才认为它是 RIDL module。
3) **禁止 build.rs 运行工具**：build.rs 只能做 copy / include / 校验；模块选择、聚合生成、工具构建必须由显式命令完成。
4) **避免硬编码**：不得通过模块白名单或 singleton 名称硬编码实现聚合/注册；新增模块只需加入 root 的 direct deps 并放入 `src/*.ridl`。
5) **多 root crate**：root 选择必须稳定，不可通过 package name 猜测；必须通过 manifest_path 精确匹配。

---

## 3. 角色分层：Driver vs Glue

### 3.1 Driver（ridl-builder）：Producer + Orchestrator

Driver 的职责是“**串行地**生产所有可被编译消费的产物”，并将“会引发 cargo 冲突的动作”集中到 build 之外运行：

- 读取 `--cargo-toml` 精确定位 root package。
- 使用 Cargo nightly `-Z unstable-options --unit-graph` 获取本次构建的真实 unit graph。
- 解析 unit graph，推导 root 的 direct deps（SoT）。
- 扫描 direct deps 中的 `src/*.ridl`，形成 RIDL module 集合。
- 调用 ridl-tool 生成聚合产物（register.h、symbols、ctx_ext、context_init 等）。
- 生成稳定的依赖摘要（工程 SoT）与 raw unit-graph JSON（审计）。
- （可选）调用 mquickjs-build 生成 QuickJS 头/库。

Driver **不**运行在 build.rs 里；它通过显式命令执行，从而避开 cargo build 锁冲突。

### 3.2 Glue（mquickjs-ridl-glue）：Build-time Consumer

Glue 的职责是“在 `cargo build/test` 编译过程中消费既有聚合产物”，只做：

- 定位 `<target_dir>/ridl/apps/<app-id>/aggregate/`。
- 校验：产物是否存在、schema_version 是否匹配、是否匹配当前 root（例如 manifest 中记录的 root cargo_toml）。
- copy 需要的 `*.rs` 到 OUT_DIR，并使 root crate 可 `include!` 使用。

Glue **不得**：构建/运行 ridl-builder、ridl-tool、mquickjs-build，或运行任何 cargo 子进程。

---

## 4. SoT 推导：基于 unit-graph（nightly）

### 4.1 为什么 unit-graph 能替代 intent

- `cargo build` 的 unit graph 不会将 dev-deps 纳入 root entry units 的 direct deps。
- `cargo test --no-run` 的 unit graph 会将 dev-deps 纳入 test units 的 direct deps。

因此只要让用户选择要模拟的 cargo 子命令（build/test），就能不依赖人工 intent。

### 4.2 entry unit 选择规则（当前实现）

- subcommand=build：root pkg 的 target kind 包含 `lib` 或 `bin` 视为 entry units。
- subcommand=test：root pkg 的 target kind 包含 `test` 视为 entry units。

对 entry units，提取其 1-hop dependencies 对应的 pkg_id，映射回 cargo metadata 的 packages。

> 注意：SoT 仍然是 direct deps（一跳），不是 transitive closure。

---

## 5. 产物与目录布局

### 5.1 输出目录

默认输出到：

- `<target_dir>/ridl/apps/<app-id>/aggregate/`

其中：

- `target_dir` 默认为 `cargo metadata.target_directory`，可由 `MQUICKJS_RIDL_TARGET_DIR` 覆盖。
- `app-id` 默认为 root package.name 归一化（所有非 `[A-Za-z0-9_]` 字符替换为 `_`），也可由 CLI/env 覆盖。

### 5.2 聚合产物（已有）

- `ridl-manifest.json`
- `mquickjs_ridl_register.h`
- `ridl_symbols.rs`
- `ridl_context_ext.rs`
- `ridl_bootstrap.rs`

### 5.3 新增“依赖图导出”产物（调试/审计）

当使用 `aggregate/prepare --cargo-subcommand ...`（unit-graph 路径）时，driver 会自动在 out_dir 写出：

- raw 审计：`ridl-unit-graph.json`
  - 内容即 `cargo ... --unit-graph` stdout JSON
  - 用途：排查“为何本次构建包含/不包含某依赖”
  - 不作为稳定接口

- 稳定摘要（工程 SoT）：`ridl-deps.json`
  - schema_version + root 信息 + cargo_subcommand + cargo_args + direct_deps
  - 用途：
    - 作为本次聚合所依据的依赖集合快照（可审计/可 diff）
    - 作为 glue 校验的输入（可选）

---

## 6. CLI 设计（可执行）

### 6.1 半自动（当前阶段）

- 构建工具（二进制）：

```bash
cargo run -p ridl-builder -- build-tools
```

- 生成聚合产物（推荐使用 unit-graph）：

```bash
cargo run -p ridl-builder -- aggregate \
  --cargo-toml /abs/path/to/root/Cargo.toml \
  --cargo-subcommand build \
  --cargo-args "--features foo"
```

- 生成 QuickJS（可选，若你需要带上 register.h 生成头/库）：

```bash
cargo run -p ridl-builder -- prepare \
  --cargo-toml /abs/path/to/root/Cargo.toml \
  --cargo-subcommand build \
  --cargo-args "--features foo"
```

- 最终构建 root crate：

```bash
cargo build -p <root-crate>
# 或
cargo test -p <root-crate>
```

### 6.2 全自动（未来：单入口串行 driver）

新增一个单入口命令（命名待实现）：

```bash
cargo run -p ridl-builder -- auto \
  --cargo-toml /abs/path/to/root/Cargo.toml \
  --cargo-subcommand build \
  --cargo-args "--features foo"
```

它串行执行：build-tools → export-unit-graph → export-deps → aggregate → mquickjs-build（可选）

> 关键：所有会引发 cargo 冲突的动作都集中在这个进程中串行发生；build.rs 只消费，不生成。

### 6.3 依赖图导出（调试/审计功能）

> 说明：本功能用于调试/审计（回答“为何本次构建包含/不包含某依赖”）。
> 主流程并不要求手动执行 export-*：当使用 `aggregate/prepare --cargo-subcommand ...` 时，driver 会自动把快照写入 out_dir。

- raw unit graph（cargo 输出，不稳定，仅审计）：

```bash
cargo run -p ridl-builder -- export-unit-graph \
  --cargo-toml /abs/path/to/root/Cargo.toml \
  --cargo-subcommand build \
  --cargo-args "--features foo" \
  --out /abs/path/to/ridl-unit-graph.json
```

- 稳定 deps 摘要（工程 SoT 快照，可 diff）：

```bash
cargo run -p ridl-builder -- export-deps \
  --cargo-toml /abs/path/to/root/Cargo.toml \
  --cargo-subcommand build \
  --cargo-args "--features foo" \
  --out /abs/path/to/ridl-deps.json
```

### 6.4 失败策略（你已确认）

当用户显式指定 `--cargo-subcommand`：

- 若 `cargo ... -Z unstable-options --unit-graph` 失败：**直接报错**，提示：
  - 需要 nightly toolchain（例如 `cargo +nightly ...`）
  - 或者改用 legacy 路径：`aggregate --intent build|test`（不走 unit-graph）

不允许静默自动回退，避免隐藏问题。

---

## 7. 如何新增一个 RIDL module（开发者工作流）

### 7.1 创建模块 crate

约定：一个 crate 成为 RIDL module 的条件是其 `src/` 下存在 `*.ridl`。

最小步骤：

1) 新建 crate（例如放到 `ridl-modules/<name>`，或其它本地路径）。
2) 在该 crate 的 `src/` 放入至少一个 `*.ridl` 文件（例如 `src/foo.ridl`）。
3) 在该 crate 中提供 `initialize_module()`（用于集中拉入/确保符号不被裁剪）。

### 7.2 让模块生效（SoT=direct deps）

在 **root crate 的 `[dependencies]`** 中添加该模块 crate（direct dep）。

注意：

- 放在 `[build-dependencies]` 不会被纳入（SoT 只看 normal/dev，且 build 模式不看 dev）。
- 不递归：模块的依赖模块不会被自动纳入；只有 root 的 direct deps 才是候选。

---

## 8. Root crate 如何完成 RIDL 集成

### 8.1 Cargo.toml 依赖

root crate 需要依赖 glue（在 build-dependencies 或 dependencies，取决于现有组织；但其 build.rs 只 copy）：

- `mquickjs-ridl-glue`

同时，root crate 在 `[dependencies]` 中声明需要的 RIDL module crates（SoT）。

### 8.2 构建前置：准备聚合产物

在执行 `cargo build/test` 前，先运行 ridl-builder：

- 半自动：`build-tools` + `aggregate`（或 `prepare`）
- 全自动：`auto`

然后 `cargo build/test` 才会成功，因为 glue build.rs 只消费聚合产物。

### 8.3 多 root crate 的路由

- ridl-builder 通过 `--cargo-toml` 精确选择 root。
- 输出用 `app-id` 隔离（默认由 root 包名归一化得到）。
- glue 需要知道它对应哪个 root（建议通过 `mquickjs.ridl.toml` nearest-wins 或 CI/env 覆盖；具体实现见相关规划文档）。

---

## 9. 多 root crate 支持

### 9.1 root 选择

- 使用 `--cargo-toml /abs/path/to/root/Cargo.toml`。
- ridl-builder 内部使用 `cargo metadata` 的 `package.manifest_path` 精确匹配，避免 workspace 同名包歧义。

### 9.2 输出隔离

- 产物路径包含 `<app-id>`，不同 root 互不干扰。

### 9.3 并发与锁（全自动阶段建议）

为避免同一 app-id 的并发写入：

- driver 在 `<out_dir>` 上加锁（文件锁/lockfile）。
- glue 不加锁，只做读取与校验。

---

## 10. 如何构建整个项目（推荐）

### 10.1 本仓库（单 root）

```bash
# 1) 生成聚合产物（推荐 unit-graph 路径；需要 nightly）
cargo run -p ridl-builder -- prepare \
  --cargo-toml /abs/path/to/root/Cargo.toml \
  --cargo-subcommand build

# 2) 构建/测试
cargo build
cargo test

# 3) JS 集成用例
cargo run -- tests
```

### 10.2 多 root（workspace 多 app）

分别对每个 root 执行：

```bash
cargo run -p ridl-builder -- prepare --cargo-toml /abs/path/to/A/Cargo.toml --cargo-subcommand build
cargo build -p A

cargo run -p ridl-builder -- prepare --cargo-toml /abs/path/to/B/Cargo.toml --cargo-subcommand build
cargo build -p B
```

---

## 11. 测试矩阵（待实现）

1) 单元测试：
- unit-graph JSON fixture：验证 root 匹配、entry unit 选择、1-hop deps 提取。
- RIDL module 判定：`src/*.ridl` 存在/不存在。

2) 集成测试：
- 在本仓库对一个 demo/root crate 跑：
  - `aggregate --cargo-subcommand build`
  - `aggregate --cargo-subcommand test`
  - `export-unit-graph` 与 `export-deps`
  - 校验输出目录与文件存在

3) 端到端：
- `cargo test`
- `cargo run -- tests`

---

## 12. 实施步骤（拆分）

- Step 1：实现 export-unit-graph / export-deps（并在 aggregate/prepare 中复用同一套解析提取逻辑）。
- Step 2：补齐 unit-graph fixture 测试。
- Step 3：实现（或完善）driver `auto` 单入口与输出目录锁。
- Step 4：glue build.rs 增强校验与友好错误提示（缺失产物时指引运行 driver）。

---

## 13. 风险与规避

- nightly unit-graph 不稳定：
  - raw JSON 仅作审计；工程 SoT 以 `ridl-deps.json` 的稳定 schema 为准。
  - 若用户不能使用 nightly，可使用 `--intent` legacy 路径（但可能与真实构建计划不一致）。

- cargo 并发冲突：
  - 禁止 build.rs 运行工具。
  - driver 串行化执行；同 out_dir 加锁。

