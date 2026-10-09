# Project Knowledge Index

> 标注 【已作废】的条目保留了原始内容与作废原因，用于防止旧结论继续被引用。
> 项目规则见根目录 `AGENTS.md`；知识写入规范见 `RULES-KNOWLEDGE-MEMORY.md`。

## Architecture
<!-- 设计决策、模块边界、为什么选 A 不选 B -->

- [GC Root + Traced 统一 tracing 设计](architecture_gc_root_traced_unified_tracing.md) — Root<T>/Traced<T> 统一基于引擎 JSGCRef；用户不感知 mark，也不再有 class gc_mark
- [base vs ridl 两套 QuickJS 输出](architecture_mquickjs_base_vs_ridl_outputs.md) — 归档拆分（core / stdlib_base / stdlib_ridl）；**变体由叶子二进制选择**
- [base/ridl 变体选择必须由叶子决定](architecture_base_vs_ridl_variant_selection.md) — js_stdlib 是唯一连接点；记录三项修复与四个构建踩坑
- [工作区目录职责约定](architecture_workspace_layout.md) — 虚拟清单；apps/ 放应用；deps/ 混放产品与 vendored 引擎（已知且接受）
- [核心 no_std 移植成本评估](assessment_core_nostd_port_cost.md) — PoC 实证可编译到 thumbv7em；14 文件 138 行改动；剩余阻塞项清单

## Gotchas
<!-- 平台坑、反直觉行为（QuickJS/Rust FFI/构建系统） -->

- [GC 压缩与 finalizer（含对旧结论的更正）](gotcha_mquickjs_gc_compaction_and_finalizer.md) — **每次 JS_GC 都压缩堆**；sweep **确实**调用 finalizer；Root/Traced 必须用 JSGCRef
- 【已作废】[mquickjs GC sweep 不调用 finalizer](gotcha_mquickjs_gc_sweep_no_finalizer.md) — 结论错误，见上一条
- [mquickjs gc_mark 签名与 quickjs 不同](gotcha_mquickjs_gc_mark_signature.md) — 实际签名 (ctx, void *opaque, const JSMarkFunc *mf)，勿凭 quickjs 知识假设
- [mquickjs-rs 不能编译到裸机](gotcha_mquickjs_rs_not_bare_metal.md) — E1 结论：阻塞集中在异步子系统；RIDL/GC/handles 核心是 no_std 干净的
- [Mimosa git-gate 高危分诊](gotcha_mimosa_git_gate_triage.md) — 本地 CLI argv 路径属接受风险；测试消息勿写成命令行样式；放行须批准且最小范围
- [Rust no_std 符号绑定清单（NuttX sim）](gotcha_rust_nostd_symbols_nuttx_sim.md) — malloc/free 落宿主 glibc、mem 落 NuttX libc.a；staging 消费点在 arch/sim Makefile:330
- [OpenVela CMake 轨适配五连坑](gotcha_vela_cmake_track.md) — lunch 覆盖 env/configure 在 m 里/树外 defconfig 断板级链/COMPILE_FLAGS 多词 flag/GLOB 不穿透符号链接
- [OpenVela QEMU aarch64 树内集成](gotcha_openvela_qemu_in_tree_port.md) — context pass 只认 .config（stamp 挂正常构建图）/app 本地 cargo+staging/aarch64-unknown-none 硬浮点直用/semihosting 无 ls/arm64 setjmp Kconfig
- [mquickjs ≠ QuickJS](gotcha_mquickjs_is_not_quickjs.md) — 独立项目，共享代码渊源但架构完全不同，不要用 QuickJS 知识推断 mquickjs
- [RIDL 生成器静默退化](gotcha_ridl_silent_generation_degradation.md) — union 丢成员/struct 回退 JSValue 曾不报错；兜底产物被使用即错 → 硬错误
- [QuickJS ROM 机制与 RIDL 扩展关系](gotcha_quickjs_rom_ridl_mechanism.md) — 当初实现时未充分理解 ROM，需要重新审视
- 【已作废】[cargo test --workspace 链接失败](gotcha_workspace_test_link_failure.md) — 曾误判为"架构固有"，实为变体选择放错位置

## Patterns
<!-- 代码约定、RIDL 模块/测试开发模式 -->

- [GC 测试用 finalizer 验证回收](pattern_gc_test_finalizer_verification.md) — 通过 finalizer 计数验证对象被回收，而非仅检查引用
- [RIDL 模块按 *.ridl 文件检测](pattern_ridl_module_detection_by_ridl_file.md) — 仅当 src/ 目录包含 *.ridl 文件时才作为 RIDL 模块
- [RIDL PEG 语法设计原则](pattern_ridl_grammar_design.md) — 关键字优先、左递归规避、类型优先级
- [RIDL 解析器测试策略](pattern_ridl_parser_testing.md) — 三层测试架构：语法单元→集成→端到端
- [RIDL 代码生成器测试策略](pattern_ridl_codegen_testing.md) — 类型映射、模板渲染、端到端编译
- [named-type override pass 范式](pattern_named_type_override_pass.md) — 聚合级命名类型走渲染前覆写（union 先行），不改无状态 filter
- [RIDL 全局函数架构](pattern_ridl_global_function_architecture.md) — glue 生成 C FFI，用户在 impls 提供实现，api.rs 不生成
- 【已作废】[RIDL 自动生成 gc_mark](pattern_ridl_gc_mark_auto_gen.md) — 有致命 ABI 缺陷且已冗余，机制整体移除

## Debugging
<!-- 调试方法、关键命令、排查路径 -->

## Decisions
<!-- 已验证的取舍（含正反馈） -->

- [Root<T> 优于 Global<T>](decision_root_over_global.md) — 跨 await 持有 JSValue 推荐用 Root<T>，Global<T> 保留兼容
- [RIDL 异步取消语义](decision_ridl_async_cancellation.md) — 默认可取消，必须显式标记 @nonCancellable，与主流框架一致
- [放弃 slint UI 与应用框架方向](decision_abandon_ui_app_framework.md) — 四路对抗性复核结论；含重新考虑的前置条件
- [SDK 正名与虚拟清单](decision_sdk_repositioning.md) — demo → SDK；根包迁 apps/demo 包名不变；ridl-builder 发现规则重设计
- 【已作废】[Context-level gc_mark 注册](decision_context_level_gc_mark_registration.md) — 该路径无法重定位，已改为 per-instance JSGCRef
- 【已作废】[RootsRegistry 用 Vec&lt;Option&gt;](decision_roots_registry_vec_option.md) — 存放裸 JSValue + gc_mark 保活在压缩式 GC 下不安全

## References
<!-- 外部资源指针、关键文件位置 -->

- [ridl-builder prepare 命令](reference_ridl_builder_prepare_command.md) — 构建前准备：生成 RIDL 聚合、构建 QuickJS base/ridl 输出
- [RIDL 测试覆盖分析](reference_ridl_test_coverage.md) — 当前状态、缺口、优先级（2026-09-04）