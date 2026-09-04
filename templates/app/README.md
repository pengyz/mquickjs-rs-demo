# {{project-name}}

基于 mquickjs-rs 的 JavaScript 引擎应用。

## 快速开始

### 1. 构建

```bash
just build
```

### 2. 运行测试

```bash
just test
```

## 项目结构

```
{{project-name}}/
├── Cargo.toml          # 项目配置
├── build.rs            # 构建脚本
├── mquickjs.ridl.toml  # RIDL 配置
├── src/
│   ├── lib.rs          # 库入口
│   ├── main.rs         # 应用入口
│   ├── my_service.ridl # RIDL 接口定义
│   └── my_service_impl.rs # Rust 实现
├── tests/
│   └── basic.js        # JS 测试
└── justfile            # 构建命令
```

## 添加新功能

### 1. 编辑 RIDL 文件

编辑 `src/my_service.ridl`，添加新方法：

```typescript
singleton MyService {
    fn hello(name: string) -> string;
    fn add(a: i32, b: i32) -> i32;
    readonly property version: string;
}
```

### 2. 实现 Rust 侧

编辑 `src/my_service_impl.rs`，实现新方法：

```rust
impl MyServiceSingleton for DefaultMyService {
    fn hello(&mut self, name: String) -> String {
        format!("Hello, {}!", name)
    }

    fn add(&mut self, a: i32, b: i32) -> i32 {
        a + b
    }

    fn version(&self) -> &str {
        &self.version
    }
}
```

### 3. 重新构建

```bash
just build
```

### 4. 测试

```bash
just test
```

## 更多信息

- [RIDL 语法参考](../../README.md#ridl-语法参考)
- [GC 系统](../../README.md#gc-系统)
- [AsyncStream](../../README.md#asyncstream--异步事件流)