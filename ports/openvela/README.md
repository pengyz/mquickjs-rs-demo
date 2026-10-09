# ports/openvela — mquickjs 的 OpenVela/NuttX 适配层

将 mquickjs 引擎以 **NSH builtin**（`js` 命令）形态集成进 OpenVela（NuttX 系）。
当前阶段：**Phase 1（M1-C）+ Phase 2a（M1-R Rust std adapter）+ Phase 2b
（M1-R+ RIDL stdlib console）已完成**（2026-10-09）。

> ⚠️ **DEPRECATED（2026-10-09 起）**：本目录的脚本式适配已由**树内一等公民
> app** 取代——OpenVela 树 `apps/system/mqjs/` + `boards/sim/sim/sim/configs/mqjs`
> （构建与哨兵说明见树内 `apps/system/mqjs/SYNC.md`）。**勿再运行
> setup-sim.sh**：它会覆盖树内 app 与 defconfig 并产生双适配器归档冲突。
> 本目录保留为历史验收路径与上游 master 副本（js_main.c 等的上游 source
> of truth），供树内 vendored 快照对账（SYNC.md 自检命令引用）。

## 集成模型：源码级（source-level）

引擎 `.c` 文件**由 openvela apps 构建用 NuttX 工具链编译**（参照
`apps/interpreters/quickjs` 的官方集成模式）。因此所有 libc 符号
（setjmp/longjmp、malloc/free、stdio）一致绑定到 NuttX 侧——不存在
"NuttX 静态 libc 与宿主 glibc 混绑"问题（该问题曾导致首个语法错误即
SIGSEGV，见计划文档 Phase 1 实施纪要）。

引擎原生工具链的角色 = **宿主代码生成**：`mqjs_stdlib` host 工具从
**`mqjs_stdlib_template.c`**（deps/mquickjs-rs，ridl 变体模板）+
`mquickjs_build.c` 以 `-DMQUICKJS_ENABLE_RIDL_EXTENSIONS` 构建，生成
atom 表头（`mquickjs_atom.h`）与 stdlib def（`mqjs_ridl_stdlib.h`，
含 **strong `js_stdlib`** 定义）。生成头引用的函数：

- 引擎内部 builtin（Object/Array/...）→ `mquickjs.c`
- `js_date_constructor`/`js_date_now` → `mqjs_stdlib_impl.c`（gettimeofday
  在 NuttX 侧）
- RIDL 扩展（**console**）→ Rust adapter 的 `js_global_singleton_console_*`
  胶水符号
- rs 探针（`rsVersion`/`rsSelfTest`，staged 模板 overlay 注入）→
  `js_main.c`（非 static——生成表现在 `mqjs_stdlib_impl.o`，跨 TU 引用；
  声明经 `gen/mqjs_rs_hooks.h` 强制包含）

**层级契约**（与用户裁定一致）：
- C 层：引擎原生 API；sim 镜像的 stdlib 表 = **ridl 变体**（语言 builtin
  来自引擎，console/print 等宿主层对象按模板契约全部由 RIDL 层提供）
- Rust 层（Phase 2/2b）：mquickjs-rs（std 模式）+ RIDL stdlib 模块
  （console singleton，`println!` → stdout = NSH 控制台）

**Context 创建在 Rust 侧**（关键约束）：mquickjs 没有运行时注册 API，
RIDL 胶水（`js_global_singleton_console_log` 等）经 `JSContext` 的
user_data 分派（`ContextToken::from_js_ctx` → `ContextInner` → ridl_ext
slot），而 user_data 只有 `mquickjs_rs::Context::new` 会安装——C 侧
`JS_NewContext` 直接创建的 context 无法承载 RIDL 胶水（会抛
"missing ctx user_data"）。因此 `js_main.c` 使用 adapter 三件套：
`mqjs_rs_ridl_context_new`（bootstrap 一次 + `ridl_context_init` +
C 侧 `JS_RIDL_StdlibInit`）/ `mqjs_rs_ridl_eval` /
`mqjs_rs_ridl_context_free`。引擎堆块由 Rust 侧持有并随
context_free 释放（C 侧不再 malloc JS 堆）。

## sim 应用的 RIDL 聚合

`ports/openvela/rust` 自身即 RIDL 叶子应用（`mquickjs.ridl.toml`，模块
选择 = `[dependencies.stdlib]`，仅 console）。聚合产物由
`ridl-builder aggregate --cargo-toml ports/openvela/rust/Cargo.toml
--intent build` 生成到 `rust/target/ridl/apps/mqjs_openvela_adapter/
aggregate/`（gitignored）：

| 产物 | 消费方 |
|------|--------|
| `mquickjs_ridl_register.h/.c`、`mquickjs_ridl_api.h` | 宿主工具（扩展进 stdlib 表）+ 镜像内 `mqjs_ridl_register.c` TU（`JS_RIDL_StdlibInit` + require 表）+ adapter 的 bindgen |
| `ridl_symbols.rs` / `ridl_context_ext.rs` / `ridl_bootstrap.rs` | adapter build.rs → OUT_DIR（keepalive 表 / CtxExt+`ridl_context_init` / 进程级初始化） |

## 文件

| 文件 | 用途 |
|------|------|
| `app/js_main.c` | NSH builtin 入口：读脚本 → adapter 三件套（创建 RIDL Context → eval → 销毁）→ 哨兵行汇总；定义 rs 探针钩子 |
| `app/Kconfig` | `CONFIG_MQJS_JS*` 符号（含 STACKSIZE ≥64KB，解析器递归需要） |
| `app/Makefile` | CSRCS = 引擎核心 + ridl stdlib 运行时 TU（`mqjs_stdlib_impl.c`/`mqjs_require.c`/聚合 `mqjs_ridl_register.c`）+ js_main.c；CFLAGS 指向 gen/、聚合目录与引擎源 |
| `app/CMakeLists.txt` | CMake 轨同构（**推迟到 M2 验证**，用户裁定） |
| `app/Make.defs` | CONFIGURED_APPS 注册（缺失则 builtin 永不构建） |
| `setup-sim.sh` | 幂等装配：符号链接、引擎源 staging（core + ridl 运行时 TU + 模板）、sim 聚合生成、rs overlay（staged 模板）、host 工具 + 头生成、defconfig 合成、configure + 构建 + 符号审计 |
| `rust/` | Rust std adapter crate = RIDL 叶子应用（聚合消费 + C 导出面：版本/自检/ridl context 三件套 + console 胶水符号） |
| `cases/ridl_console.js` | M1-R+ 验收语料（console.log 由 Rust RIDL singleton 输出） |
| `cases/rs_probe.js` | M1-R 验收语料（JS→C→Rust 全链路；其 console.log 现同样走 RIDL） |
| `cases/` | M1-C 语料：demo_pass（assert 全绿）、tiny_err / demo_syntax_error（语法错误 → 必须转 SyntaxError 异常而非崩溃） |

`gen/`、`engine_src/`、`framework.mk`、`rust/target/` 为机器本地生成物，
已 gitignore。

## 用法

```bash
# 前置：openvela 树已 repo init+sync（见 docs/planning/2026/2026-10-08-openvela-port.md）
OPENVELA_DIR=~/workspace/openvela ports/openvela/setup-sim.sh build        # Make 轨（含 Rust adapter 构建 + 符号审计）
OPENVELA_DIR=~/workspace/openvela ports/openvela/setup-sim.sh build-cmake  # CMake 轨

# 运行（NSH 内；hostfs 的 fs= 前缀是 NuttX hostfs 协议要求）
mount -t hostfs -o fs=<宿主上的绝对目录> /m
js /m/cases/ridl_console.js          # 输出 "ridl console in openvela sim"（Rust println! 通路）
js /m/cases/demo_syntax_error.js     # 预期 FAIL: SyntaxError — 验证不崩
```

**双轨状态（2026-10-09）**：Make 轨已集成并验收（M1-C/M1-R/M1-R+）；CMake 轨
**推迟到 M2**（用户裁定）——openvela 的 CMake 流程要求配置位于树内
（BOARD_CONFIG 解析、optee 自定义模块无条件 include 等），部分同步的
残缺树无法完全非侵入验证。适配物已就绪（`app/CMakeLists.txt` 与 Makefile
同构），M2 在 goldfish 完整树 + 官方 CMake 流程下重新验证。CMake 轨排障
记录见计划文档 Phase 1.5 节。

## 哨兵行协议

NSH builtin 无可脚本化退出码（NSH 无 `$?`），宿主侧扫描 stdout：

```
CASE <basename> PASS
CASE <basename> FAIL: <message>
CASES <passed>/<total> PASS
```

异常文本来自 Rust 侧 eval（`JS_GetException` → `JS_ToCString`），与
Phase 1 直连 C 路径同文。

## 已知怪癖

- `timeout` 下 sim 进程以 124 退出（管道 stdin 的既有行为，与 js 无关）
- host 工具构建时打印 `Too many properties, consider increasing ATOM_ALIGN`
  为既有哈希钳制警告（base/ridl 变体均出现，非致命）
- `exit` 后 NSH 会话的清理路径与 builtin 无关
- NSH 命令行长度上限约 64 字符（CONFIG_NSH_LINELEN 默认值）：hostfs 挂载
  的宿主路径过长会被截断——语料目录请用短路径（如 `/tmp/mc`）
- 机器本地构建状态共享：`target/mquickjs-build/framework/<triple>/release/`
  的 ridl 变体现在由 **sim 聚合**驱动（app-id 无目录是 ridl-builder 的既有
  TODO）。若要跑 `cargo run -p mquickjs-demo --release`，先重跑
  `cargo run -p ridl-builder -- prepare` 恢复 demo 变体（debug 轮不受影响）

## 路线

- ~~Phase 2a：mquickjs-rs（std）以 staticlib 进 sim~~ ✅ b531dd7
- ~~Phase 2b：RIDL stdlib console（Rust singleton 经聚合 + ridl 变体 stdlib
  接入 sim）~~ ✅ 2026-10-09
- Phase 3（M2）：aarch64 QEMU（goldfish-arm64-v8a-ap / qemu-armv8a），
  含 CMake 轨复验
