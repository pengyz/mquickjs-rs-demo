# mquickjs RIDL (Rust Interface Description Language) syntax design

## 概述

mquickjs RIDL (Rust Interface Description Language) 是一种用于定义 JavaScript 接口的强类型接口定义语言，专门针对 mquickjs 的 ES5 功能集和 Rust 后端进行了优化。该 RIDL 旨在提供一种简洁、类型安全的方式来定义 JavaScript 接口，以便在 Rust 后端自动生成类型转换代码。当前实现的是 mquickjs 后端实现。

## 语法设计原则

1. **强类型**：所有接口、方法、属性都必须有明确的类型声明
2. **ES5 兼容**：语法设计基于 ES5 的功能集
3. **Rust 集成**：类型定义与 Rust 类型映射
4. **自动转换**：支持 JSValue 到 Rust 类型，Rust 类型到 JSValue 的类型转换
5. **Rust 风格语法**：采用类似 Rust 的语法风格，包括类型后置等特性

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

Callback 是 JIDL 中的一等类型，有以下特点：

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
    fn processData(input: string, callback: callback(success: bool, result: string));
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

## 基础类型映射

| IDL 类型 | JS 类型 | Rust 类型 | 说明 |
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

使用自定义类型的接口：

```
interface ContactManager {
    // 接受 JSON 序列化的 Person 对象
    fn addPerson(person: Person);
    
    // 接受 msgpack 序列化的配置对象
    fn updateConfig(config: Configuration);
    
    // 接受 protobuf 序列化的网络包
    fn sendPacket(packet: NetworkPacket);
    
    // 返回序列化的 Person 对象
    fn getPerson(id: string) -> Person;
    
    // 复杂类型参数
    fn updateContact(person: Person, addresses: array<Address>);
    
    // 联合类型中使用自定义类型
    fn find(query: string) -> (Person | Contact);
}
```

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

### 6. Import 语句

```
// 从 .proto 文件导入单个类型
import NetworkPacket as Packet from Packet.proto

// 导入多个特定类型
import TypeA, TypeB from Types.proto
```

**重要说明**：
1. **不支持通配符导入**：不支持 `import * from Something.proto` 语法。我们仅"借用"proto中定义的类型而不解析proto文件，使用通配符导入会导致无法明确知道导入的具体类型，影响类型安全和代码生成准确性。

2. **不支持导入其他RIDL文件**：RIDL被设计用来进行简单的接口声明，暂时不支持相互引用其他RIDL文件。后续会考虑是否支持此功能。

### 7. 函数定义

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

### 8. 数组、字典和对象

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

### 9. 可选参数和默认值

```
interface OptionsExample {
    // 可选参数用 ? 标记
    fn drawCircle(x: int, y: int, radius: int?);
    
    // 带默认值的参数
    fn showMessage(message: string, modal: bool = false);
    
    // 联合类型参数
    fn renderContent(content: string | object, className: string?);
    
    // 自定义类型参数
    fn renderLog(log: LogEntry?);
}
```

### 10. 回调函数

```
// 定义命名回调类型
callback ProcessCallback(result: string | object, success: bool);
callback LogCallback(entry: LogEntry);
callback MixedCallback(data: string | int | array<string>);
callback ErrorFirstCallback(error: string?, result: LogEntry?);
callback ResultCallback(code: int, msg: string);

```
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
```

// 定义命名回调类型 - 基本回调
callback SimpleCallback();

// 定义命名回调类型 - 带参数的回调
callback DataCallback(data: string | int);

// 定义命名回调类型 - 错误优先的回调（Node.js 风格）
callback ErrorFirstCallbackType(error: string?, result: string | object?);

// 定义命名回调类型 - 多参数回调
callback ProcessCallbackType(result: string | object, success: bool, code: int);

```

### 11. 异步方法

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

### 12. 异常处理

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

| IDL 类型 | JS 类型 | Rust 类型 | 说明 |
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

``rust
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

### 12. 使用 using 进行类型重命名

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

### 13. 复合类型返回值的括号用法

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

## 标准库模块化机制

为了解决全局命名冲突问题，mquickjs提供了基于`require`函数的标准库模块化机制。该机制符合ES5标准，允许用户通过模块名获取功能对象，避免了全局命名空间污染。

### require机制设计

在JavaScript端，用户可以通过`require`函数获取特定模块的功能对象：

```
// 获取网络模块
var network = require("system.network");
network.getStatus();

// 获取设备信息模块
var deviceinfo = require("system.deviceinfo");
deviceinfo.getStatus();

// 获取特定版本的模块（如果存在多个版本）
var network_v1 = require("system.network@1.0");
```

### 模块化实现方案

1. **RIDL文件层面**：RIDL文件本身不包含模块语法，每个RIDL文件定义一个逻辑模块
2. **代码生成层面**：生成的代码将相关功能组织在对象中
3. **标准库注册层面**：在mquickjs初始化时，注册全局`require`函数和模块映射

### 模块命名规范

模块名采用点分隔的层次结构：
- `system.network` - 系统网络模块
- `system.deviceinfo` - 系统设备信息模块
- `ui.widget` - UI组件模块

版本号可选地附加在模块名后：
- `system.network@1.0` - 指定版本的系统网络模块

### 与现有RIDL语法的兼容性

现有的RIDL语法无需修改，所有定义的接口、类、函数将被自动组织到对应模块对象中：

```
// network.ridl
interface Network {
    fn getStatus() -> object;
    fn connect(url: string) -> bool;
}

// 生成的JavaScript代码将类似：
// var system = {
//   network: {
//     getStatus: function() { ... },
//     connect: function(url) { ... }
//   }
// }
```

### 模块化语法扩展

为了更好地支持模块化，RIDL语法扩展了模块声明功能：

```
// 全局函数，注册到global
fn setTimeout(callback: callback, delay: int);

// 全局单例对象，注册到global
singleton console {
    fn log(message: string);
    fn error(message: string);
}

// 模块化接口定义
// 注意：module声明必须位于文件开头
module system.network@1.0
interface Network {
    fn getStatus() -> string;
    fn connect(url: string) -> bool;
}

module system.deviceinfo@1.0
interface DeviceInfo {
    fn getStatus() -> string;
    fn getBatteryLevel() -> int;
}
```

### 模块注册规则

- **无`module`声明**：全局注册到global对象
  - 函数直接注册到global中
  - 单例对象作为属性注册到global上（如global.console）
- **有`module`声明**：通过`require("module.name")`访问
- **module声明作用域**：应用于整个RIDL文件，一个文件只能有一个module声明
- **module声明位置**：必须位于文件开头，在任何接口、类或其他定义之前
- **版本号格式**：module声明中的版本号格式为`主版本号.次版本号`（如`1.0`）或仅包含主版本号（如`1`），不允许超过两个部分的版本号（如`1.0.2.5`无效）

### 错误处理机制

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

#### 4. 实现策略

##### 4.1 阶段一：利用pest的错误处理
- 使用pest的错误处理机制处理语法错误
- 直接将pest错误转换为更友好的RIDL错误格式

##### 4.2 阶段二：实现语义验证器
- 创建`validator`模块
- 实现各种语义检查功能
- 在AST构建后执行验证

##### 4.3 阶段三：错误信息优化
- 提供详细的错误上下文
- 添加错误位置的代码片段显示
- 提供可能的修复建议

#### 5. 用户体验考虑

- **清晰的错误信息**：错误信息应该清晰易懂，避免技术术语
- **准确的位置信息**：提供精确的行号和列号
- **上下文信息**：显示错误行的上下文，帮助用户定位问题
- **修复建议**：当可能时，提供如何修复错误的建议

#### 6. 需要考虑的细节

- 如何处理多个错误：是报告第一个错误还是收集所有错误？
- 如何在错误发生时保持解析过程的稳定性？
- 如何将内部错误信息转换为用户友好的错误信息？

#### 7. 实现的功能

##### 7.1 语法错误处理
- 使用pest解析器检测语法错误
- 提取错误位置信息（行号、列号）
- 将pest错误转换为自定义的RIDL错误格式

##### 7.2 语义错误处理
- 实现了语义验证器模块
- 检测RIDL定义中的语义错误
- 验证类型引用、标识符等语义正确性

##### 7.3 错误报告机制
- 统一的错误类型枚举（RIDLErrorType）
- 包含详细位置信息的错误结构体（RIDLError）
- 支持批量错误收集与报告

#### 8. 核心组件

##### 8.1 语义验证器
```rust
pub struct SemanticValidator {
    pub file_path: String,
    pub errors: Vec<RIDLError>,
}
```

##### 8.2 核心API
实现了`parse_ridl_content`函数，作为错误处理的核心API：
```rust
pub fn parse_ridl_content(content: &str, file_path: &str) -> Result<Vec<ast::IDLItem>, Vec<validator::RIDLError>>
```

此函数的功能包括：
1. 首先使用pest解析器检查语法
2. 如果语法正确，使用现有parse_idl函数解析内容
3. 构建AST包装器以进行语义验证
4. 使用语义验证器进行额外检查
5. 返回解析结果或错误列表

#### 9. 实现细节

##### 9.1 语法错误捕获
- 使用pest解析器的错误处理机制
- 通过`line_col`字段提取错误位置
- 将pest错误转换为统一的RIDL错误格式

##### 9.2 语义验证
- 遍历AST节点进行语义检查
- 验证类型引用的有效性
- 检查重复定义等问题

#### 10. 测试结果

测试程序验证了错误处理功能的有效性：

1. **语法错误检测**：
   - 输入：`interface Test { fn method(int x) -> string; `（缺少右括号）
   - 输出：成功捕获语法错误，报告错误位置（第1行第28列）

2. **有效输入处理**：
   - 输入：有效的RIDL定义
   - 输出：成功解析，返回定义列表

#### 11. 使用示例

``rust
use jidl_tool;

fn main() {
    let invalid_ridl = r#"interface Test { fn method(int x) -> string; "# ;  // 缺少右括号
    let result = jidl_tool::parse_ridl_content(invalid_ridl, "test.ridl");
    
    match result {
        Ok(_) => println!("解析成功，没有检测到错误"),
        Err(errors) => {
            for error in errors {
                println!("  - 错误: {}", error.message);
                println!("    位置: {}:{}", error.file, error.line);
                println!("    类型: {:?}", error.error_type);
            }
        }
    }
}
```

错误处理功能的实现显著提升了RIDL解析器的可用性，通过提供详细的错误信息，使用户能够快速定位和修复RIDL定义中的问题。该实现遵循了设计文档中的规范，支持语法和语义错误的全面检测与报告。

### singleton对象定义

为了解决`object`作为类型和实例的语义冲突，引入了`singleton`关键字：

```
singleton console {
    fn log(message: string);
    fn error(message: string);
    fn warn(message: string);
    readonly property enabled: bool;
}
```

- `singleton`关键字仅允许用于全局注册，不允许在模块化注册中使用
- 语义上，singleton表示全局唯一实例，与模块化命名空间概念冲突
- 用于定义全局唯一的对象实例，如`console`

## 完整示例

```
// 定义日志级别枚举
enum LogLevel {
    DEBUG = 0,
    INFO = 1,
    WARN = 2,
    ERROR = 3
}

// 定义回调类型
callback ProcessCallback(result: string | object, success: bool);  // 回调函数无返回值

// 定义可序列化的日志条目结构
json struct LogEntry {
    level: LogLevel;
    message: string;
    timestamp: int;
    metadata: map<string, string>;
}

// 定义使用msgpack序列化的日志批次结构
msgpack struct LogBatch {
    entries: array<LogEntry>;
    source: string;
}

// 通过 import 语法导入 protobuf 序列化的网络数据包
import NetworkPacket from Packet.proto

// 定义日志配置对象
interface LogConfig {
    level: LogLevel;
    enabled: bool;
}

// 定义日志记录器类
class Logger {
    config: LogConfig;
    
    Logger(name: string);
    fn log(level: LogLevel, message: string);
    fn debug(message: string);
    fn info(message: string);
    fn warn(message: string);
    fn error(message: string);
    fn setConfig(config: LogConfig);
    fn getConfig() -> LogConfig;
    
    // 使用自定义类型的接口
    fn logEntry(entry: LogEntry);
    fn getRecentLogs(count: int) -> array<LogEntry>;
    fn processBatch(batch: LogBatch);
}

// 定义异步处理接口
interface AsyncProcessor {
    fn processAsync(data: string, callback: ProcessCallback);
    fn processSync(data: array<string>) -> array<string>;
}

// 使用联合类型的接口
interface DataHandler {
    // 参数可以是字符串、数字数组或对象
    fn handleData(data: string | array<int> | object);
    
    // 返回值可以是布尔值或对象
    fn validate(input: string) -> (bool | object);
    // 或者不使用括号，以下写法是等价的
    fn validate2(input: string) -> bool | string;
    
    // 使用字典类型的参数
    fn processMetadata(metadata: map<string, string | int>) -> map<string, string | int>;
    
    // 使用自定义类型的参数
    fn handleLogEntry(entry: LogEntry);
    fn processMixedData(input: string | LogEntry | LogBatch) -> (LogEntry | LogBatch);
    // 或者不使用括号，以下写法是等价的
    fn processMixedData2(input: string | LogEntry | LogBatch) -> LogEntry | LogBatch;
    
    // 异步处理自定义类型
    fn handleLogAsync(entry: LogEntry, callback: callback(success: bool, error: string?));
}

// 定义全局函数
fn getLogger(name: string) -> Logger;
fn sleep(milliseconds: int);
fn processData(input: string | int | float | array<string>) -> (bool | object);
fn processLogEntry(entry: LogEntry) -> bool;
```

## 类型转换机制

### JSValue 到 Rust 类型转换

- `JS_TAG_INT` → `i32`
- `JS_TAG_FLOAT64` → `f64`
- `JS_TAG_BOOL` → `bool`
- `JS_TAG_STRING` → `String`
- `JS_TAG_OBJECT` → `Object` 或自定义类型（通过 JSON 序列化）
- `JS_TAG_NULL` / `JS_TAG_UNDEFINED` → `Option<T>`
- 函数类型 → `AsyncCallback` 或 `Function`

### Rust 类型到 JSValue 转换

- `i32` → `JS_TAG_INT`
- `f64` → `JS_TAG_FLOAT64`
- `bool` → `JS_TAG_BOOL`
- `String` → `JS_TAG_STRING`
- 自定义类型 → `JS_TAG_STRING`（JSON 格式）
- 自定义类型对象 → `JS_TAG_OBJECT`（通过 JSON 反序列化）
- `Future<T>` → 通过回调函数暴露给 JS

## RIDL 处理流程

1. **解析**：使用解析器解析 RIDL 文件
2. **验证**：验证类型定义的正确性，包括联合类型和自定义类型的有效性
3. **生成**：生成 Rust 代码和 C 绑定代码，包括序列化/反序列化实现和异步桥接代码
4. **编译**：编译生成的代码到 mquickjs 标准库

## 限制和考虑

1. **ES5 限制**：不支持 ES6+ 特性如 Promise、async/await 等
2. **性能**：序列化/反序列化可能带来性能开销，对于高频操作需要考虑缓存机制
3. **内存管理**：确保 Rust 对象生命周期管理正确
4. **错误处理**：提供清晰的错误信息给 JS 调用者
5. **嵌套深度**：自定义类型支持嵌套，但需要限制最大嵌套深度以避免栈溢出
6. **异步限制**：由于 ES5 不支持 Promise，异步操作只能通过回调模式实现

## 扩展性

该 RIDL 设计允许后续扩展：

1. **其他序列化格式**：可扩展支持 protobuf、msgpack 等其他序列化格式
2. **装饰器**：可添加装饰器以支持元数据
3. **模块系统**：可添加模块导入/导出机制
4. **Promise 模拟**：可以添加 ES5 兼容的 Promise 模拟库

## 相关文档

- [RIDL_GRAMMAR_SPEC.md](file:///home/peng/workspace/mquickjs-rs-demo/deps/jidl-tool/doc/RIDL_GRAMMAR_SPEC.md) - 词法和文法规范，提供详细语法定义
- [IMPLEMENTATION_GUIDE.md](file:///home/peng/workspace/mquickjs-rs-demo/deps/jidl-tool/doc/IMPLEMENTATION_GUIDE.md) - 与 Rust 实现的对应关系和代码生成机制
- [FEATURE_DEVELOPMENT_GUIDE.md](file:///home/peng/workspace/mquickjs-rs-demo/deps/jidl-tool/doc/FEATURE_DEVELOPMENT_GUIDE.md) - 如何开发和集成基于RIDL的Feature模块
- [TECH_SELECTION.md](file:///home/peng/workspace/mquickjs-rs-demo/deps/jidl-tool/doc/TECH_SELECTION.md) - jidl-tool的技术选型和实现计划
