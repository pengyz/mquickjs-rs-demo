---
name: gotcha-mquickjs-rs-is-not-bare-metal-capable
description: mquickjs-rs 无法编译到裸机 target；阻塞项集中在异步子系统，RIDL/GC/handles 核心本身是 no_std 干净的
type: gotcha
created: 2026-09-04
sources: [cargo build -p mquickjs-rs --target thumbv7em-none-eabihf, deps/mquickjs-rs/Cargo.toml, deps/mquickjs-rs/src]
---

# mquickjs-rs **不能**编译到裸机 target（"核心运行时已完成"仅对 hosted 成立）

## 实验（E1：交叉编译门禁）

```bash
rustup target add thumbv7em-none-eabihf
cargo build -p mquickjs-rs --target thumbv7em-none-eabihf
```

**结果：构建立即失败**，且失败点不在本仓库代码，而在依赖：

```
error[E0463]: can't find crate for `std`
  --> futures-core-0.3.34/src/lib.rs:16:1   extern crate std;
  --> memchr-2.7.6/src/lib.rs:198:1         extern crate std;
  --> futures-sink/src/lib.rs
  = note: the `thumbv7em-none-eabihf` target may not support the standard library
```

## 硬阻塞项清单

| 类别 | 出现次数 | 所在文件 | 裸机问题 |
|---|---|---|---|
| `std::thread` | 20 | `async_task.rs`, `async_bridge.rs` | 裸机无线程；整个 worker 线程模型不存在 |
| `std::time` | 3 | `async_task.rs`, `async_bridge.rs` | 需要嵌入式时基 |
| `std::sync::Mutex` | 24 | `async_task.rs`, `context.rs`, `async_bridge.rs`, `roots.rs`, `handles/scope.rs`, `handles/global.rs`, `async_stream.rs` | 需 `spin`/临界区替代 |
| `std::collections::HashMap` | 3 | `async_task.rs`, `async_value.rs` | 需 alloc + hashbrown/BTreeMap |
| `std::ffi::CString` | 22 | 多处 | `alloc::ffi::CString`（需 alloc） |
| 依赖 `futures` | — | `Cargo.toml` | `futures` 需要 std |
| 依赖 `libc` | — | `Cargo.toml` | 裸机无 libc |

统计：`deps/mquickjs-rs/src` 共 30 个 `.rs`，其中 18 个使用 `std::`。

## 关键区分：哪些部分是 no_std 干净的

**`traced.rs` 以及大部分 `handles/*` 完全没有 `std::` 依赖**：

```
ridl_runtime.rs  mod.rs  ridl_include.rs  ridl_modules.rs  ridl_ext_access.rs
traced.rs  handles/{any,mod,array,array_push_pop,handle_scope_tests,array_tests}.rs
```

即：**RIDL / GC / handles 核心本身并不依赖 std**；真正把 crate 绑死在 hosted
环境上的是**异步子系统**与少量基础设施（Mutex / HashMap / CString / libc）。

这与先前的对抗性复核结论一致：异步子系统是"高风险白干"——其线程模型在裸机上
整体作废。

## 对定位的含义

1. **"核心运行时已完成"这句话，只在 hosted（std）目标上成立。**
   若项目定位含"嵌入式"，那么在最基本的"能否交叉编译"这一关上尚未通过。
2. 但**并非全盘皆输**：enclave 的 RIDL/GC/handles 核心是干净可移植的。
   若把异步与 UI 从叙事中剔除（复核已建议如此），剩下的那部分
   **离 no_std 化并不远**。
3. 若要真正支持裸机，需要的最小改造集：
   - 去掉 `futures` 依赖（或换 `futures-core` + 自带 executor）
   - `std::sync::Mutex` → `spin::Mutex` 或临界区
   - `std::collections::HashMap` → `alloc` + hashbrown，或 BTreeMap
   - `std::ffi::CString` → `alloc::ffi::CString`
   - 异步子系统：要么砍掉，要么按宿主时基/中断重做（无 `thread::spawn`）
   - `libc` 依赖需按平台条件化

## 未验证项

- 未尝试实际改造，因此**改造成本是估算而非实测**。
- `mquickjs-sys` 侧：C 引擎本身是 freestanding C，但其 `build.rs` 依赖按
  host triple 产出的构建产物（`ridl-builder` 的 `resolve_target_triple` 用 host），
  交叉编译的编排路径尚未打通——这是独立于 Rust 侧 std 问题的第二个阻塞。

## 复现

```bash
rustup target add thumbv7em-none-eabihf
cargo build -p mquickjs-rs --target thumbv7em-none-eabihf   # 期望失败
```