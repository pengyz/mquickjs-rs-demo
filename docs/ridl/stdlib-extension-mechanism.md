# 标准库扩展机制详解

## 概述

本项目通过修改 mquickjs 的标准库注册机制，将 RIDL（Rust Interface Description Language）定义的接口注册到 JS 环境中。

关键约束：**注册必须发生在编译期**。C 侧 stdlib 表在编译时包含 `mquickjs_ridl_register.h` 并展开 RIDL 注入宏，因此无法在运行时动态注册。

## 核心原理

mquickjs 通过 C 语言的结构体数组定义标准库函数和对象。其核心实现依赖于 `JSPropDef` 结构体数组，该数组定义了所有全局对象、函数和属性。通过构建时代码生成，将 RIDL 定义转换为相应的 C 结构体数组，从而实现接口的静态注册。

### 为什么需要“多 Hook”而不是单宏

对于全局 singleton（例如 `globalThis.console`），完整注册需要同时具备：

1. **文件作用域定义**：属性表、对象定义（例如 `static const JSPropDef ...`、`JS_OBJECT_DEF(...)`）。
2. **全局表注入项**：插入到 `js_global_object[]` 的条目（例如 `JS_PROP_CLASS_DEF("console", &js_console_obj)`）。

因此 `mquickjs_ridl_register.h` 会提供两个注入点：

- `JS_RIDL_DECLS`：用于“文件作用域”的声明/定义（必须在 `js_global_object[]` 定义之前展开）。
- `JS_RIDL_GLOBAL_PROPS`：用于插入 `js_global_object[]` 的条目。

同时，为兼容旧写法，头文件仍提供：

- `JS_RIDL_EXTENSIONS`：目前等价于 `JS_RIDL_GLOBAL_PROPS`（向后兼容别名）。

## 详细流程

### 1. RIDL 定义
- RIDL 文件定义了 JS 接口的签名和行为
- 通过 `ridl-tool` 解析 RIDL 文件，生成对应的 Rust 胶水代码（例如 `<module>_glue.rs`）
- 开发者根据RIDL定义手动实现具体功能（`module_name_impl.rs`）

### 2. 模板文件集成（C 侧）

以 `deps/mquickjs-rs/mqjs_stdlib_template.c` 的模式为例：

```c
#include "mquickjs.h"
#include "mquickjs_ridl_register.h"

/* 1) 文件作用域：RIDL 扩展所需的 props/object defs */
JS_RIDL_DECLS;

static const JSPropDef js_global_object[] = {
    /* ... base stdlib entries ... */

    /* 2) 注入到 js_global_object[] 的条目 */
    JS_RIDL_EXTENSIONS
    /* 或者显式使用：JS_RIDL_GLOBAL_PROPS */

    JS_PROP_END,
};
```

> 备注：这里 `JS_RIDL_EXTENSIONS` 仍可用，是因为当前它只是 `JS_RIDL_GLOBAL_PROPS` 的兼容别名。

### 3. RIDL 扩展定义（生成头文件）
- `ridl-tool` 生成 `mquickjs_ridl_register.h` 头文件
- 该头文件提供 `JS_RIDL_DECLS` / `JS_RIDL_GLOBAL_PROPS`（以及兼容别名 `JS_RIDL_EXTENSIONS`）用于静态注入

#### 聚合输入来源（App manifest 驱动 / SoT）

`mquickjs_ridl_register.h` 属于“聚合头文件”，其输入是一组 RIDL 文件列表。
当前实现中：

- **RIDL modules 由 App manifest（根 `Cargo.toml` 的 `[dependencies]`）决定**：只有当依赖 crate 的 `src/` 下至少存在 1 个 `*.ridl` 文件时，该 crate 才会被视为 RIDL module。
- **App `build.rs` 负责生成聚合产物**（通过 `ridl-tool`）：
  - `$OUT_DIR/mquickjs_ridl_register.h`：供 C 侧编译期展开注入宏（`JS_RIDL_DECLS` / `JS_RIDL_GLOBAL_PROPS` / `JS_RIDL_EXTENSIONS`）
  - `$OUT_DIR/ridl_initialize.rs`：供 Rust 侧集中初始化（`mquickjs_rs::ridl_initialize!()`）
- **mquickjs-sys `build.rs`** 在启用 feature `ridl-extensions` 时，将上面的 `mquickjs_ridl_register.h` 纳入 QuickJS stdlib 编译，从而把扩展项静态编进 `libmquickjs.a`。

因此新增模块时，不需要修改 mquickjs-sys/mquickjs-rs 的 build.rs；只需要在最终 App 的 `Cargo.toml` 添加对应模块依赖即可。

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
```