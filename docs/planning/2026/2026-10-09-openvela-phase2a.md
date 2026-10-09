# OpenVela Phase 2a：Rust no_std 核心进 sim（M1-R）实施计划

> 状态：✅ 已实施完成（M1-R 验收通过 2026-10-09；已独立复核（2026-10-09，判定"修订后通过"，前置条件已吸收；复核含 /tmp 探针端到端实证：stable 1.94 + x86_64-linux-gnu 下 Context::new(2MB)→eval 全链路走通）  
> 复核前置条件（已并入 D1/D2/D4）：① panic="abort" profile（缺失则 no_std 编译直接报错）② rust_eh_personality 空桩（hosted 目标预编译 alloc 按 unwind 构建，DW.ref 必现，rust#56152/#106864 标准 workaround）③ 禁止写 alloc_error_handler（1.68+ 有默认）④ adapter Cargo.toml 加空 [workspace]（仓库树内非 member 会报 workspace 错误）⑤ MQJS_ENGINE_LINK=external 必须恢复（build.rs:31,54 无条件 bundle 引擎对象进 staticlib，链接顺序静默选择副本——Phase 1 混绑教训的回归形态；可用 static:-bundle 修饰符替代）⑥ 符号审计升级为双层（中间 .a + 最终镜像 -Wl,-Map；新增 js_stdlib 弱符号绑定检查）⑦ --release 需先跑 ridl-builder build-mquickjs 的 release 变体 ⑧ D4 链接接入点需实做（ar 合并对象进 libapps.a 或 external/ 机制）⑨ CMake 轨在 2a 降级单轨（用户已裁定推迟到 M2）  
> 日期：2026-10-09  
> 前置：Phase 1（M1-C）已完成（94170f9）；本阶段目标为计划文档中的 **M1-R** 里程碑  
> 流程：方案 → 子 agent 独立复核 → 实施

## 目标与验收（M1-R）

在 OpenVela sim 镜像中链接 **mquickjs-rs 核心（no-std feature）**：

1. 镜像正常启动（NSH 可用，既有 C 层 js builtin 不回归）
2. Rust 侧导出一个可从 C 调用的验证函数（如 `mqjs_rs_version() -> *const c_char`），
   由 `js` builtin 暴露为 JS 可调用的 `rsVersion()`，`console.log(rsVersion())`
   输出正确 → 证明 **Rust FFI 双向通路** 在镜像内工作
3. Rust 侧 `alloc`（BTreeMap/String 等 RootsRegistry 依赖）在 NuttX 堆上工作
   （分配/释放可观测，无崩溃）

明确不做（2a 范围外）：RIDL stdlib console（2b）、Traced<T>（Phase 3）、
异步子系统（永久排除）。

## 现状与事实基础

| 事实 | 来源 |
|------|------|
| mquickjs-rs 有 `no-std` feature：`#![cfg_attr(feature = "no-std", no_std)]` + `extern crate alloc`，**排除异步子系统** | deps/mquickjs-rs/src/lib.rs、Cargo.toml（std/no-std 互斥 compile_error 守卫） |
| no_std PoC 已实证核心可编译（thumbv7em 交叉 + stub libc）；sim 是 **x86_64 Linux 宿主进程**（动态链 glibc） | docs/knowledge/assessment_core_nostd_port_cost.md、Phase 1 构建日志（sim-pac 打包 libc.so.6） |
| sim 镜像的 libc 双层结构：NuttX libc（编进镜像）+ 宿主 glibc（动态） | Phase 1 实测（sim-pac/ld-linux 等） |
| NuttX libc.a 定义 `malloc/free/strdup` 等 POSIX 面；`setjmp` 有 arch 实现 | Phase 1 链接成功（Make 轨镜像完整链接） |
| 引擎 C 对象已在镜像内（Phase 1 源码级集成）；mquickjs-rs 经 mquickjs-sys (bindgen) 引用 `JS_*` 符号 | deps/mquickjs-sys/build.rs |
| Rust `alloc` 需要全局分配器：`#[global_allocator]` 必须在最终二进制中恰好一份 | Rust 语言要求 |
| Tier-1 `x86_64-unknown-linux-gnu` + no-std feature **无需 nightly** | 平台支持文档 |

## 设计决策

### D1：构建形态 = staticlib（adapter crate 模式）

新建 `ports/openvela/rust/` 适配 crate（workspace 外独立构建，产物 `.a` 链入镜像）：

```
ports/openvela/rust/
├── Cargo.toml      # crate-type=["staticlib"], 依赖 path=deps/mquickjs-rs (no-std)
├── src/lib.rs      # GlobalAlloc 桥 + panic handler + C 导出面
└── rust-toolchain.toml  # 锁定工具链（stable 即可，见 D3）
```

- mquickjs-rs 以 `no-std` feature 编译为 rlib（依赖图自动传播）
- adapter crate 提供 Rust `alloc` 运行所需的**唯一定义**：
  - `#[global_allocator]`：桥接 NuttX libc 的 `malloc/free`（`extern "C"` 声明，
    链接期解析到镜像内 NuttX libc.a 的实现——与引擎 C 代码同一分配器）
  - `#[panic_handler]`：`printf` 报告 + `abort`（镜像内无 Rust std，不会冲突；
    NuttX C 代码不含 Rust panic 机制）
- C 导出面（2a 最小集）：`mqjs_rs_version()`、`mqjs_rs_self_test() -> i32`
  （内部构造一个 `Context`、eval "1+1"、校验结果、返回 0/非 0——
  一次调用覆盖 Context 创建/eval/GC/释放全链路）

### D2：符号绑定审计（Phase 1 教训的制度化）

Phase 1 的 setjmp 混绑教训 → 2a 在链接后强制做**符号绑定审计**：

```bash
# 引擎/Rust 归档引用的每个 libc 符号，必须解析到同一实现集合
nm --undefined-only libmqjs_rs.a | sort > rust-unds.txt
# 核对：malloc/free 对、memcpy/memset/memcmp/memmove 对、无半对半错
```

审计脚本进 `setup-sim.sh`（`audit-symbols` 子命令），发现 malloc/free 类
**分配对拆绑**（一个 NuttX 一个 glibc）即失败——这是堆损坏的定时炸弹。

### D3：工具链 = stable（不引入 nightly）

x86_64-unknown-linux-gnu 是 Tier-1，no-std crate 无需 nightly、无需 build-std。
`rust-toolchain.toml` 锁定 stable 版本，写入 ports/openvela/rust/。
（M2 aarch64-nuttx 才需要 nightly + build-std，届时另锁。）

### D4：构建集成 = apps Makefile 外挂 cargo 步骤

参照 apps/tools/Rust.mk 的模式但不依赖它（那是为 apps 树内 crate 设计的）：
app Makefile 增加 `$(call RUN_CARGO)` 前置目标：`cargo build --release
--manifest-path ports/openvela/rust/Cargo.toml --target x86_64-unknown-linux-gnu`，
产物 `.a` 加入链接（复制到 apps/staging/ 或 LDLIBS 追加，二选一，实施时按
Make.track 现状取简）。

### D5：双向验证路径（M1-R 验收的技术形态）

```
js_main.c                      libmqjs_rs.a (staticlib)
  mqjs_rs_self_test()  ──FFI──▶  Rust: Context::new(2MB block)
  ◀── return 0 ────────────────       eval("1+1") == 2
  JS: rsVersion() ──FFI──▶       mqjs_rs_version() -> "mquickjs-rs 0.1 (no-std)"
```

- `js_main.c` 新增 `rsVersion`/`rsSelfTest` 两个 C 函数注册进 js_stdlib 的
  **实际实现（复核后修正）**：mquickjs 无 QuickJS 式运行时注册 API（核心约束），
  采用**编译期 stdlib overlay**——setup-sim.sh 将 rsVersion/rsSelfTest 条目 sed
  注入 staged 的 mqjs_stdlib.c 副本（锚点校验），引擎生成器据此生成含两函数的
  mqjs_stdlib.h；js_main.c 提供钩子实现桥接 extern。原 D5 的运行时注入方案作废。
  在 **Context 创建后注入**（不碰生成物）。
- Context 的 2MB 内存块：malloc 自 NuttX 堆（与 Phase 1 相同方式）。

## 实施任务

### Task 2a.1：adapter crate 骨架（0.5 天）
- Cargo.toml/lib.rs/rust-toolchain.toml
- GlobalAlloc 桥（malloc/free）、panic_handler、mqjs_rs_version/mqjs_rs_self_test
- `cargo build --release` 通过（宿主直接构建即可验证编译面）
- **TDD**：宿主单元测试跑不了（no_std staticlib 无测试 harness）——编译期
  验证 + 镜像内验证（Task 2a.3）

### Task 2a.2：Makefile 集成 + 符号审计（0.5 天）
- app Makefile 加 cargo 构建步骤与链接
- setup-sim.sh 加 audit-symbols 子命令（D2）
- 验收：镜像链接成功 + 审计无分配对拆绑

### Task 2a.3：M1-R 端到端验收（0.5 天）
- js_main.c 注入 rsVersion/rsSelfTest
- 镜像启动 → `js /m/cases/rs_probe.js`：
  `assert(rsSelfTest() === 0); console.log(rsVersion());`
- 回归：既有三案例仍 PASS

### Task 2a.4：知识沉淀 + 状态更新（0.25 天）
- gotcha：Rust no_std 静态库进 NuttX sim 的符号绑定清单
- 计划文档 M1-R 标记完成

## 风险与止损

| 风险 | 概率 | 止损 |
|------|------|------|
| R1：compiler_builtins 与 libgcc 符号冲突（memcpy 等） | 中 | 链接参数
  `-Wl,--allow-multiple-definition`（记录）；若仍崩，改用 rlib+精确对象提取 |
| R2：mquickjs-rs 在 no-std 下有隐藏 std 依赖未被 no-std feature 覆盖 | 低 |
  PoC 已实证核心可编译；若遇，按 PoC 的排除清单处理 |
| R3：NuttX malloc 与引擎 2MB 块分配器交互（碎片化） | 低 | Context 块是
  单次大块分配，运行期无增长请求 |
| R4：#[panic_handler] 与镜像内其他 Rust 代码冲突 | 无 | 镜像内 Rust 代码
  只有本 adapter（唯一 #[panic_handler]） |

## 验收清单

- [x] `cargo build --release` adapter 通过
- [x] 镜像链接成功，符号审计无分配对拆绑
- [x] NSH 启动正常，既有 3 案例 PASS（无回归）
- [x] rsSelfTest() === 0（Rust Context 全链路）
- [x] rsVersion() 输出正确（Rust→C→JS 字符串通路）
- [x] 知识条目落地
