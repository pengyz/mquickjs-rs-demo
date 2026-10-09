# gotcha: OpenVela QEMU aarch64 树内集成（Phase 3 实测）

**类型**：gotcha（平台坑 + 模式）
**日期**：2026-10-09
**上游**：`docs/planning/2026/2026-10-09-openvela-qemu-arm64.md`（v2）；树内 `apps/system/mqjs/SYNC.md`（完整实测记录）

## 树内一等公民集成的关键机制

- **context pass 只由 `.config` 变化触发**（Unix.mk 门控 config.h 戳）——app
  Makefile 里挂在 `context::` 的规则在 `.config` 不变时**永不执行**。cargo
  增量对 make 不可见 → 改 rust/ 源会静默链旧归档。正解：stamp 挂**正常构
  建图**（自家 TU 的显式前置依赖），且 `$(CURDIR)` 而非环境 `$(PWD)`（子
  make 中后者陈旧）。
- **app 本地 cargo + `apps/staging` 万能链接口**（Application.mk:72 无条件
  LDLIBS wildcard）优于改 Rust.mk：零共享文件改动；crate-local
  `.cargo/config.toml`（如 MQJS_ENGINE_LINK=external 防重门）要求 cargo 以
  crate 目录为 cwd——app 规则天然满足，unified-lib 形态下会失效。
- **no_std × NuttX**：`aarch64-unknown-none`（tier-2 预编译 core）+ 硬浮点
  ABI 与 aarch64-none-elf-gcc 逐指令一致——免 nightly/build-std/自定义 json；
  softfloat 变体是链接期静默 ABI 破坏（f64 走错寄存器），禁用。
  panic=abort 下 `#[panic_handler]` 仍必需（lang item，非 RUSTFLAGS 可免）。
- **多表 target-specific dependencies 按目标合并**：任一适用表漏
  `default-features=false` 都会把 default(std) 拉回依赖图（症状：futures
  等意外进图）。生成代码的模式无关性用 `glue_prelude` 再导出解决（模板级，
  非 vendored 补丁）。

## QEMU/板级坑

- arm64 引擎用 setjmp：`CONFIG_ARCH_SETJMP_H=y` 必须显式开（arm64 libc 仅
  此开关下编入 arch_setjmp.S，缺省**链接期**才报错）。
- semihosting hostfs：mount 语法与 sim 同构；`ls` 不可用（opendir 是桩）；
  路径相对 QEMU cwd；QEMU 需 `-semihosting`。NSH `CONFIG_NSH_LINELEN=80`
  截断 + stdio→PL011 无流控——脚本化交互须逐字符 ~12ms 喂入。
- 适配器归档含 LTO fat object：系统 binutils LLVM 插件版本不配时回退内嵌
  机器码才链得上——换机/升级 binutils 后必须重跑哨兵。

## 遗留指针

`test_async_value_object` 预存 flaky SIGSEGV——**已解决（2026-10-09）**：根因
是 `AsyncValue::to_js` Json 路径 `JS_Call(ctx, 0)` 零推参（栈残留被当函数帧
解释），非 GC 问题；修复 + 确定性往返断言 + 50 轮压测零崩溃。
`docs/planning/2026/2026-10-09-async-value-flaky-sigsegv.md`。
教训：**C 调用约定类 flaky 会被误诊为 GC 问题**——先审计推参/帧纪律再怀疑
生命周期。

**关联**：[[gotcha_vela_cmake_track]]、[[nuttx-prebuilt-archive-mixed-binding]]
