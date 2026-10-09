---
name: gotcha-rust-nostd-symbols-nuttx-sim
description: Rust no_std 静态库进 NuttX sim 的符号绑定清单与 staging/EXTRA_LIBS 消费机制
type: gotcha
created: 2026-10-09
sources: [ports/openvela/setup-sim.sh, ports/openvela/rust/, 8c17153]
---

**符号绑定清单**（Rust no_std staticlib → NuttX sim 镜像，实测 2026-10-09）：

| Rust 侧 UND 符号 | 镜像内解析到 | 备注 |
|------------------|-------------|------|
| `malloc`/`free`/`calloc` | **宿主 glibc**（sim 的 NuttX libc.a 不定义它们） | 引擎 C 与 Rust 一致共享宿主分配器 |
| `memcpy`/`memset`/`memcmp` | NuttX libc.a | compiler_builtins(linux-gnu) 不自带 mem 实现 |
| `js_stdlib` | 生成头 weak def（`gen/mqjs_stdlib.h:2691`） | Rust 强 UND 契约 |
| `printf`/`strlen`/`abort` | NuttX libc.a / 宿主 | |
| `JS_*`（引擎 FFI） | openvela 源码级编译的引擎对象 | MQJS_ENGINE_LINK=external |

**staging 消费机制**：sim 平铺镜像走 `nuttx/arch/sim/src/Makefile:330` 的
`EXTRA_LIBS += $(wildcard $(APPDIR)/staging/*.a)`（在 LDSTART/ENDGROUP 组内），
**不是** `apps/Application.mk` 的 staging 通配（那只作用于 BUILD_MODULE 应用）。

**多轮处理**：openvela CMake 流程会多轮处理 apps 子目录（kconfig 前后各一轮），
app CMakeLists 必须幂等（`if(TARGET apps_xxx) return()`）。

**Why**: 混绑（同一分配对绑到不同实现）是堆损坏定时炸弹；Phase 1 的
setjmp 混绑就是实例。审计已制度化进 setup-sim.sh `audit-symbols`。

**How to apply**: 新增 Rust 静态库进镜像时，跑 `audit-symbols` 核对
分配对成套 + 绑定对称；`sim_appinit.c` 类 SRCS 收集依赖 .config 驱动的
CONFIG_* 变量，确保 configure 先于 cmake -B 执行。
