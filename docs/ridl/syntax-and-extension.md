# RIDL 语法与扩展

## 概述

RIDL (Rust Interface Definition Language) 是一种用于定义 JavaScript 接口的强类型接口定义语言，专门针对 mquickjs 的 ES5 功能集和 Rust 后端进行了优化。该 RIDL 旨在提供一种简洁、类型安全的方式来定义 JavaScript 接口，以便在 Rust 后端自动生成类型转换代码。当前实现的是 mquickjs 后端实现。

## 语法设计原则

1. **强类型**：所有接口、方法、属性都必须有明确的类型声明
2. **ES5 兼容**：语法设计基于 ES5 的功能集
3. **Rust 集成**：类型定义与 Rust 类型映射
4. **强类型优先**：默认不依赖 JS 的隐式类型转换（ToString/ToNumber）。如需额外收紧，可使用 `mode strict;`。
5. **Rust 风格语法**：采用类似 Rust 的语法风格，包括类型后置等特性

## 文法（精简）

以下片段用于描述本项目 RIDL 的关键扩展点（非完整语法）。实现以 `deps/ridl-tool/src/parser/grammar.pest` 为准。

### 文件级 mode

- 必须在文件顶部，且位于 `module ...` 之前

```ebnf
idl          ::= SOI mode_decl? module_decl? definition* EOI
mode_decl    ::= "mode" WS mode_name ";"
mode_name    ::= "strict"
```

### 参数与可变参数（varargs）

- `...` 只能出现在参数列表的最后一个参数

```ebnf
param_list   ::= param ("," param)*
param        ::= normal_param | variadic_param
normal_param ::= identifier ":" type
variadic_param ::= "..." identifier ":" type
```

## 类型后置规则

RIDL 采用类似 Rust 的类型后置语法，这体现在以下几个方面：

### 1. 参数类型后置

函数参数采用 `name: type` 的形式，而不是 `type name`：

```
interface Example {
    // 正确：类型后置
    fn correctExample(message: string, count: int);
    
    // 错误：类型前置（不支持）
    // fn incorrectExample(string message, int count);
}
```

### 2. 变量和属性类型后置

结构体和类的属性定义也采用类型后置：

```
struct Person {
    name: string;
    age: int;
    email: string?;
}

class Logger {
    level: LogLevel;
    enabled: bool;
}
```

### 3. 可空类型 (Nullable Types)

可空类型使用 `?` 后缀表示该值可以为 `null` 或 `undefined`，对应 Rust 的 `Option<T>` 类型：

```
interface NullableExample {
    // 可空基础类型
    fn getName() -> string?;
    fn getAge() -> int?;
    
    // 可空复杂类型
    fn getPerson() -> Person?;
    fn getItems() -> array<string>?;
    
    // 可空联合类型
    fn getValue() -> (string | int)?;
    
    // 参数也可以是可空的
    fn processName(name: string?);
}

// 在结构体和类中使用可空类型
struct UserProfile {
    id: int;
    name: string;
    email: string?;      // 可空邮箱
    phone: string?;      // 可空电话
    address: Address?;   // 可空地址
}

class DataProcessor {
    cache: map<string, string>?;
    config: Configuration?;
}
```

在 JavaScript 中，可空类型可以接受 `null` 或实际类型的值。在 Rust 中，这些类型会被转换为 `Option<T>` 类型，其中 `Some(value)` 表示有值，`None` 表示空值。

### 4. Callback 类型

Callback 是 RIDL 中的一等类型，有以下特点：

1. **作为结构体成员**：struct 中允许以 callback 作为变量，callback 在实现层会转换为 callbackId，可以序列化和反序列化：

```
struct CallbackContainer {
    on_success: callback OnSuccess(result: string);
    on_error: callback OnError(error: string);
}
```

2. **作为函数参数或返回值**：callback 允许作为函数参数或返回值：

```
interface CallbackExample {
    fn processWithCallback(data: string, callback: callback ProcessCallback(success: bool, result: string));
    fn getCallback() -> callback GetCallback();
}
```

3. **命名Callback声明**：callback 可以有自己的名字，定义格式为 `callback Name(param: Type)`，与函数类似但使用 `callback` 关键字。注意，回调函数不具有返回值，主要用于传递异步结果：

```
// 命名Callback声明（注意：回调函数没有返回值）
callback MyCallback(a: int, msg: string);

// 在函数中使用命名Callback
interface Example {
    fn invokeCb(cb: MyCallback);
}
```

4. **回调函数的用途**：回调函数主要用于传递异步操作的结果，它们没有返回值，因为异步操作的结果是通过回调参数传递的：

```
// 正确的回调使用方式
callback ProcessCallback(success: bool, result: string);  // 无返回值

interface Example {
    fn processDataAsync(input: string, callback: ProcessCallback);
}
```

5. **匿名Callback**：也可以直接在使用时声明匿名Callback类型：

```
interface Example {
    fn processData(input: string, cb: callback(success: bool, result: string));
}
```

### 5. 函数返回值后置

函数返回值使用 `->` 操作符声明，紧跟在函数参数列表之后：

```
interface Example {
    // 有返回值的函数
    fn getValue() -> int;
    fn process(input: string) -> (bool | object);
    
    // 无返回值的函数（可省略void）
    fn log(message: string);
    fn update(value: int) -> void;  // 或显式声明void
}
```

## mode strict

RIDL 支持文件级声明 `mode strict;`，用于收紧 glue 层的参数类型检查，避免 QuickJS 的默认类型转换（ToString/ToNumber）掩盖调用错误。

### 语法

- 作用域：文件级（必须放在 RIDL 文件顶部，位于 `module ...` 之前）
- 当前启用：`ridl-modules/stdlib/src/stdlib.ridl` 已启用 strict（用于 console 等 stdlib glue）

```ridl
mode strict;

singleton console {
    fn log(content: string);
}
```

### 语义（v1）

- default（未声明 mode）
  - 强类型（安全优先）：参数必须满足声明的 JS 类型；不进行 JS 默认转换（ToString/ToNumber）。
  - 例：`echo_str(123)` 抛 TypeError（因为参数必须是 string）
  - 例：`add_i32("1", "2")` 抛 TypeError（因为参数必须是 number）

- strict
  - 同样是强类型；并额外收紧：禁止 `any` 出现在非可变参数位置。
  - 对不满足类型要求的参数：抛 TypeError，并返回 `JS_EXCEPTION`

### 当前类型检查策略（v1）

> v1 目标类型：`string/bool/i32/f64/any`（含 `void` 返回）。

- `string`
  - strict：必须是 JS string（`JS_IsString(ctx, val) != 0`），否则 TypeError
  - default：必须是 JS string（不允许 ToString）

- `bool`
  - strict/default：必须是 JS bool（内部基于 tag 检查 `JS_TAG_BOOL`）

- `int` / `double`
  - strict：必须是 JS number（`JS_IsNumber`），然后 `JS_ToInt32` / `JS_ToNumber`
  - default：必须是 JS number（`JS_IsNumber`），然后 `JS_ToInt32` / `JS_ToNumber`

- `any`
  - default：允许（不做类型限制，透传）
  - strict：仅允许用于可变参数（varargs）。非 varargs 位置使用 `any` 会在 RIDL 校验阶段报错。

### 限制与后续

- v1 暂不对 `null/undefined` 做额外限制（等 nullable/optional 类型完善后再扩展 strict 规则）。

## 可变参数（varargs）

RIDL 支持在函数/方法参数列表中声明可变参数（必须位于最后一个参数位置）：

```ridl
fn log(...args: any);
fn sum(...nums: int) -> int;
```

语义：

- `...args: T` 表示该参数会绑定到 `argv[idx..argc)` 的所有剩余参数。
- `T` 的检查按文件 mode 执行：
  - default：强类型（安全优先），逐元素检查必须满足 T。
  - strict：同样逐元素强类型检查；并且仅 varargs 位置允许 `T = any`。

错误：

- 若某个元素类型不匹配，应抛 TypeError（建议消息带上参数名与元素下标，例如 `invalid int argument: nums[2]`）。

## 基础类型映射

| RIDL 类型 | JS 类型 | Rust 类型 | 说明 |
|----------|---------|-----------|------|
| `bool` | Boolean | `bool` | 布尔值 |
| `int` | Number | `i32` | 32位整数 |
| `float` | Number | `f32` | 32位单精度浮点数 |
| `double` | Number | `f64` | 64位双精度浮点数 |
| `string` | String | `String` | 字符串 |
| `array<T>` | Array | `Vec<T>` | 数组（T 为具体类型） |
| `object` | Object | `Object` | 对象 |
| `callback` | Function | `AsyncCallback` | 异步回调 |
| `null` | null | `Option<T>` | 空值 |
| `void` | undefined | `()` | 无返回值 |

## 复杂类型

### 联合类型 (Union Types)

联合类型允许一个值可以是几种类型之一，用 `|` 分隔多个类型：

```
interface DataProcessor {
    // 参数可以是字符串、数字或字符串数组
    fn processInput(data: string | int | array<string>);
    
    // 返回值可以是布尔值或对象
    fn validateData(input: string) -> (bool | object);
    // 或者不使用括号，以下写法是等价的
    fn validateData2(input: string) -> bool | string;
}
```

### 可空类型 (Nullable Types)

可空类型使用 `?` 后缀表示值可能是 `null` 或 `undefined`：

```
interface ConfigManager {
    // 配置项可以是字符串或 null
    fn getConfigValue(key: string) -> string?;
    
    // 对象属性可以是多种类型或 null
    fn setOption(key: string, value: string | int | bool | null);
}
```

### 字典类型 (Dictionary Types)

字典类型使用 `map<K, V>` 语法表示键值对集合：

```
interface DictionaryExample {
    fn getCounts() -> map<string, int>;
    fn setMetadata(metadata: map<string, string>);
}
```

### 自定义类型与序列化 (Custom Types with Serialization)

对于复杂自定义类型，我们采用序列化方式支持。RIDL 支持多种序列化格式的结构体定义，系统将自动生成相应格式的序列化/反序列化代码：

```
// 默认使用 JSON 序列化
struct Person {
    name: string;
    age: int;
    email: string?;
}

// 使用 JSON 序列化（显式声明）
json struct Address {
    street: string;
    city: string;
    country: string;
}

// 使用 msgpack 序列化
msgpack struct Configuration {
    settings: map<string, string>;
    features: array<string>;
}

// 通过 import 语法导入 protobuf 序列化的类型
// protobuf 定义通过 .proto 文件给出，RIDL 层通过 import 语法显式导入
import NetworkPacket as Packet from Packet.proto
import TypeA, TypeB from Types.proto

// 注意：不支持 import * from Something.proto 语法，因为我们需要明确知道导入的具体类型
// 同时，RIDL不支持导入其他RIDL文件，仅支持从.proto文件导入类型

// 被导入的类型被视为 proto struct
```

使用自定义类型的接口：

```
interface ContactManager {
    // 接受 JSON 序列化的 Person 对象
    fn addPerson(person: Person);
    
    // 接受 msgpack 序列化的配置对象
    fn updateConfig(config: Configuration);
    
    // 接受 protobuf 序列化的网络包（通过 import 导入）
    fn sendPacket(packet: Packet);
    
    // 返回序列化的 Person 对象
    fn getPerson(id: string) -> Person;
    
    // 复杂类型参数
    fn updateContact(person: Person, addresses: array<Address>);
    
    // 联合类型中使用自定义类型
    fn find(query: string) -> (Person | Contact);
}
```

不同序列化机制的特点：

- **JSON 序列化**：文本格式，可读性好，兼容性强，但体积较大，解析速度较慢
- **MessagePack 序列化**：二进制格式，体积小，解析速度快，适合高频数据传输
- **Protocol Buffers 序列化**：二进制格式，需要预定义 schema，性能最优，适合大规模数据处理

## 语法定义

### 1. 接口定义

```
interface Console {
    fn log(message: string);
    fn error(message: string);
    fn warn(message: string);
    fn debug(message: string);
}
```

### 2. 类定义

```
class Person {
    name: string;
    age: int;
    
    Person(name: string, age: int);
    fn getName() -> string;
    fn getAge() -> int;
    fn setAge(age: int);
}
```

### 3. 枚举定义

```
enum LogLevel {
    DEBUG = 0,
    INFO = 1,
    WARN = 2,
    ERROR = 3
}
```

### 4. Import 语句

```
// 从 .proto 文件导入单个类型
import NetworkPacket as Packet from Packet.proto

// 导入多个特定类型
import TypeA, TypeB from Types.proto
```

**重要说明**：
1. **不支持通配符导入**：不支持 `import * from Something.proto` 语法。我们仅"借用"proto中定义的类型而不解析proto文件，使用通配符导入会导致无法明确知道导入的具体类型，影响类型安全和代码生成准确性。

2. **不支持导入其他RIDL文件**：RIDL被设计用来进行简单的接口声明，暂时不支持相互引用其他RIDL文件。后续会考虑是否支持此功能。

### 5. 函数定义

```
// 全局函数
fn setTimeout(callback: callback, delay: int);
fn setInterval(callback: callback, delay: int);

// 带返回值的函数
fn add(a: int, b: int) -> int;
fn concat(a: string, b: string) -> string;

// 使用复杂类型的函数
fn processMixed(input: string | array<string> | int) -> (bool | object);

// 使用自定义类型的函数
fn logMessage(entry: LogEntry) -> bool;
fn processBatch(batch: LogBatch);
```

### 6. 数组、字典和对象

```
interface ComplexExample {
    fn getItems() -> array<string>;
    fn processArray(items: array<int>);
    fn getConfig() -> object;
    fn getMetadata() -> map<string, string>;
    fn updateConfig(config: object);
    fn getGroupedData() -> map<string, array<int>>;
    
    // 使用自定义类型的接口
    fn getLogEntries() -> array<LogEntry>;
    fn addLogEntries(entries: array<LogEntry>);
    fn getLogMap() -> map<string, LogEntry>;
}
```

### 7. 可空类型 (Nullable Types)

可空类型使用 `?` 后缀表示值可能是 `null` 或 `undefined`：

```
interface ConfigManager {
    // 配置项可以是字符串或 null
    fn getConfigValue(key: string) -> string?;
    
    // 对象属性可以是多种类型或 null
    fn setOption(key: string, value: string | int | bool | null);
}
```

### 8. 回调函数

```
// 定义命名回调类型
callback ProcessCallback(result: string | object, success: bool);
callback LogCallback(entry: LogEntry);
callback MixedCallback(data: string | int | array<string>);
callback ErrorFirstCallback(error: string?, result: LogEntry?);
callback ResultCallback(code: int, msg: string);

interface CallbackExample {
    // 使用回调函数的方法
    fn processData(input: string, callback: ProcessCallback);
    
    // 使用自定义类型的回调
    fn processWithLogCallback(entry: LogEntry, callback: LogCallback);
    
    // 使用联合类型的回调
    fn processMixedCallback(data: string, callback: MixedCallback);
    
    // 错误优先回调
    fn processWithErrorCallback(input: string, callback: ErrorFirstCallback);
    
    // 使用resultCallback示例
    fn processWithResultCallback(input: string, callback: ResultCallback);
}

// 定义命名回调类型 - 基本回调
callback SimpleCallback();

// 定义命名回调类型 - 带参数的回调
callback DataCallback(data: string | int);

// 定义命名回调类型 - 错误优先的回调（Node.js 风格）
callback ErrorFirstCallbackType(error: string?, result: string | object?);

// 定义命名回调类型 - 多参数回调
callback ProcessCallbackType(result: string | object, success: bool, code: int);
```

### 9. 异步方法

```
interface AsyncExample {
    // 异步方法使用回调
    fn processAsync(input: string, callback: ProcessCallback);
    
    // 异步方法处理自定义类型
    fn saveLogAsync(entry: LogEntry, callback: callback(success: bool, error: string?));
    
    // 异步批量处理
    fn processBatchAsync(
        entries: array<LogEntry>, 
        callback: callback(results: array<LogEntry>, error: string?)
    );
}
```

### 10. 异常处理

RIDL不支持异常定义，错误处理需要通过返回值或回调实现。接口定义中不包含throws子句。

## 序列化机制

RIDL 支持多种序列化格式的结构体定义，系统将自动生成相应格式的序列化/反序列化代码：

1. **JSON 序列化**：使用 `json struct` 关键字定义，系统会生成 JSON 序列化/反序列化代码
2. **MessagePack 序列化**：使用 `msgpack struct` 关键字定义，系统会生成 MessagePack 序列化/反序列化代码
3. **Protocol Buffers 序列化**：通过 `import` 语法从 `.proto` 文件导入，系统会生成 Protocol Buffers 序列化/反序列化代码


对于 `json struct` 和 `msgpack struct` 类型，系统将自动生成序列化和反序列化代码：

- JavaScript 侧：传递普通 JavaScript 对象，自动转换为相应格式
- Rust 侧：接收序列化数据，自动反序列化为 Rust 结构体
- 返回值：Rust 结构体自动序列化，再转换为 JavaScript 对象

Protocol Buffers 类型通过 import 语法导入：

```
// 从 .proto 文件导入 protobuf 类型
import NetworkPacket as Packet from Packet.proto
import TypeA, TypeB from Something.proto

// 注意：不支持 import * from Something.proto 语法，因为我们需要明确知道导入的具体类型
// 同时，RIDL不支持导入其他RIDL文件，仅支持从.proto文件导入类型
```

导入的类型被视为 `proto struct` 并支持相应的序列化操作。

### 序列化类型转换

| RIDL 类型 | JS 类型 | Rust 类型 | 说明 |
|----------|---------|-----------|------|
| `json struct` | Object | `CustomStruct` | JSON序列化结构体 |
| `msgpack struct` | Object | `CustomStruct` | MessagePack序列化结构体 |
| `import` 类型 | Object | `CustomStruct` | 从.proto文件导入的Protocol Buffers序列化结构体 |
| `array<LogEntry>` | Array of Objects | `Vec<LogEntry>` | 具体类型的对象数组 |
| `map<string, LogEntry>` | Object | `HashMap<String, LogEntry>` | 具体类型的对象映射 |

## 异步处理与回调函数

### 回调函数定义

JavaScript 的异步操作通常使用回调函数模式，RIDL 提供 `callback` 关键字定义回调类型：

```
// 定义回调类型 - 基本回调
callback SimpleCallback();

// 定义回调类型 - 带参数的回调
callback DataCallback(data: string | int);

// 定义回调类型 - 错误优先的回调（Node.js 风格）
callback ErrorFirstCallbackType(error: string?, result: string | object?);

// 定义回调类型 - 多参数回调
callback ProcessCallbackType(result: string | object, success: bool, code: int);
```

### 异步方法定义

在接口中定义接受回调的异步方法：

```
interface AsyncProcessor {
    // 接受回调的异步方法
    fn processAsync(data: string, callback: ProcessCallback);
    
    // 错误优先的异步方法
    fn validateAsync(input: string, callback: ErrorFirstCallbackType);
    
    // 多个回调参数的方法
    fn fetchResource(url: string, callback: callback(data: object?, error: string?, status: int));
}
```

### Rust 异步与 JS 回调的桥接

为了统一 Rust 的 async/await 和 JavaScript 的回调模式，我们提供以下机制：

#### 1. Future 到回调的转换

```
// RIDL 定义异步函数
interface DataFetcher {
    // 在 RIDL 中定义异步方法，但实际在 JS 中通过回调调用
    fn fetchData(url: string, callback: callback(data: object?, error: string?));
}
```
对应的 Rust 实现：

```rust
impl DataFetcher {
    async fn fetch_data_impl(url: String) -> Result<serde_json::Value, String> {
        // 实际的异步实现
        // 使用 Rust 的 async/await
        todo!()
    }
    
    fn fetch_data(&self, url: String, callback: AsyncCallback) {
        // 启动异步任务
        let future = Self::fetch_data_impl(url);
        
        // 将 Future 转换为回调
        tokio::spawn(async move {
            match future.await {
                Ok(data) => {
                    // 调用回调成功
                    callback.call_with_args(Some(data), None);
                }
                Err(error) => {
                    // 调用回调失败
                    callback.call_with_args(None, Some(error));
                }
            }
        });
    }
}
```

## 语法说明

### 11. 使用 using 进行类型重命名

RIDL 使用 `using` 关键字对任意类型（包括函数类型）进行重命名，替代传统的 `typedef` 语法：

```
// 重命名基础类型
using UserId = int;
using Email = string;

// 重命名复杂类型
using UserList = array<User>;
using Metadata = map<string, string>;

// 重命名回调类型（为已定义的回调创建别名）
using SuccessCallback = callback(result: string);  // 回调函数无返回值
using ErrorCallback = callback(error: string);     // 回调函数无返回值

// 在函数中使用重命名的类型
interface UserService {
    fn getUser(id: UserId) -> User?;
    fn processUsers(users: UserList, successCb: SuccessCallback, errorCb: ErrorCallback);
}

// 在结构体中使用
struct Task {
    id: int;
    onSuccess: SuccessCallback;
    onError: ErrorCallback;
}
```

这种语法提供了一种清晰的方式为复杂类型创建别名，提高代码的可读性和可维护性。

### 12. 复合类型返回值的括号用法

对于返回复合类型，括号是可选的，以下两种写法是等价的：

```
// 使用括号
fn someFunc() -> (bool | string);

// 省略括号
fn someFunc2() -> bool | string;

// 对于更复杂的联合类型
fn complexFunc() -> (Person | LogEntry | string);
// 等价于
fn complexFunc2() -> Person | LogEntry | string;
```

这种灵活性允许开发者根据可读性需求选择是否使用括号，特别是在复杂的联合类型情况下，使用括号可以提高代码的可读性。

## 错误处理机制

为确保用户能够快速定位和修复RIDL文件中的错误，需要实现一个全面的错误处理系统。

#### 1. 错误类型分类

##### 1.1 语法错误 (Syntax Errors)
- 词法错误：无效字符、未闭合的字符串、无效标识符等
- 语法错误：缺少分号、括号不匹配、错误的语法规则等

##### 1.2 语义错误 (Semantic Errors) 
- 无效标识符：使用了关键字作为标识符
- 无效类型：引用了不存在的类型
- 重复定义：同一作用域内重复定义标识符
- 无效的类型引用：引用了不存在的类型
- 模块声明位置错误：module声明不在文件开头

#### 2. 技术方案

##### 2.1 语法错误处理
利用pest内置的错误处理机制，它已经可以提供：
- 错误位置（行号、列号）
- 错误原因描述
- 问题代码的上下文显示

##### 2.2 语义错误处理
在AST构建阶段进行额外的语义验证，实现一个专门的验证器模块，对以下方面进行检查：

1. **标识符验证**：检查是否使用了关键字作为标识符
2. **类型引用验证**：验证所有类型引用是否有效
3. **重复定义检查**：确保没有重复定义
4. **模块声明位置验证**：验证module声明是否在文件开头

#### 3. 错误报告格式

建议的错误报告格式：

``rust
#[derive(Debug, Clone)]
pub struct RIDLError {
    pub message: String,
    pub line: usize,
    pub column: usize,
    pub file: String,
    pub error_type: RIDLErrorType,
}

#[derive(Debug, Clone)]
pub enum RIDLErrorType {
    SyntaxError,
    SemanticError,
    ValidationError,
}
```

有关 RIDL 标准库扩展机制的详细信息，请参见 [标准库扩展机制文档](stdlib-extension-mechanism.md)。