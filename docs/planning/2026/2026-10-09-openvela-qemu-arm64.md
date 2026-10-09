# OpenVela Phase 3：QEMU aarch64 树内集成移植计划（v2，对抗复核后）

> 状态：**已完成 2026-10-09**（3.1-3.5 全部落地；整体对抗复核判定"通过"，
> 无 Critical；1 项预存 flaky SIGSEGV 单列专项
> `2026-10-09-async-value-flaky-sigsegv.md`）
>
> **终态验收**：QEMU arm64（qemu-armv8a:mqjs，RS=y）四哨兵全绿
> （demo_pass / rs_probe `no-std` / ridl_console / tiny_err 不崩）；sim 四
> 哨兵零回归；上游 618/0（616 基线 + 2 个 feature 卫生测试，ridl-tool 子集
> 426/0）+ JS 31/31；镜像审计（防重门/符号单一定义/nm -u 空）逐字复验。
> 复核员独立复跑全部声明成立（含 worktree@HEAD 证明 flaky 为预存）。
> 修订记录：v1 主线为 Rust.mk 树内机制；复核证伪（arm64 无 Rust 链路、build-std
> 无注入点、adapter 依赖闭包与防重门失效），v2 采纳替代主线：**app 本地 cargo +
> apps/staging 链接口，零共享文件改动**。
> 原则：树内集成 = 纯新增文件（app 目录、defconfig），不改任何受跟踪文件。

## 1. 目标与非目标

**目标**：`js` builtin（C 引擎 + Rust stdlib adapter + RIDL console）以树内 app
形态（`apps/system/mqjs/`）跑在 QEMU aarch64（nuttx 板 `qemu-armv8a`，-M virt）
上，四哨兵（demo_pass / rs_probe / ridl_console / tiny_err）全绿。
机制验证（树内集成形态）在 sim 上先行（host target，已知域）；target/ABI 验证
在 QEMU 上进行——两者解耦（复核 I1）。

**非目标**：goldfish（vendor 配置虽在，产品特性重，后续另立）、CMake 轨验收
（CMakeLists 占位同步维护）、32 位板、真机、sim 轨脚本方案的下线（deprecated
保留）。

## 2. 勘察结论（v2，经复核修正）

| 事项 | 结论 |
|------|------|
| 板级口径 | **锁定树内 nuttx 板 `qemu-armv8a`**（configs/nsh 基线 + 自增 configs/mqjs）；vendor 的 qemu-arm64-v8a-ap / goldfish-arm64-v8a-ap 均不用（产品特性/依赖重） |
| 工具链 | `CROSSDEV ?= aarch64-none-elf-`（arch/arm64/src/Toolchain.defs:208）；PATH 由 `source build/envsetup.sh` 注入 prebuilts 路径——构建前必须 source |
| QEMU | `prebuilts/qemu/linux-x86_64/bin/qemu-system-aarch64`；启动 `-kernel ./nuttx`（ELF 直读，板文档 index.rst:16）`-nographic -semihosting` |
| 语料投递 | `CONFIG_ARM64_SEMIHOSTING_HOSTFS=y`（nsh defconfig:17）→ semihosting hostfs，mount 语法与 sim 同构（fs/hostfs/hostfs.c:1103 解析 fs= 选项），路径相对 QEMU cwd（3.2 实测） |
| arm64 Rust 链路 | **Make 轨无任何 Rust 接线**（Rust.defs 仅 sim/riscv/arm32 include；Rust.mk 无 aarch64 且不传 -Z）→ 不走 Rust.mk，走 app 本地 cargo |
| arm64 Rust target | **tier-2 `aarch64-unknown-none`**（stable、rustup 预编译 core，无 build-std/nightly/自定义 json）；硬浮点 ABI（AAPCS64 v 寄存器）待与 C 引擎 FP 状态对齐验证（3.2 spike） |
| FP/ABI | softfloat 与硬浮点混链是**链接期静默 ABI 破坏**（f64 走错寄存器）；aarch64-none-elf-gcc 默认硬浮点——target 选 `aarch64-unknown-none`（非 softfloat 变体），3.2 实测 qemu FP 使能后定案 |
| 本机 Rust | stable（默认）+ nightly（rust-src 已装，备用）；`vela-nightly` 是空壳不可用 |
| staging 链接口 | `apps/Application.mk:72`：`LDLIBS += $(wildcard $(APPDIR)/staging/*$(LIBEXT))`——Make 轨通用（非 sim 专属），已验收机制的原样树内化 |
| 防重门 | `MQJS_ENGINE_LINK=external`（engine 对象不进 adapter 归档）经 crate-local `.cargo/config.toml` 生效的前提是 **cargo 以 crate 目录为 cwd**——app 本地 cargo 规则天然满足（复核 C3 解法） |
| no_std × RIDL | **`ridl-extensions` feature 隐含 `std`**（mquickjs-rs/Cargo.toml）；d945524 的 no_std adapter 无 console；console 输出桥需 FFI puts 实现 + feature 卫生拆分——列为 3.4 显式工作项（复核 C4） |

## 3. 树内集成形态（openvela 树内全部为新增，零受跟踪文件改动）

```
apps/system/mqjs/
├── Kconfig                    # CONFIG_MQJS_JS / _RS（Rust 桥开关）/ PROGNAME / PRIORITY / STACKSIZE
├── Make.defs                  # CONFIGURED_APPS 注册
├── Makefile                   # CSRCS=engine/*.c + js_main.c；context:: 规则=app 本地 cargo 构建.adapter
├── CMakeLists.txt             # 占位同步维护（本轮不验收）
├── js_main.c                  # NSH 入口（哨兵协议），master 副本在 mquickjs-rs-demo 仓库
├── engine/                    # vendored：deps/mquickjs 核心 + mqjs_stdlib_impl/template/require，SYNC.md 记录来源 rev
├── gen/                       # 集成期生成入库：atom 头 + ridl 聚合（api/register/…）+ 框架 bindgen 头；
│                              #   SYNC.md 注明两条生成命令（ridl-builder aggregate / mquickjs-build）与 diff 自检
└── rust/                      # adapter crate（RIDL 叶子=stdlib console）+ crate-local
                               #   rust-toolchain.toml / .cargo/config.toml（MQJS_ENGINE_LINK=external）
boards/sim/sim/sim/configs/mqjs/defconfig              # sim 配置（新增）
boards/arm64/qemu/qemu-armv8a/configs/mqjs/defconfig   # qemu arm64 配置（新增）
```

上游（mquickjs-rs-demo 仓库：deps/mquickjs 子模块、ridl-tool、adapter 上游、
mquickjs-rs/mquickjs-sys crate）保持 master；树内 `rust/` 为 vendored 闭包
（含 mquickjs-rs/mquickjs-sys 源码或以路径外链——3.1 spike 定案，倾向入树
扁平化以兑现 vendored 语义），SYNC.md 为对账依据 + 重生成 diff 自检命令。

## 4. 实施步骤（v2）

### 3.1 树内 app 落树，sim 机制验证（1 天）
- 落树内全部骨架（C + rust/ + 两份 defconfig 的 sim 份）
- sim：`sim:mqjs` 配置构建；adapter 经 app 本地 cargo（host target
  x86_64-unknown-linux-gnu，std 模式——与已验收 sim 完全同 target，机制平移）
- **验收**：四哨兵与既有 sim 结果一致 → 树内集成机制成立
- spike 定案：mquickjs-rs/mquickjs-sys 的 vendored 闭包形态（build.rs 输入、
  bindgen 依赖在集成机的可用性）

### 3.2 QEMU arm64 基线 spike（0.5 天）
- `source build/envsetup.sh`（PATH 注入）→ `tools/configure.sh qemu-armv8a:nsh`
  → make → QEMU 启动进 NSH（锁定完整命令行）
- semihosting hostfs 挂载宿主目录实测（含路径语义）；FP 使能状态实测（定
  aarch64-unknown-none 的 feature 对齐）
- **产出**：启动/挂载命令 + FP 结论（写入 SYNC.md）

### 3.3 QEMU C checkpoint（0.5 天）
- `qemu-armv8a` 的 `configs/mqjs`（Rust 关，CONFIG_MQJS_RS=n，js_main 走历史
  M1-C 变体的 C 桩路径）
- **验收**：demo_pass PASS / tiny_err SyntaxError 不崩——隔离 QEMU/defconfig
  域问题，为 Rust 域问题划清边界

### 3.4 QEMU Rust（1-1.5 天，含 no_std 工作项）
- mquickjs-rs feature 卫生：`ridl-extensions` 不再隐含 `std`（TDD）
- adapter no_std 模式（cfg 切换保留 sim std 模式）：console 输出改 FFI puts 桥
- `aarch64-unknown-none` + stable 交叉构建 adapter → staging → 链接
- audit-symbols arm64 化（分配对保留；host-glibc 绑定对称检查换 NuttX 单边）
- **验收**：rs_probe PASS（rust-bridge ok）

### 3.5 RIDL console 全链路 + 收尾（0.5 天）
- ridl_console PASS；workspace 回归（cargo test 全绿、JS 语料 31/31）
- 知识沉淀（树内集成模式 / QEMU 轨差异 / 复核修正记录）、计划标记完成、提交

## 5. 风险与对策（v2，复核校准）

| 风险 | 对策 |
|------|------|
| vendored 闭包（mquickjs-sys build.rs：mquickjs.build.toml + ridl-builder 产物 + bindgen/libclang）进树形态未定 | 3.1 首项 spike；倾向源码入树扁平化 + 预生成产物入库，构建期零宿主工具依赖 |
| `ridl-extensions`→`std` 依赖链比预期深 | 3.4 拆 feature 前 TDD 摸清 std 实际使用点；不可行则降级：QEMU Rust 面先只做 rs_probe（version/self_test），console 另立任务 |
| aarch64-unknown-none 与引擎 FP ABI 不匹配 | 3.2 实测定案；不符则 target json 加 feature 对齐（仍 stable 可解） |
| semihosting 路径语义差异 | 3.2 实测；备选 ROMFS initrd |
| 树内 gen/ 双源漂移（ridl-builder 聚合 + mquickjs-build 框架产物） | SYNC.md 记录来源 rev + 重生成 diff 自检命令（人工防线 + 命令化校验） |
| 禁止"sim 过 = QEMU 过"推定 | 每阶段哨兵实机复跑；机制验证与 target 验证解耦（sim=host target，QEMU=aarch64） |

## 6. setup-sim.sh 处置

保留不动（已验收路径）；树内方案落地后标注 deprecated，后续任务清理。

## 7. 总估算：3.5-4.5 天

## 附：对抗复核记录

- 复核人：独立子 agent（对抗性 mandate），2026-10-09
- 判定：修改后通过；Critical C1-C4 / Important I1-I6 全部吸收进 v2
- 关键修正：主线从 Rust.mk 树内机制 → app 本地 cargo + staging；arm64 target
  从自定义 nuttx json + build-std → tier-2 aarch64-unknown-none（stable）；
  新增 no_std×RIDL feature 卫生工作项；FP 风险重新定性为链接期 ABI；板级口径
  锁定树内 nuttx 板
