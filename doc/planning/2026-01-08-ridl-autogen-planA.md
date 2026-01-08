# 计划：方案A - 在 mquickjs-rs/build.rs 自动扫描 ridl-modules 并生成/聚合（2026-01-08）

## 背景
当前希望做到：新增一个 RIDL 模块时，不需要手工改 Rust/C 代码或手工维护 include/注册列表；只要把模块放进 `ridl-modules/<module>/src/*.ridl`，构建时自动生成 glue/impl 及聚合文件，并确保符号不会被链接器裁剪。

## 约束（来自 DEVELOPING_GUIDE.md）
- 先写计划并讨论，通过确认后才能开始开发。
- 任何需求开发都需要测试用例；先写测试并请你审阅。
- 遇到不确定/阻塞优先询问，不猜。
- 功能完成后更新相关文档，且不修改/不删除 DEVELOPING_GUIDE.md。

## 目标
1) `deps/mquickjs-rs/build.rs` 在构建时：
   - 扫描 `deps/mquickjs-rs/ridl-modules/*/src/*.ridl`（约定路径）
   - 对每个 `.ridl` 调用 `deps/ridl-tool` 生成 `<name>_glue.rs` / `<name>_impl.rs`
   - 调用 `aggregate` 生成共享聚合产物（至少 `ridl_symbols.rs` / `mquickjs_ridl_register.h`）
2) 生成一个集中入口（如 `generated/ridl_modules.rs`）强制拉入/集中注册，新增模块零改动。
3) 避免在 build.rs 内部 `cargo run` 导致 Cargo lock 死锁；改为直接执行 `ridl-tool` 二进制（可用 `RIDL_TOOL_BIN` 覆盖）。

## 范围
- **包含**：`deps/mquickjs-rs/build.rs` 的 RIDL 扫描与生成流程、生成文件落盘位置、rerun-if-changed 触发、符号保活（force_link）。
- **不包含**：重做 mquickjs-sys / mquickjs-rs 拆分方案（那是另一份计划）。

## 实施步骤
1) 现状核对
   - 检查当前 build.rs 是否已实现：扫描、module/aggregate 两阶段生成、生成文件布局。
   - 明确 `generated/` 与 crate root 下 shims（如 `*_impl.rs` shim）是否仍必要。

2) 生成流程
   - `discover_ridl_files(ridl-modules/*/src/*.ridl)` 得到稳定排序列表。
   - 对每个 `.ridl`：执行 `ridl-tool module <ridl> <generated_dir>`。
   - 生成 `generated/ridl_modules.rs`：
     - `#[path="...generated/<name>_impl.rs"] mod <name>_impl;`
     - `#[path="...generated/<name>_glue.rs"] mod <name>_glue;`
     - `pub use ...;`
     - `pub fn force_link()`：引用每个模块的 `__ridl_force_link` 符号。
   - 执行 `ridl-tool aggregate <all ridl...> <generated_dir>`。
   - 将聚合产物（如 `ridl_symbols.rs`/`mquickjs_ridl_register.h`）复制到 crate root（若现有 include 依赖此路径）。

3) 构建触发与稳定性
   - `cargo:rerun-if-changed`：目录 + 每个 `.ridl` + 关键生成输出（如 ridl_modules.rs、shim）。
   - `RIDL_TOOL_BIN` 环境变量优先；默认路径指向 `deps/ridl-tool/target/debug/jidl-tool`。

4) 测试（先做）
   - 添加/调整测试，覆盖：
     - 扫描逻辑对新增 module 的可发现性（最小 dummy ridl）。
     - `generated/ridl_modules.rs` 是否包含对应 module 的导出与 `force_link` 引用。
     - aggregate 输出文件存在且包含期望符号声明（基于固定输入 RIDL）。
   - 说明：若现有测试框架/目录结构不明，需要先调研项目现有测试写法再定。

5) 验证
   - 按项目约定命令运行 `cargo fmt` / `cargo check` / `cargo test`（以仓库现有脚本/文档为准）。

6) 文档同步
   - 更新与 RIDL 模块接入相关的 README（例如 ridl-modules/registry 或 mquickjs-rs 相关说明）。

## 验收标准
- 在 `deps/mquickjs-rs/ridl-modules/` 新增一个模块（含 `.ridl`）后：无需改 Rust 代码即可编译通过。
- `generated/` 下能看到对应 `<name>_glue.rs` / `<name>_impl.rs`，且 `generated/ridl_modules.rs` 自动包含。
- `ridl_symbols.rs` / `mquickjs_ridl_register.h` 等聚合产物生成成功。
- 测试用例全部通过。

## 风险/待确认问题
1) **ridl-tool 二进制命名与构建方式**：默认 `target/debug/jidl-tool` 是否总成立？是否需要在 CI/开发流程中先构建 ridl-tool？
2) **shim 需求**：当前生成 glue 是否硬编码期望 `CARGO_MANIFEST_DIR/<name>_impl.rs`？若是，shim 的保留是必要的。
3) **测试策略**：构建脚本测试在 Rust 中往往不直观；可能需要将 discover/生成入口拆到可测试模块，或用集成测试跑一次 build。

## 状态
- 进行中（等待你确认计划后再开始编码/补测试）。
