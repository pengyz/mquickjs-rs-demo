# Project Knowledge Index

## Architecture
<!-- 设计决策、模块边界、为什么选 A 不选 B -->

- [GC Root + Traced 统一 tracing 设计](architecture_gc_root_traced_unified_tracing.md) — Root<T> + Traced<T> 统一 tracing GC，用户不感知 mark
- [base vs ridl 两套 QuickJS 输出](architecture_mquickjs_base_vs_ridl_outputs.md) — base 无 RIDL 扩展用于核心层，ridl 含 RIDL 用于应用层

## Gotchas
<!-- 平台坑、反直觉行为（QuickJS/Rust FFI/构建系统） -->

- [QuickJS ROM 机制与 RIDL 扩展关系](gotcha_quickjs_rom_ridl_mechanism.md) — 当初实现时未充分理解 ROM，需要重新审视
- [mquickjs gc_mark 签名与 quickjs 不同](gotcha_mquickjs_gc_mark_signature.md) — 实际签名 (ctx, void *opaque, const JSMarkFunc *mf)，勿凭 quickjs 知识假设
- [mquickjs GC sweep 不调用 finalizer](gotcha_mquickjs_gc_sweep_no_finalizer.md) — opaque Box 泄漏到 context teardown，分配压力测试需限制迭代次数
- [mquickjs ≠ QuickJS](gotcha_mquickjs_is_not_quickjs.md) — 独立项目，共享代码渊源但架构完全不同，不要用 QuickJS 知识推断 mquickjs

## Patterns
<!-- 代码约定、RIDL 模块/测试开发模式 -->

- [RIDL 自动生成 gc_mark](pattern_ridl_gc_mark_auto_gen.md) — RIDL 自动生成 user class gc_mark 枚举 Traced<T> 字段
- [GC 测试用 finalizer 验证回收](pattern_gc_test_finalizer_verification.md) — 通过 finalizer 计数验证对象被回收，而非仅检查引用
- [RIDL 模块按 *.ridl 文件检测](pattern_ridl_module_detection_by_ridl_file.md) — 仅当 src/ 目录包含 *.ridl 文件时才作为 RIDL 模块
- [RIDL PEG 语法设计原则](pattern_ridl_grammar_design.md) — 关键字优先、左递归规避、类型优先级
- [RIDL 解析器测试策略](pattern_ridl_parser_testing.md) — 三层测试架构：语法单元→集成→端到端
- [RIDL 代码生成器测试策略](pattern_ridl_codegen_testing.md) — 类型映射、模板渲染、gc_mark 生成、端到端编译
- [RIDL 全局函数架构](pattern_ridl_global_function_architecture.md) — glue 生成 C FFI，用户在 impls 提供实现，api.rs 不生成

## Debugging
<!-- 调试方法、关键命令、排查路径 -->

## Decisions
<!-- 已验证的取舍（含正反馈） -->

- [Root<T> 优于 Global<T>](decision_root_over_global.md) — 跨 await 持有 JSValue 推荐用 Root<T>，Global<T> 保留兼容
- [Context-level gc_mark 注册](decision_context_level_gc_mark_registration.md) — 一次注册遍历所有 roots，优于每个 Root 单独注册
- [RootsRegistry 用 Vec<Option>](decision_roots_registry_vec_option.md) — Mutex<Vec<Option<JSValue>>> 简单有效，性能足够
- [RIDL 异步取消语义](decision_ridl_async_cancellation.md) — 默认可取消，必须显式标记 @nonCancellable，与主流框架一致

## References
<!-- 外部资源指针、关键文件位置 -->

- [ridl-builder prepare 命令](reference_ridl_builder_prepare_command.md) — 构建前准备：生成 RIDL 聚合、构建 QuickJS base/ridl 输出
- [RIDL 测试覆盖分析](reference_ridl_test_coverage.md) — 当前状态、缺口、优先级（2026-09-04）
