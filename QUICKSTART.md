# 快速入门指南

## 先决条件

### 必需

- **Rust 1.70+**（推荐使用 [rustup](https://rustup.rs/)）
- **C 编译器**（gcc 或 clang）
- **Git**（需要支持 submodule）

### 可选

- **libclang**（bindgen 需要，Ubuntu: `apt install libclang-dev`）
- **just**（命令运行器，`cargo install just`）

## 5 分钟快速开始

### 1. 克隆仓库

```bash
git clone --recurse-submodules <repo-url>
cd mquickjs-rs-demo
```

> ⚠️ **重要**：必须使用 `--recurse-submodules`，否则 C 引擎源码会缺失。

### 2. 检查环境

```bash
# 使用 doctor 脚本检查
bash scripts/doctor.sh
```

### 3. 构建

```bash
# 方式 1：使用 just（推荐）
just build

# 方式 2：手动执行
cargo run -p ridl-builder -- prepare
cargo build
```

### 4. 运行测试

```bash
# 方式 1：使用 just（推荐）
just test

# 方式 2：手动执行
cargo test -p ridl-tool
cargo test -p mquickjs-demo
cargo run -- tests
```

## 创建新应用

### 方式 1：使用模板（推荐）

```bash
# 安装 cargo-generate
cargo install cargo-generate

# 从模板创建
cargo generate --git <template-repo> --name my-app

# 进入项目目录
cd my-app

# 构建
just build
```

### 方式 2：手动创建

#### 步骤 1：创建应用目录

```bash
mkdir -p apps/my-app/src
cd apps/my-app
```

#### 步骤 2：创建 Cargo.toml

```toml
[package]
name = "my-app"
version = "0.1.0"
edition = "2024"

[dependencies]
mquickjs-rs = { path = "../../deps/mquickjs-rs" }
mquickjs-sys = { path = "../../deps/mquickjs-sys" }

[build-dependencies]
mquickjs-ridl-glue = { path = "../../deps/mquickjs-ridl-glue" }

[features]
default = ["ridl-extensions"]
ridl-extensions = ["mquickjs-rs/ridl-extensions"]
```

#### 步骤 3：创建 build.rs

```rust
fn main() {
    mquickjs_ridl_glue::emit();
}
```

#### 步骤 4：创建 mquickjs.ridl.toml

```toml
version = 1
app_id = "my_app"
```

#### 步骤 5：创建 RIDL 文件

```typescript
// src/my_module.ridl
module my.app@1.0.0;

singleton MyService {
    fn hello(name: string) -> string;
    readonly property version: string;
}
```

#### 步骤 6：创建实现文件

```rust
// src/my_module_impl.rs
use crate::api::MyServiceSingleton;

pub struct DefaultMyService {
    version: String,
}

impl DefaultMyService {
    pub fn new() -> Self {
        Self {
            version: "1.0.0".to_string(),
        }
    }
}

impl MyServiceSingleton for DefaultMyService {
    fn hello(&mut self, name: String) -> String {
        format!("Hello, {}!", name)
    }

    fn version(&self) -> &str {
        &self.version
    }
}

pub fn create_my_service_singleton() -> Box<dyn MyServiceSingleton> {
    Box::new(DefaultMyService::new())
}
```

#### 步骤 7：创建 lib.rs

```rust
mquickjs_rs::ridl_include_module!();

pub mod impls {
    pub use crate::api::MyServiceSingleton;
    pub use crate::my_module_impl::DefaultMyService;
    pub use crate::my_module_impl::create_my_service_singleton;
}

mod my_module_impl;
```

#### 步骤 8：添加到 workspace

编辑根目录 `Cargo.toml`，添加：

```toml
[workspace]
members = [
    # ... 其他 members ...
    "apps/my-app",
]

[dependencies.my_app]
path = "apps/my-app"
```

#### 步骤 9：构建

```bash
cd ../..  # 回到根目录
cargo run -p ridl-builder -- prepare
cargo build
```

#### 步骤 10：测试

创建 `apps/my-app/tests/basic.js`：

```javascript
(function () {
    if (typeof globalThis.MyService === "undefined") {
        throw new Error("expected globalThis.MyService singleton");
    }

    var result = MyService.hello("World");
    if (result !== "Hello, World!") {
        throw new Error("expected 'Hello, World!', got '" + result + "'");
    }

    console.log("MyService test passed");
})();
```

运行测试：

```bash
cargo run -- tests/apps/my-app
```

## 创建新 RIDL 模块

### 步骤 1：创建模块目录

```bash
mkdir -p tests/global/my_module/test_my_module/src
cd tests/global/my_module/test_my_module
```

### 步骤 2：创建 Cargo.toml

```toml
[package]
name = "test_my_module"
version = "0.1.0"
edition = "2024"

[dependencies]
mquickjs-rs = { path = "../../../../deps/mquickjs-rs", features = ["ridl-extensions"] }

[build-dependencies]
mquickjs-rs = { path = "../../../../deps/mquickjs-rs" }
```

### 步骤 3：创建 RIDL 文件

```typescript
// src/test_my_module.ridl
mode strict;

singleton MyModuleSingleton {
    fn doSomething(input: string) -> string;
}
```

### 步骤 4：创建实现文件

```rust
// src/singleton_impl.rs
use crate::api::MyModuleSingletonSingleton;

pub struct DefaultMyModuleSingleton;

impl MyModuleSingletonSingleton for DefaultMyModuleSingleton {
    fn do_something(&mut self, input: String) -> String {
        format!("Processed: {}", input)
    }
}

pub fn create_my_module_singleton_singleton() -> Box<dyn MyModuleSingletonSingleton> {
    Box::new(DefaultMyModuleSingleton)
}
```

### 步骤 5：创建 lib.rs

```rust
mquickjs_rs::ridl_include_module!();

pub mod impls {
    pub use crate::api::MyModuleSingletonSingleton;
    pub use crate::singleton_impl::DefaultMyModuleSingleton;
    pub use crate::singleton_impl::create_my_module_singleton_singleton;
}

mod singleton_impl;
```

### 步骤 6：创建 build.rs

```rust
fn main() {
    mquickjs_rs::ridl_build_helper::emit();
}
```

### 步骤 7：添加到 workspace

编辑根目录 `Cargo.toml`，添加：

```toml
[workspace]
members = [
    # ... 其他 members ...
    "tests/global/my_module/test_my_module",
]

[dependencies.test_my_module]
path = "tests/global/my_module/test_my_module"
```

### 步骤 8：构建和测试

```bash
cd ../../../..  # 回到根目录
cargo run -p ridl-builder -- prepare
cargo build
cargo run -- tests
```

## 常见问题

### Q: 编译失败，提示 "Missing mquickjs build outputs"

**A**: 需要先运行 `cargo run -p ridl-builder -- prepare`。

### Q: 编译失败，提示 "undefined symbol"

**A**: 可能是 submodule 没有初始化，运行：

```bash
git submodule update --init --recursive
```

### Q: 测试失败，提示 "expected globalThis.XXX singleton"

**A**: 可能是 RIDL 模块没有正确注册，检查：

1. 模块是否添加到 workspace members
2. 模块是否添加到 dependencies
3. 是否运行了 `cargo run -p ridl-builder -- prepare`

### Q: 如何调试 RIDL 生成的代码？

**A**: 生成的代码在 `target/debug/build/<crate-name>/out/` 目录下：

- `api.rs`：生成的 Rust API（trait 定义）
- `glue.rs`：生成的 FFI 绑定代码
- `ridl_context_ext.rs`：上下文扩展代码

## 下一步

- 阅读 [RIDL 语法参考](README.md#ridl-语法参考)
- 阅读 [GC 系统](README.md#gc-系统)
- 阅读 [AsyncStream](README.md#asyncstream--异步事件流)
- 查看 [示例项目](tests/)