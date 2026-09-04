# mquickjs-rs

**RIDL 工具链 + mquickjs 嵌入式 JavaScript 引擎的 Rust 集成**

[![Tests](https://img.shields.io/badge/tests-310+-brightgreen)](#testing)
[![Coverage](https://img.shields.io/badge/coverage-100%25-brightgreen)](#testing)

## 项目定位

mquickjs-rs 是一个**生产就绪的工具链**，用于在 Rust 应用中嵌入 [mquickjs](https://github.com/bellard/mquickjs)（Micro QuickJS）JavaScript 引擎，通过 RIDL（Rust Interface Definition Language）实现类型安全的 Rust/JS 双向互操作。

### 核心价值

- **类型安全**：RIDL 生成的代码在编译时检查类型，避免运行时 FFI 错误
- **零手写 FFI**：自动生成 Rust trait、C 头文件、glue 代码
- **GC 集成**：Root<T>/Traced<T> 系统确保 JS 对象在 Rust 引用期间不被回收
- **嵌入式友好**：mquickjs 专为资源受限环境设计（10kB RAM、100kB ROM）

### 适用场景

- 嵌入式/IoT 设备运行 JavaScript 插件
- Rust 应用需要可扩展的脚本引擎
- 游戏引擎的脚本系统
- 边缘计算的轻量级 JS 运行时

## 架构

```
┌─────────────────────────────────────────────────────────────┐
│                    Application Layer                         │
│  (Your Rust app + RIDL modules + JS scripts)                │
├─────────────────────────────────────────────────────────────┤
│                    RIDL Tool Chain                           │
│  ┌──────────┐  ┌──────────────┐  ┌──────────────────────┐  │
│  │  Parser   │→│  Code Gen    │→│  Templates (.j2)      │  │
│  │ (pest)    │  │ (Rust/C/JS)  │  │  (Askama)            │  │
│  └──────────┘  └──────────────┘  └──────────────────────┘  │
├─────────────────────────────────────────────────────────────┤
│                    mquickjs-rs                               │
│  ┌──────────┐  ┌──────────────┐  ┌──────────────────────┐  │
│  │ Context   │  │ Root<T>      │  │ Traced<T>            │  │
│  │ (JS ctx)  │  │ (GC roots)   │  │ (GC traced fields)   │  │
│  └──────────┘  └──────────────┘  └──────────────────────┘  │
├─────────────────────────────────────────────────────────────┤
│                    mquickjs (C engine)                       │
│  Tracing GC · ES5 subset · ROM classes · 10kB RAM           │
└─────────────────────────────────────────────────────────────┘
```

## 快速开始

### 1. 构建

```bash
# 准备：构建工具 + 生成 RIDL 聚合 + 构建 QuickJS base/ridl 输出
cargo run -p ridl-builder -- prepare

# 构建应用
cargo build
```

### 2. 定义 RIDL 接口

```typescript
// src/my_module.ridl
module my.app@1.0.0;

import { Helper } from "helper_lib";

class Calculator {
    opaque {
        history: array<Traced<Value>>
    }

    fn add(a: i32, b: i32) -> i32;
    fn getHistory() -> array<i32>;
}

singleton MathService {
    fn initialize() -> void;
    readonly property version: string;
}
```

### 3. 实现 Rust 侧

```rust
// src/calculator_impl.rs
use crate::api::CalculatorClass;

pub struct DefaultCalculator {
    history: Vec<mquickjs_rs::Traced<mquickjs_rs::Value>>,
}

impl CalculatorClass for DefaultCalculator {
    fn add(&mut self, a: i32, b: i32) -> i32 {
        let result = a + b;
        // 记录到历史...
        result
    }

    fn get_history(&mut self) -> Vec<i32> {
        // 返回历史记录...
        vec![]
    }

    fn gc_mark(&self, mf: *const mquickjs_rs::mquickjs_ffi::JSMarkFunc) {
        // 标记 Traced 字段
        for item in &self.history {
            unsafe { item.gc_mark(mf) };
        }
    }
}
```

### 4. 运行 JavaScript

```javascript
// scripts/main.js
const calc = new Calculator();
console.log(calc.add(2, 3)); // 5
console.log(calc.getHistory()); // [5]
```

## RIDL 语法参考

### 类型系统

| RIDL 类型 | Rust 类型 | 说明 |
|---|---|---|
| `i32`, `i64`, `f32`, `f64` | `i32`, `i64`, `f32`, `f64` | 数值类型 |
| `string` | `String` | 字符串 |
| `bool` | `bool` | 布尔值 |
| `void` | `()` | 无返回值 |
| `object` | `JSValue` | 任意 JS 对象 |
| `any` | `JSValue` | 任意类型 |
| `array<T>` | `Vec<T>` | 数组 |
| `map<K, V>` | `HashMap<K, V>` | 字典 |
| `T?` | `Option<T>` | 可选值 |
| `T \| U` | Enum | 联合类型 |
| `Traced<T>` | `Traced<T>` | GC 追踪字段 |

### 定义类型

```typescript
// 类
class MyClass {
    const MAX: i32 = 100;           // 常量
    var count: i32 = 0;             // 变量
    proto var state: i32 = 0;       // 原型变量
    readonly property name: string; // 只读属性
    property value: i32;            // 读写属性
    constructor(x: i32);            // 构造函数
    fn doSomething() -> void;       // 方法

    opaque {                        // GC 追踪字段
        held: Traced<Value>
        items: array<Traced<Value>>
    }
}

// 单例
singleton MyService {
    fn initialize() -> void;
    readonly property version: string;
}

// 接口
interface Drawable {
    fn draw(ctx: object) -> void;
}

// 枚举
enum Color { RED, GREEN, BLUE }

// 结构体
json struct Config {
    name: string;
    value: i32;
}

// 回调
callback EventHandler(event: object);

// 类型别名
using StringMap = map<string, string>;
```

### 导入

```typescript
import { Foo, Bar } from "my_lib";
import { Helper as H } from "utils";
import * as Utils from "utils";
```

## GC 系统

### Root<T> — 跨调用持有 JS 值

```rust
let token = ctx.token();
let scope = token.enter_scope();
let val = ctx.eval("42").unwrap();
let root = Root::new(&scope, val);

// root 保持 JS 值存活，即使 JS 侧不再引用
ctx.eval("globalThis.val = null").unwrap();
gc(&mut ctx); // val 不会被回收

drop(root); // 释放 root，下次 GC 可以回收
```

### Traced<T> — RIDL 类内部字段

```rust
pub struct MyNode {
    held: Option<Traced<Value>>, // 自动参与 GC 标记
}

impl MyNodeClass for MyNode {
    fn gc_mark(&self, mf: *const JSMarkFunc) {
        if let Some(ref held) = self.held {
            unsafe { held.gc_mark(mf) };
        }
    }
}
```

### 平台限制

> **重要**：mquickjs 的 GC sweep 释放 JS 对象但**不调用 finalizer**。Finalizer 只在 `JS_FreeContext`（context 销毁）时运行。这是嵌入式引擎的设计选择。
>
> 影响：GC sweep 后 native opaque（Box）泄漏到 context teardown。对于短生命周期 context（创建→执行→销毁），这是可接受的。

## AsyncStream — 异步事件流

AsyncStream 提供了类型安全的异步事件流机制，用于管理异步回调的生命周期。

### 基本用法

```rust
use mquickjs_rs::async_stream::AsyncStream;
use mquickjs_rs::Root;

// 创建事件流
let mut stream = AsyncStream::<i32>::new();

// 订阅事件
let callback = ctx.eval_jsvalue("(function(v) { console.log(v); })").unwrap();
let cb_root = Root::new(&scope, scope.value(callback).try_into_function(&scope).unwrap());
let sub = unsafe { stream.subscribe(cb_root) };

// 发射事件
unsafe { stream.emit(&scope, &42); }

// 取消订阅（自动清理）
sub.unsubscribe(&mut stream);
```

### 错误处理

```rust
use mquickjs_rs::async_error::AsyncError;

// 发射错误事件
let error = AsyncError::Message("something went wrong".to_string());
unsafe { stream.emit_error(&scope, &error); }
```

### 线程安全

```rust
use mquickjs_rs::async_stream::{ThreadSafeEventQueue, EventCompletion};
use std::sync::Arc;

// 创建线程安全队列
let queue = Arc::new(ThreadSafeEventQueue::<String>::new());

// 在 Worker 线程中推送事件
let queue_clone = queue.clone();
std::thread::spawn(move || {
    queue_clone.push(EventCompletion {
        stream_id: 1,
        value: "data from worker".to_string(),
    });
});

// 在 JS 主线程中处理事件
let events = queue.drain();
for event in events {
    // 处理事件
}
```

### 生命周期管理

- **Subscription Drop**：自动从 AsyncStream 中移除 subscriber
- **AsyncStream Drop**：清理所有 subscriber
- **Weak 引用**：避免循环引用导致内存泄漏

## 测试

```bash
# 运行所有测试
cargo test -p ridl-tool          # RIDL 工具链测试（310+ 个）
cargo test -p mquickjs-demo      # 应用测试
cargo run -- tests               # JS 集成测试（19 个）

# 运行特定测试
cargo test -p ridl-tool --test comprehensive_syntax_test  # 语法覆盖测试
cargo test -p ridl-tool --test end_to_end_codegen_test    # 端到端代码生成测试
```

### 测试覆盖

| 组件 | 测试数 | 覆盖率 |
|---|---|---|
| RIDL 解析器 | 266 | 100% 语法规则 |
| 代码生成器 | 44+ | 核心功能 |
| GC 系统 | 8 | 核心场景 |
| JS 集成 | 19 | 全部通过 |
| **总计** | **310+** | — |

## 知识库

项目知识沉淀在 `docs/knowledge/`：

- **Architecture**：设计决策、模块边界
- **Gotchas**：平台坑、反直觉行为
- **Patterns**：代码约定、开发模式
- **Decisions**：已验证的取舍
- **References**：外部资源指针

使用 `/post-commit-memory` 在 commit 后沉淀知识，`/knowledge-dream` 定期整理。

## 项目结构

```
mquickjs-rs-demo/
├── deps/
│   ├── mquickjs/          # mquickjs C 引擎（git submodule）
│   ├── mquickjs-rs/       # Rust 绑定 + Root<T>/Traced<T>
│   ├── mquickjs-sys/      # FFI 绑定
│   └── ridl-tool/         # RIDL 解析器 + 代码生成器
├── ridl-builder/          # 构建编排器
├── ridl-modules/          # 标准库模块
├── tests/                 # 测试 RIDL 模块
├── docs/
│   ├── knowledge/         # 项目知识库（20 个条目）
│   └── planning/          # 规划文档
├── AGENTS.md              # AI 协作规则
└── Cargo.toml
```

## 相关链接

- [mquickjs](https://github.com/bellard/mquickjs) — Micro QuickJS 引擎
- [QuickJS](https://bellard.org/quickjs/) — 原始 QuickJS 引擎
- [RIDL 语法规范](docs/ridl-spec.md) — 完整的 RIDL 语言规范

## 许可证

MIT
