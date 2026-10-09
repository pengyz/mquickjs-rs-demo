# mquickjs-rs 移植 OpenVela 适配计划（sim 真实案例先行）

> 状态：✅ 已对抗复核 + **Phase 0 侦察完成**（2026-10-08，U1-U3/U6 已实证收敛）  
> 日期：2026-10-08  
> 背景决策：用户（Vela 背景，熟悉小米内部 Rust no_std 路线）确认走 OpenVela 适配；  
> 无真机设备，用 OpenVela sim + QEMU（ISA 模拟）作为验证载体。

## Phase 0 侦察结果（2026-10-08，全部实证）

| # | 结论 |
|---|------|
| U1 ✅ | sim 基线**已构建并冒烟通过**：`repo init -u https://github.com/open-vela/manifests.git -b dev -m openvela.xml` + 部分同步（nuttx/apps/build/vendor/vendor_openvela）→ `./build.sh sim:nsh` → NSH 可交互（help/uname/hello 正常，builtin 注册机制存活）。sim 镜像**动态链接宿主 glibc**（sim-pac 打包 libc.so.6 等） |
| U2 ✅ | **OpenVela 有成体系 Rust 集成**：`apps/cmake/nuttx_add_rust.cmake`（LLVM 参数→nuttx target 映射，支持 **x86_64-unknown-nuttx / i686**——上游没有的自定义 target，规范在 `apps/tools/x86_64-unknown-nuttx.json`：softfloat、panic=abort、rust-lld）+ `apps/tools/Rust.mk`（cargo --target 包装，支持 .json target）+ 示例 `apps/examples/rust/baremetal`（`#![no_std]` + printf FFI + `#[panic_handler]` + `xxx_main` NSH 入口——**与我们核心同构**）。build/envsetup.sh 有 `setup_rust_toolchain`，但引用的 `prebuilts/rust` **不在开源 manifest** → OSS 侧用 rustup nightly + `-Z build-std=core,alloc` 替代（rust-toolchain.toml 锁定） |
| U3 ✅ | M2 板卡实证：`nuttx/boards/arm64/qemu/qemu-armv8a`（NuttX 原生 aarch64 QEMU）+ `vendor/openvela/boards/vela/configs/goldfish-arm64-v8a-ap`（CONFIG_ARCH=arm64）。32 位板维持排除 |
| U6 ✅（sim 部分） | sim 的 libc = NuttX libc（编进镜像）+ 宿主 glibc（动态）双层；C 引擎所需头全部可用；setjmp 实测项保留到 Phase 1（NuttX ostest 自带 setjmp 测试，风险低） |
| U7 | 待 Phase 2 尖刀实验（需先有 sim 内 C 引擎作对照） |
| U8 | sim=hostfs（nuttx sim 自带宿主目录挂载能力） |

**Rust no_std vs 非 no_std 裁定**（用户问题的正式回答）：**走 no_std**。
- 平台官方路径即 no_std 裸金属（U2 实证）；std on nuttx target 需 build-std=std 且 libc 兼容面大，官方文档声明 no_std
- sim 的双层 libc 结构下，std 构建会直接绑宿主 glibc，绕过 NuttX 语义——不是"NuttX 应用"
- 我们的 FFI 架构（C 引擎执行面 + Rust 管理面）与 no_std 模型天然匹配；Rust 侧浮点极轻（softfloat target 可接受，浮点重活在 C 引擎）
- RIDL stdlib 的 std 绑定是 Phase 2b 已知工作项，不改变核心裁定

## 目标与总验收（复核修订版）

- **M1-C（sim，纯 C）**：`nsh> js run_cases.js` 在 OpenVela sim 中运行选语料，
  纯 C 引擎 + C stdlib print，stdout 按**哨兵行协议**输出
  `CASE <n> PASS` / `CASE <n> FAIL: <msg>`（NSH 无 `$?`，退出码不可脚本化），
  宿主侧 expect 扫描全绿。含一个故意语法错误的用例验证 setjmp/longjmp 路径。
- **M1-R（sim，Rust no_std）**：mquickjs-rs 核心（no-std feature）以
  staticlib 链入 sim，手写 C singleton 注册；RIDL console 视 2b 进度。
- **M2（QEMU，aarch64）**：同一栈跑在 aarch64 参考板（goldfish-arm64-v8a-ap
  或 NuttX qemu-armv8），交叉编译全通。
- 真机板级适配不在本计划范围。

## 事实基础（复核后修正）

1. Rust 上游 `*-nuttx-*` 全为 Tier-3 no_std（[rustc platform-support](https://doc.rust-lang.org/rustc/platform-support.html)）：
   **无 x86_64** —— tier-3 路线只适用 M2 的 arm/riscv 板（nightly +
   `-Z build-std=core,alloc`）；sim 下 Rust 只有两条真实路径：
   - **主线 (a)**：`x86_64-unknown-linux-gnu` + no-std feature + `staticlib`
     （panic=abort、`#[global_allocator]` 桥 malloc）——无需 nightly；
   - **对照臂 (c)**：宿主 std 默认 feature 构建。仅用于 M1 语料铺量与
     符号解析探针，**不作为验收形态**（其 PASS 对 M2 无迁移价值，且有
     libc 符号割脑风险）。
2. **RIDL 全链当前硬绑 std（复核实证，U5 已定论）**：
   `ridl-extensions ⇒ std`（mquickjs-rs/Cargo.toml:32）；生成胶水产
   `std::ffi::CString`/`std::collections::HashMap`（filters.rs:106,1913,1925）；
   stdlib_impl 用 `println!`。→ **RIDL 去 std 是工具链改造**（feature 解绑 +
   生成器三类替换 + stdlib_impl 输出层 printf FFI），单列 Phase 2b 独立估价，
   不是编译实验。
3. mquickjs C 依赖面小且全部落在 NuttX libc 提供范围（stdlib/stdio/stdarg/
   inttypes/string/assert/math/setjmp/ctype）；**setjmp/longjmp 是关键路径**
   （`JS_Eval` 语法错误经 longjmp 展开，mquickjs.c:7549 等）——选板上必须实测。
   Rust 侧 bindgen 保留 nostd stub 头（纯声明无害），C 侧用 NuttX 头。
4. NuttX sim = 宿主 x86 Linux 进程，`configure.sh sim:nsh && make`；
   builtin 注册经 Kconfig + PROGNAME/MAINSRC → builtin_list.h。

## 集成边界裁定（用户问题：C 层与 Rust 层的工具链互斥吗？）

**不互斥——base/ridl 归档拆分就是这个边界的既有架构**（知识库
`architecture_mquickjs_base_vs_ridl_outputs.md`）：

| 归档 | 内容 | 用户 |
|------|------|------|
| `libmquickjs_core.a` | 变体无关引擎对象（两变体 md5 相同） | 共享 |
| `libmquickjs_stdlib_base.a` | C 层 stdlib（引擎原生：console/print C 实现） | **C 层用户** |
| `libmquickjs_stdlib_ridl.a` | RIDL 扩展 stdlib（js_c_function_table → Rust 胶水） | **Rust 用户** |
| `libmquickjs.a` | 合并归档（外部 consumer 向后兼容） | — |

- **C 层用户**：引擎原生工具链（mquickjs_build/Makefile 流程）→ base 变体 +
  `mquickjs.h` 公共 API。是被支持且被测试的路径（mquickjs-rs 自身测试固定链
  base；`tests/base_build_no_ridl_headers.rs` 保证 base 构建无 RIDL 头）。
  我们的 mquickjs-build 实为引擎自有工具（mqjs_stdlib/build_atoms）的构建编排
  ——产出的 base 变体就是引擎原生产物，不是另一套语义。
- **Rust 用户**：ridl-builder pipeline → ridl 变体 + mquickjs-rs crate + 胶水。
- **共存规则**：一个叶子二进制恰好链一个 stdlib 变体（base XOR ridl）；base
  符号导出为 **weak**、ridl 为 strong——同时可见也不 duplicate symbol。
- **允许 C 层使用引擎：是**，且 openvela 移植里它是默认卖点（C 开发者拿 base
  产物；Rust 开发者拿全套）。Phase 1 = base 变体 + `js` builtin；Phase 2 =
  core + ridl + Rust。C 层契约在 Phase 2 之后不变。

## 关键未知（Phase 0 收敛）

| # | 未知 | 收敛方式 |
|---|------|---------|
| U1 | OpenVela sim 基线：**术语二选一**——openvela fork 内 NuttX 原生 `boards/sim`（轻）vs openvela 官方"模拟器"= goldfish AP 镜像（重）。M1 用前者 | 实跑空 nsh |
| U2 | OpenVela Rust 集成现状（build 包装/示例有无） | grep 仓库树 |
| U3 | QEMU 决策矩阵：**主候选 aarch64**（goldfish-arm64-v8a-ap / qemu-armv8）；**排除 32 位板**（JS_PTR32 + ROM 表对齐是独立工作项，评估文档 :150-153 明确无界） | 文档+仓库盘点 |
| U4 | （已收敛，见事实基础 1：主线 (a) + 对照臂 (c)） | — |
| U5 | （已定论，见事实基础 2：RIDL 链 = std 绑定） | — |
| U6 | NuttX libc 对 9 个 nostd stub 头的覆盖对照 | 逐头核对 |
| **U7** | **引擎对象双重链接**：mquickjs-rs/build.rs:31 无条件 `link-lib=static=mquickjs_core`，与 apps Makefile 编译的同一批 C 文件在最终镜像里符号冲突 → 需提供跳过链接的开关（如 `MQJS_ENGINE_LINK=external`，bindgen-only） | 最小符号实验（尖刀实验 i） |
| **U8** | 脚本进目标 FS 的通道：sim=hostfs 挂宿主目录；goldfish=romfs/data 分区 | Phase 1 机械项 |

## 尖刀实验（Phase 0 内，各 0.5 天，先于 Phase 2 承诺）

- **尖刀 i**：现有 demo 以 std 默认 feature 产 staticlib，链入空 sim 镜像跑
  hello——收敛 U7 的符号解析规则（镜像内 NuttX libc 对象 vs 宿主 glibc
  动态符号谁抢占）。
- **尖刀 ii**：单份生成 glue.rs 加 `#![no_std]` 试编——量化 2b 的真实改造面。

## 适配层落位

```
ports/openvela/
├── README.md                 # 实施态文档（含哨兵行协议、构建记录）
├── app/
│   ├── Kconfig               # CONFIG_MQJS_JS
│   ├── Makefile              # 显式 STACKSIZE ≥64KB 起调；JS 堆块显式供给
│   │                         # （JS_NewContext 内存不足**静默**不报错，
│   │                         #  mquickjs.h:281-289——必须自检堆大小）
│   └── js_main.c             # NSH 入口：读脚本 → eval → 哨兵行汇总
├── rust/
│   ├── Cargo.toml            # 适配 crate：GlobalAlloc 桥 + panic_handler
│   │                         # + MQJS_ENGINE_LINK 开关 + 导出面
│   └── src/lib.rs
└── qemu/                     # M2：aarch64 板级配置（Phase 0 选型后补）
```

- openvela 本体 clone 到并行目录 `~/workspace/openvela/`（repo 多仓 +
  prebuilts，官方要求 **≥80GB 磁盘 / ≥16GB RAM**，sync 小时级）；
  本仓库只进 ports/ 子树；ports/ 与 openvela apps 树的接线方式
  （拷贝/软链/CONFIG_APPS_DIR 外挂）Phase 0 定。
- 工具链全部按 openvela prebuilts 验证（不自带 gcc）；Rust 侧
  rust-toolchain.toml 锁定 nightly 版本。
- 语料选择标准（RIDL 依赖面升序）：(i) 零 RIDL 依赖（literals/constants 类，
  Phase 1 可跑）；(ii) 仅 console；(iii) singleton 调用。**排除** async_test
  全部 4 个（no_std 结构性排除）与 map 参数用例（拉进 HashMap 胶水）。
  用例按"crate 移植"计工作量（RIDL crate 编译 + singleton 注册 + 按目标
  重生成），不按 JS 文件数计；现网计数口径修正为 tests/global 20 +
  module 9 + stdlib 3 = 32 个 JS 文件。

## 分阶段实施

### Phase 0：侦察与环境基线（**1.5-2 天**，复核修正）
U1-U8 收敛 + 两个尖刀实验；产出：sim 空镜像构建记录、Rust 现状结论、
QEMU 决策矩阵、libc 对照表、接线方式结论。
**任一被证伪 → 停下重议路径。**

### Phase 1：C 引擎上 sim → M1-C（1-2 天）【✅ 已完成 2026-10-08】

**集成方式（独立复核调研结论：源码级集成，弃用预构建归档链接）**：

openvela 对"JS 引擎作为 NuttX 应用"的官方参照是
`apps/interpreters/quickjs`：**引擎 .c 源文件进 apps 构建，用 NuttX 工具链
编译**（CSRCS + CFLAGS 平台适配）。引擎原生工具链的角色 = **宿主代码生成**
（`mqjs_stdlib` host 工具 → `mquickjs_atom.h` + `mqjs_stdlib.h`，默认 flags
含 console/print——框架 base 变体是无 console 的最小面，那是 RIDL 应用的
形态，不是 C 层契约）。

- app Makefile：`CSRCS = js_main.c mquickjs.c cutils.c dtoa.c libm.c
  mqjs_stdlib.c`，CFLAGS 指向生成头目录 + 引擎源目录；所有 libc 绑定
  一致落在 NuttX 侧（setjmp/longjmp、malloc/free 无混绑可能）。
- setup-sim.sh 增加 host 工具链步骤：host cc 编 `mqjs_stdlib.host.o +
  mquickjs_build.host.o` → `mqjs_stdlib` → 生成头到 `ports/openvela/app/gen/`
  （gitignore；引擎原生工具，默认 flags）。
- **弃用**：glibc 预构建归档链接 + objcopy 符号重定向（混绑面无法穷尽审计：
  任何 libc 符号对分属 NuttX/glibc 即潜在损坏；setjmp 只是第一个爆的）。
  framework base 产物仍存在，但只服务 Rust/Ph2 与 host 侧，不进 sim 链接。
- js builtin：Kconfig/Makefile/Make.defs（Make.defs 是 CONFIGURED_APPS 注册
  的必要文件）+ js_main.c（哨兵行协议、JS 堆显式供给与自检、hostfs 读脚本）。
- 验收（M1-C）：`nsh> js run_cases.js` 全绿 + 语法错误用例走通
  setjmp/longjmp 不崩 + 哨兵行被宿主扫描确认。语料 = assert-only
  （console 属 RIDL 层，2b 才有）。

**实施纪要（2026-10-08 调试实证，支撑上述裁定）**：预构建 glibc 归档链入
sim 镜像后，引擎 UND 的 `_setjmp` 落 glibc 动态、`longjmp` 被 NuttX 静态
libc.a 抢占（=NXlongjmp）——混绑导致首个语法错误即 SIGSEGV（gdb 实锤崩在
NXlongjmp 的 `jmp *JB_RIP`，RIP 为垃圾）。 曾尝试 objcopy 重定向
`longjmp→siglongjmp`，属单点补丁（libc 符号对无法穷尽审计）——已弃用。

### Phase 1.5：CMake 轨调查【⏸ 推迟到 M2（2026-10-09 用户裁定）】

调查结论（多次构建实证）：
- openvela 的 CMake 流程从设计上要求配置位于树内：BOARD_CONFIG 解析、
  `_is_makefile` 分流、`nuttx_custom_module.cmake` 无条件 include
  `apps/external/optee/TA*.cmake`（来自部分同步拉不到的仓库）
- js builtin 已完成 CMake 轨适配物：`app/CMakeLists.txt`
  （nuttx_add_application + INCLUDE_DIRECTORIES/COMPILE_FLAGS）
- **残留阻塞**：双轨互斥（Make 产物与 cmake configure 互斥，切换需
  distclean）、外部 BOARD_CONFIG 路径的树内校验、romfs etc 目录依赖
  configure 时序
- **M2 处置**：goldfish 完整树 + 官方 CMake 流程下重新验证
  （部分同步的残缺树不代表 M2 真实环境）

### Phase 2：Rust 进 sim（2a：1-2 天；2b：3-5 天，独立验收）
- **2a**：主线 (a) 形态链入 no-std 核心 + 手写 C singleton；GlobalAlloc 桥；
  符号面 = memcpy/memset/memcmp/malloc/free + FFI 导出（备
  `--allow-multiple-definition` 应对 compiler_builtins 撞 libgcc）。
  验收 = M1-R。
- **2b（独立特性）**：RIDL 去 std 三件套（feature 解绑 / 生成器
  CString/HashMap/print 替换 / stdlib_impl 输出层 printf FFI）。
  验收：RIDL console 在 no-std 配置下可用；现网全量测试无回归。
  对照臂 (c) 仅用于铺量与符号探针，不作为验收。

### Phase 2a+2b 实施记录【✅ 2a 完成（b531dd7）；2b/M1-R+ 完成 2026-10-09】

**形态裁定演进**：2a 实施时 sim 确认为宿主 Linux 进程（动态链 glibc），
no_std 变体（GlobalAlloc 桥/panic_handler）整体让位 **std 模式**
（变体保留在 git 历史 d945524）；2b 相应不再是"去 std 三件套"，而是
**把 RIDL stdlib（console singleton）经既有聚合管线接入 sim 镜像**。

**2b 集成设计（M1-R+）**：
- **镜像 stdlib 切换为 ridl 变体**：运行时 TU 换成 deps/mquickjs-rs 的
  `mqjs_stdlib_impl.c`（strong `js_stdlib`，Date 自带）+ `require.c` +
  sim 聚合的 `mquickjs_ridl_register.c`（`JS_RIDL_StdlibInit` + require
  表）；宿主工具改从 `mqjs_stdlib_template.c` 构建（模板契约：宿主层
  对象 console/print/timers/load 全部由 RIDL 扩展提供，模板不保留 C 残面）。
  base `mqjs_stdlib.c` 退出镜像。
- **sim 应用 = 独立 RIDL 叶子**：`ports/openvela/rust` 挂
  `mquickjs.ridl.toml`，模块选择仅 `[dependencies.stdlib]`（console），
  `ridl-builder aggregate --cargo-toml ... --intent build` 出迷你聚合
  （slot 0 = console，无 user 类，无 module 模式模块 → 无 romclass 映射
  依赖）。**不复用 mquickjs_demo 聚合**——那会把全部测试模块（19 crate、
  ~150 js_* 符号）拖进 sim 镜像并与测试语料目录耦合；聚合是生成物而非
  "基础设施"，为 sim 重建只需一条命令。
- **Context 创建在 Rust 侧（关键约束实证）**：RIDL 胶水经 `JSContext`
  user_data（`Arc<ContextInner>`）分派，而 user_data 只由
  `mquickjs_rs::Context::new` 安装——C 侧 `JS_NewContext` 建的 context
  必然抛 "missing ctx user_data"，且 deps/mquickjs 不可改。故 js_main.c
  改用 adapter 三件套 `mqjs_rs_ridl_context_new / mqjs_rs_ridl_eval /
  mqjs_rs_ridl_context_free`（engine 堆块移交 Rust 持有，C 侧不再
  malloc JS 堆）；进程级 `ridl_bootstrap!`（模块/符号 keepalive）在
  adapter 内 Once 化。原设计倾向中的 "C API + ridl_context_init 导出"
  被该约束否决，"Rust 侧封装 eval" 成为唯一可行路径。
- **rs 探针保留**：overlay 从 base stdlib 搬到 staged 模板
  （锚点 `JS_PROP_NULL_DEF("globalThis", 0 ),`）；生成表跨 TU 引用钩子，
  `js_rs_version/js_rs_self_test` 改非 static，声明经生成的
  `gen/mqjs_rs_hooks.h` 强制包含进每个 app TU。
- 符号审计扩展：镜像级新增 `js_global_singleton_console_log` 落地检查；
  镜像检查在镜像落后于 adapter 归档时自动跳过（build 序 = stage →
  make → audit）。

**验收记录（2026-10-09，sim Make 轨）**：
- `ridl_console.js`：输出 "ridl console in openvela sim"（Rust
  `println!` 通路），CASE PASS ✅
- 无回归：demo_pass PASS、rs_probe PASS（其 console.log 亦改走 RIDL）、
  tiny_err / demo_syntax_error → SyntaxError 文本不崩 ✅
- audit-symbols PASS（分配对 OK；malloc/free/calloc/realloc 绑定一致
  host；`js_stdlib` 与 `js_global_singleton_console_log` 均在镜像落地）✅
- 镜像内 `js_print` 0 命中（C print 钩子确已移除，console 归属 RIDL）✅
- 本仓库 demo 语料 31/31 PASS（root workspace 未触碰，环境无回归）✅


### Phase 1.5：CMake 轨集成【✅ 真实验收通过 2026-10-09 下午】（原计划 M2 时补，用户裁定提前）

> 更正：本节此前一次"已完成"记录为假阳性——镜像能产出但 `js` builtin
> 并未注册（CMake GLOB 不穿透符号链接），当日复核予以推翻并重做。

- app `CMakeLists.txt`：`nuttx_add_application` 注册；force-include 用调用后
  `target_compile_options(... "SHELL:-include x.h")`（COMPILE_FLAGS 列表
  逐元素为一个 argv，多词 flag 会被引号合并成"含空格文件名"）；adapter
  归档经 `nuttx_add_extra_library()` 进链接（extras 排在 apps 库之后，
  sim 链接整体包 `--start-group`，交叉引用自动闭合）
- setup 脚本：① app 接入点由符号链接改为 `cp -a` 真实目录拷贝（staging
  与头生成之后同步；GLOB 不穿透符号链接而 Make wildcard 可穿透）；
  ② 合成 defconfig 落**树内** `boards/sim/sim/sim/configs/mqjs`（纯新增），
  树外绝对路径会打断 `NUTTX_BOARD_ABS_DIR/../..` 板级回溯（etc romfs
  规则消失）；③ `VELA_BUILD_BOARD_CONFIG` 必须在 **lunch 之后**导出
  （lunch 无条件用原始参数覆盖它）；④ 构建走官方 `lunch → m`（configure
  在 `_build_board` 内，不在 lunch 里）
- 引擎/生成器清理（Vela cmake 全局 `-Werror` 暴露）：mquickjs.c 11 处
  shadow/unused 局部重命名（`JS_PUSH_VALUE` 宏 token 拼接 `v##_ref`，
  外层 JSGCRef 槽位是承重的，内层重复声明才可删）；libm.c 删无效
  `#define NDEBUG`（命令行已定义且文件无 assert）；ridl-tool 空表门控
  （proto_vars 为 0 不再生成未引用的 `ridl_proto_var_entries`，TDD：
  proto_var_empty_table_test，424→426）
- 验收（cmake_out/sim_nsh 镜像实机）：`js` 命令存在；demo_pass PASS、
  rs_probe PASS（rust-bridge ok）、ridl_console PASS（Rust 控制台全链路）、
  tiny_err SyntaxError 文本上报不崩——与 Make 轨哨兵一致；回归：workspace
  616/0、JS 语料 31/31、ridl-tool 426/0
- 细节坑已沉淀 `docs/knowledge/gotcha_vela_cmake_track.md`

### Phase 3：QEMU aarch64 → M2（2-4 天）
- 按 U3 选型迁移；Rust 侧换 `aarch64-unknown-nuttx` + `-Z build-std`；
  工具链走 openvela prebuilts；ROM/胶水按目标重生成。
- 验收：M1 语料在 QEMU 上哨兵全绿；字长项已在选型时规避（aarch64
  JS_PTR64 天然一致）。

## 总估算（复核修正）：7.5-13 天（aarch64 口径；32 位板不在内，无界）

## 风险与止损

- R1：openvela sim 基线宿主不可构建 → 退 NuttX 上游 sim 验证技术路。
- R2：no-std 核心链接失败（CRT/entry/符号）→ 交付 M1-C；Rust 侧回 PoC
  rlib 形态出评估报告。
- R3：2b 改造超预期 → M1-R 降级为"no-std 核心 + 手写 singleton"已可交付，
  2b 独立排期。
- R4：工具链版本漂移 → rust-toolchain.toml + prebuilts 锁定。
- R5（复核新增）：sim 与 QEMU 行为差异（字长一致但 libc/对齐细节不同）
  → 每阶段哨兵协议复跑，禁止"sim 过 = QEMU 过"的推定。

## 明确不做（本轮）

- 32 位板（JS_PTR32 + ROM 表对齐需先立项）
- 异步子系统上 RTOS（维持 host-only）
- RIDL stdlib 全量上 sim（2b 只做 console 最小面）
- openvela Rust 集成的通用化贡献（先把自家通路打通，回馈上游另议）
