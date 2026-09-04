# RIDL 异步任务使用指南

## 概述

RIDL 支持异步任务取消语义，通过装饰器标记任务的取消行为。默认所有异步任务**可取消**，开发者必须显式标记不可取消的任务。

## 装饰器语法

```typescript
callback AsyncCallback(error: string?, data: string?);

class MyService {
    // 默认：可取消（无需装饰器）
    fn fetchData(url: string, cb: AsyncCallback) -> void;
    
    // 显式标记：不可取消
    @nonCancellable
    fn saveToDisk(data: string, cb: AsyncCallback) -> void;
    
    // 超时取消
    @timeout(5000)
    fn updateCache(key: string, cb: AsyncCallback) -> void;
}
```

## 装饰器说明

| 装饰器 | 行为 | 使用场景 |
|---|---|---|
| （无） | 可取消：context drop 时立即取消 | 网络请求、数据获取 |
| `@nonCancellable` | 不可取消：必须完成 | 数据落盘、事务提交 |
| `@timeout(ms)` | 超时后自动取消 | 缓存更新、非关键操作 |

## 设计原则

- **默认可取消**：与主流框架一致（Kotlin、Swift、Rust、C#、Go）
- **显式不可取消**：`@nonCancellable` 必须手动标记，明确承担资源占用风险
- **无隐式行为**：不提供块级/模块级默认覆盖

## 生命周期管理

### Context Drop 时的行为

```
Context Drop
    ↓
┌─────────────────────────────────────┐
│ 1. 标记 context 为 dropping         │
│ 2. 取消所有 Cancellable 任务        │
│ 3. 等待 NonCancellable 任务完成     │
│ 4. 超时任务按 timeout 处理          │
│ 5. 释放所有 Root<T>                 │
│ 6. 调用 JS_FreeContext              │
└─────────────────────────────────────┘
```

### Root<T> 生命周期

- 任务启动时：Root<T> 保持 callback 存活
- 任务完成时：释放 Root<T>，允许 GC 回收 callback
- 任务取消时：释放 Root<T>
- Context drop 时：释放所有 Root<T>

## Rust 侧实现

### 基本用法

```rust
use mquickjs_rs::async_bridge::{AsyncBridge, js_async};

fn fetch(&mut self, ctx: &Context, url: String, cb: Root<Value>) {
    let bridge = ctx.async_bridge();
    bridge.spawn_cancellable(cb, async move {
        let result = http_get(&url).await;
        result_to_js(result)
    });
}
```

### 不可取消任务

```rust
fn save_to_disk(&mut self, ctx: &Context, data: String, cb: Root<Value>) {
    let bridge = ctx.async_bridge();
    bridge.spawn_non_cancellable(cb, async move {
        write_to_disk(&data).await;
        JS_TRUE
    });
}
```

### 超时任务

```rust
fn update_cache(&mut self, ctx: &Context, key: String, cb: Root<Value>) {
    let bridge = ctx.async_bridge();
    bridge.spawn_with_timeout(cb, Duration::from_millis(5000), async move {
        update_cache_impl(&key).await;
        JS_TRUE
    });
}
```

### 使用宏

```rust
fn fetch(&mut self, ctx: &Context, url: String, cb: Root<Value>) {
    js_async!(ctx, cb, http_get(&url));
}
```

## 错误处理

### 取消时的处理

```rust
bridge.spawn_cancellable(cb, async move {
    tokio::select! {
        result = http_get(&url) => result_to_js(result),
        _ = cancel_token.cancelled() => {
            // 取消时返回 null 或错误
            JS_NULL
        }
    }
});
```

### 超时时的处理

```rust
bridge.spawn_with_timeout(cb, Duration::from_millis(5000), async move {
    // 超时后自动取消，callback 不会被调用
    update_cache_impl(&key).await;
    JS_TRUE
});
```

## 最佳实践

1. **默认可取消**：大多数异步任务应该是可取消的
2. **显式标记关键任务**：数据落盘、事务提交用 `@nonCancellable`
3. **设置合理超时**：避免任务无限期运行
4. **处理取消信号**：在 Future 中检查取消状态
5. **资源清理**：在 Drop 实现中释放资源

## 限制

- 异步任务必须在 Context 存活时完成
- NonCancellable 任务可能阻塞 Context 销毁
- 超时任务的超时是近似的（依赖 tokio 定时器精度）
- 当前不支持任务优先级（所有任务同等优先级）
