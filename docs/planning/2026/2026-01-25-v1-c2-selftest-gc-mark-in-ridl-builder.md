# 规划：将 `selftest_gc_mark.c` 接入 `ridl-builder` 的自动化回归（方案C）

日期：2026-01-25

## 背景

目前引擎级 `gc_mark` 的最小回归用例位于 `deps/mquickjs/selftest_gc_mark.c`。

现状：

- 已通过手工 `cc` 编译并运行验证（exit 0）。
- 但未接入仓库统一的自动化回归入口，导致：
  - 回归依赖人工步骤，容易漏跑；
  - CI 若只跑 `cargo test` / `cargo run -- tests`，无法覆盖到这 3 条引擎级断言。

仓库约定：

- “完成特性后，除 `cargo test` 外还必须跑 JS 集成：`cargo run -- tests`”。
- 希望将引擎级自检也纳入同一套自动化入口。

## 目标

将 `deps/mquickjs/selftest_gc_mark.c` 的构建与执行接入到 `ridl-builder`，使其可通过稳定命令自动运行：

- 本地与 CI 都可以重复执行；
- 失败时有清晰的 stdout/stderr 输出；
- 不引入硬编码的相对路径 `rerun-if-changed`；
- 不破坏现有 `cargo run -- tests` 流程。

## 方案选择（C）

选择方案 C：在 `ridl-builder` 新增子命令 `selftest`，统一构建并运行引擎级自检。

形式：

- `cargo run -p ridl-builder -- selftest-gc-mark`

并在仓库主回归入口 `cargo run -- tests` 的 CI 脚本/README 中补充建议：

- `cargo run -p ridl-builder -- selftest-gc-mark`

> 说明：主二进制 `mquickjs-demo` 当前的 `tests` runner 是 JS 集成用例驱动，
> 而引擎级自检更适合挂在构建工具侧（`ridl-builder`），避免污染运行时入口。

## 设计细节

### 1. 构建方式

在 `ridl-builder` 中通过 `std::process::Command` 调用宿主编译器（默认 `cc`），构建一个临时二进制：

- 输入：`deps/mquickjs/selftest_gc_mark.c` + 引擎所需的最小 C 源文件集合（与手工验证一致）。
- 输出：写入可写临时目录（例如 `$TMPDIR/ridl-builder-tests/mquickjs_gc_mark_selftest`）。

兼容性：

- Linux/macOS：默认 `cc` 可用。
- 若宿主缺少 `cc`，应给出明确错误提示（引导安装 build-essential / clang 等）。

### 2. 自检执行

运行生成的二进制：

- 透传 stdout/stderr。
- 若 exit code != 0：`ridl-builder` 以非 0 退出。

### 3. 路径与可移植性

- `ridl-builder` 的 workspace root 可通过 `env!("CARGO_MANIFEST_DIR")` 向上定位 `[workspace]`（现已有 `find_workspace_root()`）。
- 所有源码路径都以 workspace root 绝对路径构造，避免对 cwd 的隐式依赖。

### 4. 输出与诊断

打印：

- 编译命令（简化版，至少打印输出路径与关键输入文件）；
- 运行路径；
- 失败时提示：
  - “找不到 selftest_gc_mark.c”
  - “未找到 cc”
  - “编译失败（请查看 stderr）”

### 5. 测试策略

- 对 `ridl-builder` 的单元测试：验证命令行解析/分支可达；
- 对 end-to-end：在本仓库回归脚本中执行 `cargo run -p ridl-builder -- selftest-gc-mark`。

## 交付清单

- `ridl-builder/src/main.rs`：新增 `selftest-gc-mark` 命令分支与实现函数。
- （如需要）`ridl-builder/src/...`：新增 `selftest` 模块封装编译+执行逻辑。
- 新增规划文档（本文）。
- 回归验证：`cargo test` + `cargo run -- tests` + `cargo run -p ridl-builder -- selftest-gc-mark`。

## 风险与缓解

- 风险：不同平台 C 编译器参数差异。
  - 缓解：优先沿用当前手工验证所需的最小参数集；失败时输出完整 stderr。
- 风险：引擎源码文件集合变动导致 selftest 链接失败。
  - 缓解：将“需要的 C 文件列表”集中在 `ridl-builder` 的单一函数中，并在变动时同步更新；
    同时让错误提示指向该列表。

## 状态

- 状态：待确认 → 待实现
