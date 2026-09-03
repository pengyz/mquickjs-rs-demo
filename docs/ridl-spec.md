# RIDL 语法规范

RIDL（Rust Interface Definition Language）是用于定义 Rust/JS 互操作接口的领域特定语言。

## 1. 概述

RIDL 文件（`.ridl`）定义了：
- **类型**：Rust 和 JS 之间的数据类型映射
- **接口**：Rust trait 和 JS 对象的方法签名
- **模块**：代码组织和导入导出

### 设计目标

- **类型安全**：编译时检查 Rust/JS 类型匹配
- **零手写 FFI**：自动生成绑定代码
- **TypeScript 兼容**：语法尽量接近 TypeScript
- **嵌入式友好**：生成的代码适合资源受限环境

## 2. 词法

### 2.1 关键字

```
interface  class  enum  struct  const  readonly  property  proto
array  map  true  false  fn  import  as  from  using  module
singleton  opaque  Traced
```

### 2.2 标识符

```
identifier = !keyword ~ [a-zA-Z_] ~ [a-zA-Z0-9_]*
```

- 不能以数字开头
- 不能是关键字
- 可以包含下划线

### 2.3 字面量

| 类型 | 示例 |
|---|---|
| 字符串 | `"hello"`, `"world"` |
| 整数 | `0`, `42`, `-1` |
| 浮点数 | `3.14`, `0.0` |
| 布尔 | `true`, `false` |
| 空 | `null` |

### 2.4 注释

```typescript
// 单行注释
/* 多行注释 */
```

## 3. 类型系统

### 3.1 基本类型

| RIDL 类型 | Rust 类型 | JS 类型 | 说明 |
|---|---|---|---|
| `bool` | `bool` | `boolean` | 布尔值 |
| `i32` | `i32` | `number` | 32 位整数 |
| `i64` | `i64` | `number` | 64 位整数 |
| `f32` | `f32` | `number` | 32 位浮点数 |
| `f64` | `f64` | `number` | 64 位浮点数 |
| `string` | `String` | `string` | 字符串 |
| `void` | `()` | `undefined` | 无返回值 |
| `object` | `JSValue` | `object` | 任意 JS 对象 |
| `any` | `JSValue` | `any` | 任意类型 |
| `null` | `Option<()>` | `null` | 空值 |

### 3.2 复合类型

```typescript
// 数组
array<i32>          // Vec<i32>
array<string>       // Vec<String>
array<Traced<Value>> // Vec<Traced<Value>>

// 字典
map<string, i32>    // HashMap<String, i32>
map<string, object> // HashMap<String, JSValue>

// 可选值
string?             // Option<String>
i32?                // Option<i32>
Traced<Value>?      // Option<Traced<Value>>

// 联合类型
string | i32        // Enum { String(String), I32(i32) }
string | i32 | bool // Enum { String(String), I32(i32), Bool(bool) }

// 分组
(string | i32)?     // Option<Enum>
```

### 3.3 特殊类型

```typescript
// Traced<T> - GC 追踪字段
Traced<Value>       // 持有 JSValue，参与 GC 标记
Traced<Value>?      // 可选的 GC 追踪字段

// 回调
callback(x: i32) -> void  // 函数类型
callback EventHandler(event: object)  // 命名回调
```

## 4. 定义

### 4.1 模块声明

```typescript
// 基本模块
module my.app@1.0.0;

// 带路径的模块
module my.lib.utils@2.1.0;

// 版本格式：MAJOR.MINOR.PATCH
module my.app@1.0.0;
module my.app@1.0;
module my.app@1;
```

### 4.2 模式声明

```typescript
// 严格模式（禁用某些特性）
mode strict;
```

### 4.3 导入

```typescript
// 花括号导入（TypeScript 风格）
import { Foo } from "my_lib";
import { Foo, Bar, Baz } from "my_lib";
import { Foo as F } from "my_lib";
import { Foo, Bar as B, } from "my_lib";  // 尾逗号

// 通配符导入
import * as Utils from "my_lib";
```

### 4.4 类型别名

```typescript
using StringMap = map<string, string>;
using IntList = array<i32>;
using Callback = callback(x: i32) -> void;
```

### 4.5 接口

```typescript
interface Drawable {
    fn draw(ctx: object) -> void;
    fn getBounds() -> array<f32>;
    fn isVisible() -> bool;
}
```

### 4.6 类

```typescript
class Calculator {
    // 常量
    const MAX_HISTORY: i32 = 100;

    // 变量
    var count: i32 = 0;

    // 原型变量（JS prototype 上的属性）
    proto var state: i32 = 0;

    // 原型只读属性
    proto readonly property id: string;

    // 原型读写属性
    proto property value: i32;

    // 只读属性
    readonly property name: string;

    // 读写属性
    property history: array<i32>;

    // 构造函数
    constructor(initialValue: i32);

    // 方法
    fn add(a: i32, b: i32) -> i32;
    fn clear() -> void;

    // GC 追踪字段
    opaque {
        held: Traced<Value>
        items: array<Traced<Value>>
        cache: map<string, Traced<Value>>
        optional: Traced<Value>?
        plainData: i32
    }
}
```

### 4.7 单例

```typescript
singleton MathService {
    fn initialize() -> void;
    fn calculate(x: i32) -> i32;
    readonly property version: string;
    property precision: i32;
}
```

### 4.8 枚举

```typescript
enum Color {
    RED,
    GREEN,
    BLUE,
}

enum Status {
    PENDING = 0,
    ACTIVE = 1,
    INACTIVE = 2,
}
```

### 4.9 结构体

```typescript
// 默认序列化格式
struct Point {
    x: f64;
    y: f64;
}

// JSON 序列化
json struct Config {
    name: string;
    value: i32;
    enabled: bool;
}

// MessagePack 序列化
msgpack struct Data {
    id: i64;
    payload: string;
}

// Protobuf 序列化
protobuf struct Message {
    header: string;
    body: string;
}
```

### 4.10 全局函数

```typescript
fn helper(x: i32, y: i32) -> i32;
fn logMessage(msg: string) -> void;
fn processData(data: array<i32>) -> array<string>;
```

### 4.11 回调

```typescript
// 匿名回调
callback(x: i32) -> void;

// 命名回调
callback EventHandler(event: object);
callback AsyncCallback(result: string?, error: string?);
```

## 5. Opaque 块

Opaque 块定义了类的 GC 追踪字段。这些字段在 GC 标记阶段被自动标记。

### 5.1 语法

```typescript
class MyNode {
    opaque {
        // Traced<T> 字段 - 参与 GC 标记
        held: Traced<Value>
        items: array<Traced<Value>>
        cache: map<string, Traced<Value>>
        optional: Traced<Value>?

        // 普通字段 - 不参与 GC 标记
        count: i32
        name: string
    }

    fn doSomething() -> void;
}
```

### 5.2 语义

- `Traced<T>` 字段在 GC 标记阶段被标记为可达
- `Option<Traced<T>>` 字段在 `Some` 时被标记
- `array<Traced<T>>` 字段的每个元素被标记
- `map<K, Traced<T>>` 字段的每个值被标记
- 普通字段（`i32`, `string` 等）不参与 GC 标记

### 5.3 生成的代码

```rust
// 自动生成的 opaque struct
pub struct MyNodeOpaque {
    pub held: mquickjs_rs::Traced<mquickjs_rs::Value>,
    pub items: Vec<mquickjs_rs::Traced<mquickjs_rs::Value>>,
    pub cache: std::collections::HashMap<String, mquickjs_rs::Traced<mquickjs_rs::Value>>,
    pub optional: Option<mquickjs_rs::Traced<mquickjs_rs::Value>>,
    pub count: i32,
    pub name: String,
}

// 自动生成的 gc_mark 方法
impl MyNodeOpaque {
    pub(crate) unsafe fn gc_mark(&self, mf: *const mquickjs_rs::mquickjs_ffi::JSMarkFunc) {
        unsafe { self.held.gc_mark(mf) };
        for item in &self.items {
            unsafe { item.gc_mark(mf) };
        }
        for (_key, value) in &self.cache {
            unsafe { value.gc_mark(mf) };
        }
        if let Some(ref inner) = self.optional {
            unsafe { inner.gc_mark(mf) };
        }
    }
}
```

## 6. 类型映射

### 6.1 基本类型映射

| RIDL | Rust | JS |
|---|---|---|
| `bool` | `bool` | `boolean` |
| `i32` | `i32` | `number` |
| `i64` | `i64` | `number` |
| `f32` | `f32` | `number` |
| `f64` | `f64` | `number` |
| `string` | `String` | `string` |
| `void` | `()` | `undefined` |
| `object` | `JSValue` | `object` |
| `any` | `JSValue` | `any` |
| `null` | `Option<()>` | `null` |

### 6.2 复合类型映射

| RIDL | Rust |
|---|---|
| `array<T>` | `Vec<T>` |
| `map<K, V>` | `HashMap<K, V>` |
| `T?` | `Option<T>` |
| `T \| U` | `Enum { T(T), U(U) }` |
| `Traced<T>` | `Traced<T>` |
| `callback(...)` | `Box<dyn Fn(...)>` |

### 6.3 命名转换

- **camelCase → snake_case**：`myMethod` → `my_method`
- **PascalCase → snake_case**：`MyClass` → `my_class`
- **保留关键字**：`class`, `fn`, `let` 等不能作为标识符

## 7. 作用域和可见性

### 7.1 全局作用域

```typescript
// 全局函数
fn globalHelper() -> void;

// 全局类型别名
using GlobalType = i32;
```

### 7.2 模块作用域

```typescript
module my.app@1.0.0;

// 模块内的定义
class MyClass { ... }
singleton MyService { ... }
```

### 7.3 导入作用域

```typescript
import { Helper } from "utils";

// 使用导入的类型
class MyClass {
    fn useHelper(h: Helper) -> void;
}
```

## 8. 错误处理

### 8.1 语法错误

```typescript
// 缺少分号
class Foo {
    fn bar() -> void  // 错误：缺少分号
}

// 缺少花括号
class Foo fn bar() -> void; }  // 错误：缺少左花括号

// 缺少类型
opaque { x: }  // 错误：缺少字段类型
```

### 8.2 语义错误

```typescript
// 未定义的类型
class Foo {
    fn bar() -> UndefinedType;  // 错误：UndefinedType 未定义
}

// 重复的字段名
class Foo {
    opaque {
        x: i32
        x: string  // 错误：重复的字段名
    }
}
```

## 9. 示例

### 9.1 完整的 RIDL 文件

```typescript
// my_module.ridl
module my.app@1.0.0;

import { EventEmitter } from "events";

using StringMap = map<string, string>;

callback ErrorCallback(error: string?);

interface Serializable {
    fn serialize() -> string;
    fn deserialize(data: string) -> void;
}

enum LogLevel {
    DEBUG = 0,
    INFO = 1,
    WARN = 2,
    ERROR = 3,
}

json struct Config {
    name: string;
    version: string;
    debug: bool;
}

class Logger {
    const MAX_ENTRIES: i32 = 1000;
    var entries: array<string> = [];

    readonly property level: LogLevel;
    property format: string;

    constructor(level: LogLevel);

    fn log(message: string) -> void;
    fn error(message: string, callback: ErrorCallback?) -> void;
    fn getEntries() -> array<string>;

    opaque {
        emitter: Traced<Value>
        buffer: array<Traced<Value>>
    }
}

singleton AppService {
    fn initialize(config: Config) -> void;
    fn getLogger(name: string) -> Logger;
    readonly property version: string;
    property debug: bool;
}
```

### 9.2 生成的 Rust 代码

```rust
// 自动生成的 api.rs
pub trait LoggerClass {
    fn get_level(&mut self) -> LogLevel;
    fn get_format(&mut self) -> String;
    fn set_format(&mut self, v: String);
    fn log(&mut self, message: String);
    fn error(&mut self, message: String, callback: Option<Box<dyn Fn(Option<String>)>>);
    fn get_entries(&mut self) -> Vec<String>;
    fn gc_mark(&self, mf: *const mquickjs_rs::mquickjs_ffi::JSMarkFunc);
}

pub struct LoggerOpaque {
    pub emitter: mquickjs_rs::Traced<mquickjs_rs::Value>,
    pub buffer: Vec<mquickjs_rs::Traced<mquickjs_rs::Value>>,
}

impl LoggerOpaque {
    pub(crate) unsafe fn gc_mark(&self, mf: *const mquickjs_rs::mquickjs_ffi::JSMarkFunc) {
        unsafe { self.emitter.gc_mark(mf) };
        for item in &self.buffer {
            unsafe { item.gc_mark(mf) };
        }
    }
}
```

## 10. 限制

### 10.1 当前不支持的特性

- 泛型（TypeScript generics）
- 装饰器（TypeScript decorators）
- 可选链（`?.`）
- 空值合并（`??`）
- 动态导入（`import()`）
- 命名空间（`namespace`）

### 10.2 平台限制

- mquickjs 的 GC sweep 不调用 finalizer（只在 context teardown 时调用）
- `object` 类型作为返回值不支持
- `array<T>` 作为返回值不支持（需要特殊处理）

## 11. 最佳实践

### 11.1 命名约定

- 类名：PascalCase（`MyClass`）
- 方法名：camelCase（`myMethod`）
- 常量：UPPER_SNAKE_CASE（`MAX_SIZE`）
- 模块名：snake_case（`my_module`）

### 11.2 类型选择

- 优先使用具体类型（`i32`, `string`）而非 `any`
- 使用 `Traced<T>` 而非 `object` 持有 JS 引用
- 使用 `Option<T>` 而非 `null` 表示可选值

### 11.3 GC 考虑

- 在 opaque 块中声明所有需要 GC 追踪的字段
- 实现 `gc_mark` 方法标记所有 `Traced<T>` 字段
- 避免在 opaque 中存储大量小对象（影响 GC 性能）
