# 嵌入式应用框架架构设计

## 概述

基于 mquickjs-rs + slint 的嵌入式应用框架架构设计。

**核心思想**：JS 用于 UI 开发，Rust 用于核心运行时，RIDL 用于接口定义。

**状态**：✅ 已确认

## 架构设计

### 分层架构

```
┌─────────────────────────────────────────────────────────────┐
│                    Layer 3: 应用层（JS）                     │
│  ┌──────────┐  ┌──────────────┐  ┌──────────────────────┐  │
│  │ UI 组件   │  │ 业务逻辑     │  │ 配置管理             │  │
│  └──────────┘  └──────────────┘  └──────────────────────┘  │
├─────────────────────────────────────────────────────────────┤
│                    Layer 2: UI 框架层（RIDL 接口）          │
│  ┌──────────┐  ┌──────────────┐  ┌──────────────────────┐  │
│  │ UI 框架   │  │ 路由系统     │  │ 状态管理             │  │
│  └──────────┘  └──────────────┘  └──────────────────────┘  │
├─────────────────────────────────────────────────────────────┤
│                    Layer 1: UI 渲染层（Rust + slint）       │
│  ┌──────────┐  ┌──────────────┐  ┌──────────────────────┐  │
│  │ JS 引擎   │  │ UI 渲染     │  │ 系统 API             │  │
│  │ (mquickjs)│  │ (slint)      │  │ (文件/网络/设备)     │  │
│  └──────────┘  └──────────────┘  └──────────────────────┘  │
├─────────────────────────────────────────────────────────────┤
│                    Layer 0: 核心运行时（Rust）               │
│  ┌──────────┐  ┌──────────────┐  ┌──────────────────────┐  │
│  │ JS 引擎   │  │ 类型系统     │  │ 异步机制             │  │
│  │ (mquickjs)│  │ (RIDL)       │  │ (AsyncStream)        │  │
│  └──────────┘  └──────────────┘  └──────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

### 核心设计原则

1. **JS 用于 UI 开发**：用户用 JS 编写 UI 组件和业务逻辑
2. **Rust 用于核心运行时**：引擎层、渲染层、系统 API
3. **RIDL 用于接口定义**：类型安全的接口定义
4. **分层架构**：不同抽象层，支持不同开发方式

## 各层设计

### Layer 0: 核心运行时（已完成）

```rust
// mquickjs-rs 已实现
- JS 引擎（mquickjs）
- 类型系统（RIDL）
- 异步机制（AsyncStream）
- GC 集成（Root/Traced）
```

### Layer 1: UI 渲染层

```rust
// Rust 实现 UI 渲染
pub struct UIRenderer {
    slint_app: slint::App,
}

impl UIRenderer {
    pub fn create_element(&self, type_name: &str, props: &HashMap<String, String>) -> ElementId;
    pub fn set_property(&self, element_id: ElementId, key: &str, value: &str);
    pub fn append_child(&self, parent_id: ElementId, child_id: ElementId);
    pub fn add_event_listener(&self, element_id: ElementId, event: &str, callback: Callback);
    pub fn render(&self);
}
```

### Layer 2: UI 框架层（RIDL 接口）

```typescript
// RIDL 接口定义
singleton UI {
    // 创建元素
    fn createElement(type: string, props: object) -> i32;
    
    // 设置属性
    fn setProperty(elementId: i32, key: string, value: string) -> void;
    
    // 添加子元素
    fn appendChild(parentId: i32, childId: i32) -> void;
    
    // 添加事件监听
    fn addEventListener(elementId: i32, event: string, callback: callback(args: string)) -> void;
    
    // 渲染
    fn render() -> void;
}

// 组件接口
interface Component {
    fn render() -> i32;
    fn onMount() -> void;
    fn onUpdate() -> void;
    fn onUnmount() -> void;
}
```

### Layer 3: 应用层（JS）

```javascript
// JS 开发 UI
class MyComponent {
    constructor() {
        this.state = { count: 0 };
    }
    
    render() {
        // 创建 UI 元素
        const container = UI.createElement("div", { class: "container" });
        const title = UI.createElement("h1", {});
        UI.setProperty(title, "text", "Hello from JS!");
        
        const button = UI.createElement("button", {});
        UI.setProperty(button, "text", "Click me");
        UI.addEventListener(button, "click", () => {
            this.state.count++;
            this.update();
        });
        
        const counter = UI.createElement("span", {});
        UI.setProperty(counter, "text", `Count: ${this.state.count}`);
        
        // 构建 UI 树
        UI.appendChild(container, title);
        UI.appendChild(container, button);
        UI.appendChild(container, counter);
        
        return container;
    }
    
    update() {
        // 更新 UI
        UI.render();
    }
}

// 应用入口
const app = new MyComponent();
app.render();
```

## 用户开发方式

### 方式 1：纯 JS 开发（推荐）

```javascript
// 完全用 JS 开发 UI
class MyApp {
    render() {
        const div = UI.createElement("div", {});
        UI.setProperty(div, "text", "Hello");
        return div;
    }
}
```

### 方式 2：JS + RIDL 混合开发

```typescript
// RIDL 定义接口
interface MyComponent {
    fn render() -> i32;
    fn onClick() -> void;
}

// JS 实现
class MyComponentImpl {
    render() {
        // ...
    }
    
    onClick() {
        // ...
    }
}
```

### 方式 3：Rust 扩展

```rust
// Rust 实现系统 API
pub struct FileSystem;

impl FileSystem {
    pub fn read_file(&self, path: &str) -> String {
        // 实现文件读取
    }
}
```

## 实现阶段

### 阶段 1：核心运行时（已完成）
- mquickjs-rs 核心引擎
- RIDL 工具链
- 异步机制

### 阶段 2：UI 渲染层（当前）
- slint 集成
- UI 接口定义（RIDL）
- 基础 UI 元素

### 阶段 3：UI 框架层（未来）
- 组件模型
- 响应式绑定
- 声明式语法

### 阶段 4：应用框架（未来）
- 路由系统
- 状态管理
- 生命周期管理

## 技术栈

| 层 | 技术 | 说明 |
|----|------|------|
| Layer 0 | mquickjs-rs | 核心运行时 |
| Layer 1 | slint | UI 渲染引擎 |
| Layer 2 | RIDL | 接口定义 |
| Layer 3 | JS | 应用开发 |

## 优势

| 优势 | 说明 |
|------|------|
| **JS 用于 UI 开发** | 开发效率高，热更新 |
| **Rust 用于核心** | 性能好，类型安全 |
| **RIDL 用于接口** | 类型安全，编译时校验 |
| **分层架构** | 不同抽象层，灵活 |
| **轻量级** | 适合嵌入式设备 |

## 与快应用规范的区别

| 维度 | 快应用规范 | 我们的设计 |
|------|------------|------------|
| **UI 开发语言** | JS | JS + Rust（可选） |
| **渲染引擎** | 原生 | slint（Rust 原生） |
| **类型安全** | 无 | RIDL 校验 |
| **热更新** | 支持 | 支持 |
| **性能** | 中等 | 高（Rust 运行时） |
| **完整性** | 不完整 | 完整（4层架构） |
| **多维度** | 单一 | 多维度（不同抽象层） |