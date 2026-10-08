# ports/openvela — mquickjs 的 OpenVela/NuttX 适配层

将 mquickjs 引擎以 **NSH builtin**（`js` 命令）形态集成进 OpenVela（NuttX 系）。
当前阶段：**Phase 1（M1-C，纯 C 层）已完成** —— Rust 层见计划文档 Phase 2。

## 集成模型：源码级（source-level）

引擎 `.c` 文件**由 openvela apps 构建用 NuttX 工具链编译**（参照
`apps/interpreters/quickjs` 的官方集成模式）。因此所有 libc 符号
（setjmp/longjmp、malloc/free、stdio）一致绑定到 NuttX 侧——不存在
"NuttX 静态 libc 与宿主 glibc 混绑"问题（该问题曾导致首个语法错误即
SIGSEGV，见计划文档 Phase 1 实施纪要）。

引擎原生工具链的角色 = **宿主代码生成**：`mqjs_stdlib` host 工具
（源自 deps/mquickjs，引擎自有工具）生成 atom 表头（`mquickjs_atom.h`）
与 stdlib def（`mqjs_stdlib.h`）。生成头引用的平台钩子（print/Date/
timers/load）由嵌入方实现——`app/js_main.c` 按 `mqjs.c`（引擎 REPL）
同款实现提供；timers 无事件循环，显式 stub。

**层级契约**（与用户裁定一致）：
- C 层：引擎原生 API + 全量 base stdlib（含 console/print 的 C 实现）
- Rust 层（Phase 2/2b）：mquickjs-rs（no_std）+ RIDL stdlib（console
  从 RIDL 层来）。两层不互斥，一个叶子二进制按其形态选择。

## 文件

| 文件 | 用途 |
|------|------|
| `app/js_main.c` | NSH builtin 入口：读脚本 → 每脚本独立 Context eval → 哨兵行汇总 |
| `app/Kconfig` | `CONFIG_MQJS_JS*` 符号（含 STACKSIZE ≥64KB，解析器递归需要） |
| `app/Makefile` | CSRCS 引擎源文件 + js_main.c；CFLAGS 指向 gen/ 与引擎源 |
| `app/Make.defs` | CONFIGURED_APPS 注册（缺失则 builtin 永不构建） |
| `setup-sim.sh` | 幂等装配：符号链接进 apps/system/mqjs、Kconfig source 行、引擎源 staging、host 工具链生成头、configure + 构建 |
| `cases/` | M1-C 语料：demo_pass（assert 全绿）、tiny_err / demo_syntax_error（语法错误 → 必须转 SyntaxError 异常而非崩溃） |

`gen/`、`engine_src/`、`framework.mk` 为机器本地生成物，已 gitignore。

## 用法

```bash
# 前置：openvela 树已 repo init+sync（见 docs/planning/2026/2026-10-08-openvela-port.md）
OPENVELA_DIR=~/workspace/openvela ports/openvela/setup-sim.sh build        # Make 轨
OPENVELA_DIR=~/workspace/openvela ports/openvela/setup-sim.sh build-cmake  # CMake 轨

# 运行（NSH 内；hostfs 的 fs= 前缀是 NuttX hostfs 协议要求）
mount -t hostfs -o fs=<宿主上的绝对目录> /m
js /m/cases/demo_pass.js
js /m/cases/demo_syntax_error.js     # 预期 FAIL: SyntaxError — 验证不崩
```

**双轨状态（2026-10-09）**：Make 轨已集成并验收（M1-C）；CMake 轨
**推迟到 M2**（用户裁定）——openvela 的 CMake 流程要求配置位于树内
（BOARD_CONFIG 解析、optee 自定义模块无条件 include 等），部分同步的
残缺树无法完全非侵入验证。适配物已就绪（`app/CMakeLists.txt`，
`nuttx_add_application` 注册），M2 在 goldfish 完整树 + 官方 CMake
流程下重新验证。CMake 轨排障记录见计划文档 Phase 1.5 节。

## 哨兵行协议

NSH builtin 无可脚本化退出码（NSH 无 `$?`），宿主侧扫描 stdout：

```
CASE <basename> PASS
CASE <basename> FAIL: <message>
CASES <passed>/<total> PASS
```

## 已知怪癖

- `timeout` 下 sim 进程以 124 退出（管道 stdin 的既有行为，与 js 无关）
- `JS_PrintValueF` 对数字的输出在 sim 控制台上未显示（console.log 字符串
  正常）——待查，不阻塞
- `exit` 后 NSH 会话的清理路径与 builtin 无关

## 路线

- Phase 2a：mquickjs-rs（no_std）以 staticlib 进 sim，GlobalAlloc 桥
- Phase 2b：RIDL 去 std 三件套（console 移交 RIDL 层的 no_std 实现）
- Phase 3（M2）：aarch64 QEMU（goldfish-arm64-v8a-ap / qemu-armv8a）
