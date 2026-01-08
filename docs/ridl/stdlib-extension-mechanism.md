# 标准库扩展机制详解

## 概述

本项目通过修改 mquickjs 的标准库注册机制，将 RIDL（Rust Interface Description Language）定义的接口注册到 JS 环境中。我们通过自定义的构建流程生成 `mqjs_ridl_stdlib` 工具，该工具负责将 RIDL 定义的接口编译为 mquickjs 可用的标准库头文件。

## 核心原理

mquickjs 通过 C 语言的结构体数组定义标准库函数和对象。其核心实现依赖于 `JSPropDef` 结构体数组，该数组定义了所有全局对象、函数和属性。通过构建时代码生成，将 RIDL 定义转换为相应的 C 结构体数组，从而实现接口的静态注册。

## 详细流程

### 1. RIDL 定义
- RIDL 文件定义了 JS 接口的签名和行为
- 通过 `ridl-tool` 解析 RIDL 文件，生成对应的 Rust 胶水代码（`module_name_glue.rs`）
- 开发者根据RIDL定义手动实现具体功能（`module_name_impl.rs`）

### 2. 模板文件生成
- 以 `mqjs_stdlib.c` 为基础创建 `mqjs_stdlib_template.c`
- 在模板文件中预留 `JS_RIDL_EXTENSIONS` 宏位置
- 保留 mquickjs 原有的标准库功能，仅添加 RIDL 扩展点

### 3. RIDL 扩展定义
- `ridl-tool` 生成 `mquickjs_ridl_register.h` 头文件
- 该文件定义了 `JS_RIDL_EXTENSIONS` 宏，包含所有 RIDL 定义的接口

#### 聚合输入来源（registry 驱动）

`mquickjs_ridl_register.h` 属于“聚合头文件”，其输入是一组 RIDL 文件列表。
当前实现中：

- **RIDL 清单由 `ridl-modules/registry` 提供**：registry 的 `build.rs` 会解析 `Cargo.toml` 中的 `path` 依赖，筛选出 `src/` 下存在 `*.ridl` 的 crate 作为 RIDL module，然后把这些 `*.ridl` 的绝对路径写入 `$OUT_DIR/ridl_manifest.json`。
- registry 同时通过环境变量导出清单路径：`RIDL_REGISTRY_MANIFEST=$OUT_DIR/ridl_manifest.json`。
- **mquickjs 标准库生成由 `deps/mquickjs-rs` 负责**：`deps/mquickjs-rs/build.rs` 会读取 `RIDL_REGISTRY_MANIFEST`，并调用 `ridl-tool aggregate` 生成：
  - `deps/mquickjs-rs/generated/mquickjs_ridl_register.h`
  - `deps/mquickjs-rs/generated/ridl_symbols.rs`

这样新增模块时只需要在 registry 的 `Cargo.toml` 添加 path 依赖即可被纳入聚合。

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