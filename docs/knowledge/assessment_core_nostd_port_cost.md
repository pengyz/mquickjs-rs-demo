---
name: assessment-core-nostd-port-cost
description: mquickjs-rs 核心（剔除异步）的 no_std 移植成本评估——PoC 已证明可编译到 thumbv7em
type: architecture
created: 2026-09-04
sources: [/tmp/mqjs-nostd-probe, gotcha_mquickjs_rs_not_bare_metal.md]
---

# 核心 no_std 移植成本评估（含 PoC 实证）

承接 [`gotcha_mquickjs_rs_not_bare_metal.md`](gotcha_mquickjs_rs_not_bare_metal.md)：
E1 已证明整个 crate 无法交叉编译。本文件回答下一个问题 —— **剔除异步后，
核心离裸机有多远。**

## 结论

**可以移植，成本有界。** PoC 已实证：核心（`context` / `env` / `roots` /
`traced` / `handles` / FFI）在 `#![no_std]` + `alloc` 下**零错误编译**到
`thumbv7em-none-eabihf`，产出 ARM EABI5 目标文件。

## PoC 做法与结果

位于 `/tmp/mqjs-nostd-probe`（按仓库规则在**独立副本**中实验，未污染工作区）：

- `include/`：8 个裸机 libc **桩头文件**（`assert/ctype/inttypes/math/setjmp/stdio/stdlib/string`）
- `rust/`：核心源文件 + 宿主生成的 bindings，改造为 `no_std` + `alloc`

```bash
# C 侧：全部 4 个引擎对象编译为 ARM
clang --target=thumbv7em-none-eabihf -c mquickjs.c -o mquickjs_arm.o \
      -I include -I <engine> -O1 -ffreestanding
# → ELF 32-bit LSB relocatable, ARM, EABI5   ✓ 零错误

# Rust 侧
cargo build --target thumbv7em-none-eabihf
# → Finished dev profile   ✓ 零错误，rlib 内为 ARM EABI5 目标文件
```

## 改动量（实测）

**14 个源文件，138 差异行。**

| 文件 | 差异行 |
|---|---|
| `context.rs` | 48 |
| `roots.rs` | 29 |
| `handles/scope.rs` | 15 |
| `handles/global.rs` | 13 |
| `handles/handle_scope.rs` | 13 |
| 其余 7 个文件 | 各 1–5 |

## 改动分类

### 1. 机械路径替换（绝大多数差异行）

`std::X` → `core::X` / `alloc::X`，可脚本化：

| 原 | 新 |
|---|---|
| `std::marker` `std::ptr` `std::mem` `std::cell` `std::pin` `std::fmt` `std::panic` | `core::*` |
| `std::sync::atomic` | `core::sync::atomic` |
| `std::sync::Arc` | `alloc::sync::Arc` |
| `std::ffi::CStr` | `core::ffi::CStr` |
| `std::ffi::CString` | `alloc::ffi::CString` |
| `std::os::raw` | `core::ffi` |
| `std::collections` | `alloc::collections` |

外加每个文件顶部的 `use alloc::{vec::Vec, string::String, boxed::Box, ...};`。

### 2. 结构性改动（真正的"成本"）

| 项 | 处数 | 改法 |
|---|---|---|
| `thread_local!`（`context.rs`、`handles/scope.rs`） | 2 | 换成单线程 `UnsafeCell` 容器 `SingleThreadTls<T>`；**所有 `.with(...)` 调用点无需改动** |
| `Mutex<Vec<Option<Box<JSGCRef>>>>`（`roots.rs`） | 5 行 | 换 `RefCell` —— JS 本身单线程，`Root` 也刻意是 `!Send` |
| `eprintln!`（`context.rs`） | 2 | 需引入日志抽象（PoC 中置空） |
| 异步引用（`context.rs` 的 `async_task_manager`） | ~10 行 | feature-gate 掉（PoC 中给了最小桩） |

### 3. 一个必须注意的构建约束

**bindgen 的布局断言在 32 位目标上必然失败。**

宿主（x86-64）生成的 bindings 里含有：

```rust
["Size of JSGCRef"][size_of::<JSGCRef>() - 16usize];   // ARM 上指针 4 字节 → 8
```

在 `thumbv7em` 上触发 `E0080: attempt to compute 8usize - 16usize, which would overflow`。

⇒ **bindings 必须按目标平台重新生成**（bindgen 需要用目标的头文件与布局）。
这是构建系统改动，不是代码改动。PoC 中移除了这些断言以继续验证。

## 剩余阻塞项（PoC 未覆盖）

| # | 阻塞 | 性质 | 备注 |
|---|---|---|---|
| 1 | bindgen 需按目标生成 | 构建系统 | 见上 |
| 2 | C 侧需 libc 桩 | 已实证可行 | 8 个头 + ~15 个函数（`memcpy`/`memset`/`abort`/`printf` 等；`malloc` 仅字节码路径用） |
| 3 | 异步子系统需剔除 | 代码 | 依赖 `std::thread`/`std::time`/`futures` |
| 4 | `Cargo.toml` 的 `libc` 依赖 | 构建 | 核心不用，但需按平台条件化 |
| 5 | `ridl-builder` 用宿主 triple | 构建系统 | `just nostd-check` 直接调 `mquickjs-build` 绕过；正式集成仍需打通 |
| 6 | **32 位目标需对齐字长与 ROM 表** | 代码+构建 | 见上"为什么是 64 位目标" |
| 7 | `mquickjs_ext_romclass_map.o` 仍由 `cc` 按宿主产出 | 构建 | 交叉链接真实固件时会暴露 |

## 工作量估计（诚实标注为估计，非实测）

- **核心 Rust 移植**：改动量小（138 行）且机械，主要风险在 `thread_local`
  与 `Mutex` 的语义替换是否在所有路径上都成立 —— 需要逐个核对原先依赖
  TLS 的调用路径（三者：`ContextToken::current`、`Scope` 的当前上下文栈、
  以及 re-entrancy 守卫）。**数天量级。**
- **构建系统**（按目标 bindgen + C 交叉构建 + libc 桩）：与代码移植相当。
- **异步重构**（若要保留异步能力）：需要按宿主时基/中断重做，**另计**；
  若接受剔除，则成本为零。

## 固化后的仓库内实现

`no_std` 路径已作为**实验性 feature** 固化，不再是 `/tmp` 里的一次性验证：

```bash
rustup target add aarch64-unknown-none
just nostd-check        # 或直接跑 justfile 里那两条命令
```

包含：

| 组件 | 改动 |
|---|---|
| `deps/mquickjs-rs` | `no-std` feature（与 `std` 互斥，有 `compile_error!` 守卫）；路径统一为 `core::`/`alloc::`；TLS 与 `RootsRegistry` 的锁按 mode 分派；异步子系统整体 cfg 排除 |
| `deps/mquickjs-sys` | 加 `std` / `no-std` feature；仅 `include_dir`/`header_path` 两个构建期辅助函数需要 std |
| `deps/mquickjs-build` | 新增 `--target`：引擎对象改用 `clang --target=<triple> -ffreestanding`；生成器工具仍用宿主编译器 |
| `deps/mquickjs-build/nostd-include/` | 9 个裸机 libc 桩头文件（含 `sys/time.h`） |
| `deps/mquickjs-rs/build.rs` | 交叉时把目标传给 clang，并加 **`-ffreestanding`**；no-std 时用 bindgen `.use_core()` |

验证：`cargo build -p mquickjs-rs --target aarch64-unknown-none --no-default-features --features no-std` **零错误**；std 模式 `cargo test --workspace` 546 个测试全绿（无回归）。

### 为什么是 64 位目标

`mquickjs.h` 的 `JS_PTR64` 是**硬编码**的：

```c
#if INTPTR_MAX >= INT64_MAX
#define JS_PTR64   /* JSValue = uint64_t */
#endif
```

而 ROM 表由生成器按 64 位产出，其中的 `JS_ROM_VALUE(offset)` 展开为
`(JSWord)((uintptr_t)ptr + 1)` —— 32 位目标上把 32 位地址零扩展到 64 位
无法表达为重定位，导致 `initializer element is not a compile-time constant`。

⇒ **32 位目标需要把字长配置（`JS_PTR64`）与 ROM 表生成一并对齐**，
这是一个独立于 Rust 侧的工作项。64 位裸机目标（`aarch64-unknown-none`）
与硬编码字长一致，因此可用于验证 Rust 侧的可移植性。

### 一个隐蔽的坑：裸机目标必须给 clang 传 `-ffreestanding`

不带 `-ffreestanding` 时，clang 在 `--target=aarch64-unknown-none` 下找不到
（也不使用）目标标准头，`inttypes.h` 的 `INTPTR_MAX` 判定退化为 32 位，
于是 `JS_PTR64` **未定义** → `JSValue` 变成 `uint32_t`。

后果不是报错，而是 **bindgen 按错误的字长计算全部结构布局**，生成的断言
与真实布局不符，以 `E0080: index out of bounds` 的形式在编译期爆出。
实测同一头文件：带 `-ffreestanding` 时 `JSValue` 为 8 字节，不带时为 4 字节。

## 对定位的含义

E1 + 本评估合起来给出一个**可验证**的判断：

> "核心运行时"在 **hosted** 目标上成立；在裸机上**尚未**成立，
> 但差距是**有界的、已量化的**，且与对抗性复核建议剔除的部分
> （异步、UI）正好重合。

也就是说：**"嵌入式运行时"目前是口号；把它变成事实的路径已经清楚，
成本可估。** 是否投入是一个定位决策，而不是技术未知。

## 复现

PoC 在 `/tmp/mqjs-nostd-probe`（临时目录，非仓库内容）。
若要在仓库内固化，建议新建 `deps/mquickjs-rs-nostd/` 或加 `no_std` feature，
而不是改动现有 crate 的默认行为。