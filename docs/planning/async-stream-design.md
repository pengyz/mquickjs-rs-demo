# AsyncStream 设计文档

## 概述

AsyncStream 是 mquickjs 异步任务的事件流机制，统一管理一次性 callback 和多次 callback 的生命周期。

**状态**：✅ 已实现并测试通过

## 核心设计原则

1. **线程安全**：emit() 只能在 JS 主线程调用，通过完成队列间接触发
2. **自动清理**：Subscription 实现 Drop，自动释放 Root
3. **类型安全**：泛型 T 确保事件类型正确
4. **语义清晰**：统一为 AsyncStream，废弃 AsyncResult

## 类型定义

### Rust 侧

```rust
/// 订阅句柄 - 管理 callback 生命周期
pub struct Subscription {
    id: u64,
    // Drop 时自动释放 Root
}

impl Drop for Subscription {
    fn drop(&mut self) {
        // 从 EventEmitter 中移除 subscriber
        // 释放 Root
    }
}

/// 事件发射器
pub struct AsyncStream<T: Send + 'static> {
    subscribers: Vec<SubscriberEntry<T>>,
    next_id: u64,
}

struct SubscriberEntry<T> {
    id: u64,
    callback: Root<Function>,
    _marker: PhantomData<T>,
}

impl<T: Send + 'static> AsyncStream<T> {
    /// 订阅（只能在 JS 主线程调用）
    /// 
    /// # Safety
    /// 必须在 JS 主线程调用
    pub unsafe fn subscribe(&mut self, cb: Root<Function>) -> Subscription {
        let id = self.next_id;
        self.next_id += 1;
        
        self.subscribers.push(SubscriberEntry {
            id,
            callback: cb,
            _marker: PhantomData,
        });
        
        Subscription { id }
    }
    
    /// 发射事件（只能在 JS 主线程调用）
    /// 
    /// # Safety
    /// 必须在 JS 主线程调用
    pub unsafe fn emit(&self, value: &T) {
        for entry in &self.subscribers {
            // 将 T 转换为 JSValue
            let js_value = value.to_js();
            // 调用 callback
            self.call_callback(&entry.callback, js_value);
        }
    }
    
    /// 取消所有订阅
    pub fn clear(&mut self) {
        self.subscribers.clear();
    }
    
    /// 移除指定订阅
    fn remove(&mut self, id: u64) {
        self.subscribers.retain(|entry| entry.id != id);
    }
}

/// 一次性 callback 的类型别名
pub type AsyncCallback<T> = AsyncStream<T>;
```

### JS 侧（IDL 语法）

```typescript
/// 一次性事件流（完成即结束）
type AsyncResult<T> = AsyncStream<T>;

/// 多次事件流
type AsyncStream<T> = {
    subscribe(cb: (value: T) => void): Subscription;
};

/// 订阅句柄
type Subscription = {
    unsubscribe(): void;
};
```

## IDL 语法映射

### 基本用法

```typescript
class HttpClient {
    // 一次性事件流（完成即结束）
    fn fetch(url: string, result: AsyncStream<Response>) -> Subscription;
    
    // 多次事件流
    fn download(url: string, progress: AsyncStream<Progress>) -> Subscription;
    
    // 混合：结果 + 进度
    fn upload(
        data: buffer,
        result: AsyncStream<Response>,
        progress: AsyncStream<Progress>
    ) -> Subscription;
}
```

### nonCancellable 约束

```typescript
class FileService {
    // nonCancellable 只能用一次性 callback（语法层面禁止 AsyncStream）
    @nonCancellable
    fn save(data: string, cb: AsyncCallback<void>) -> void;
    
    // 可取消任务可以用 AsyncStream
    fn download(url: string, progress: AsyncStream<Progress>) -> Subscription;
}
```

## 线程安全模型

### 线程约束

| 操作 | 线程要求 | 说明 |
|------|----------|------|
| subscribe() | JS 主线程 | 创建 Root，访问 JS 状态 |
| emit() | JS 主线程 | 调用 callback，访问 JS 状态 |
| unsubscribe() | 任意线程 | 只修改 Rust 数据结构 |
| clear() | 任意线程 | 只修改 Rust 数据结构 |

### Worker 线程触发事件

```rust
// Worker 线程不能直接调用 emit()
// 必须通过完成队列间接触发

// Worker 线程
fn worker_task(stream_id: u64, value: T) {
    // 将事件入队
    completions.push(EventCompletion {
        stream_id,
        value: AsyncValue::from(value),
    });
}

// JS 主线程 drain
fn drain_events(ctx: *mut JSContext) {
    while let Some(completion) = completions.pop() {
        // 还原 AsyncValue 为 JSValue
        let js_value = completion.value.to_js(ctx);
        
        // 找到对应的 AsyncStream
        if let Some(stream) = streams.get(&completion.stream_id) {
            // 在 JS 主线程调用 emit
            unsafe { stream.emit(&js_value); }
        }
    }
}
```

## 生命周期管理

### Subscription 生命周期

```
subscribe() → Subscription { id }
    ↓
[事件发射中...]
    ↓
unsubscribe() 或 Drop → 从 AsyncStream 中移除 → 释放 Root
```

### 自动清理机制

```rust
// Rust 侧：Subscription Drop 自动清理
impl Drop for Subscription {
    fn drop(&mut self) {
        // 从全局 registry 中移除
        // 释放 Root
    }
}

// JS 侧：FinalizationRegistry 兜底
const registry = new FinalizationRegistry((subscription) => {
    subscription.unsubscribe();
});
```

### Context Drop 清理

```rust
// Context drop 时清理所有 AsyncStream
impl Context {
    fn drop(&mut self) {
        // 清理所有 AsyncStream
        for stream in self.streams.values_mut() {
            stream.clear();
        }
        
        // 清理所有 Subscription
        for subscription in self.subscriptions.values() {
            subscription.unsubscribe();
        }
    }
}
```

## 错误处理

### 错误传播策略

```typescript
class ErrorHandler {
    // 方式 1：fail-fast（默认）
    fn onData(handler: AsyncStream<Data>) -> Subscription;
    // 一个 subscriber 异常，停止所有
    
    // 方式 2：onError 回调
    fn onData(
        handler: AsyncStream<Data>,
        onError: AsyncStream<Error>
    ) -> Subscription;
    // 异常发送到 onError，继续执行其他 subscriber
}
```

### 实现

```rust
impl<T: Send + 'static> AsyncStream<T> {
    pub unsafe fn emit(&self, value: &T) {
        let mut has_error = false;
        
        for entry in &self.subscribers {
            if has_error {
                // 如果已经有异常，跳过后续 subscriber
                continue;
            }
            
            match self.call_callback(&entry.callback, value) {
                Ok(_) => {},
                Err(e) => {
                    // 记录异常，停止后续 subscriber
                    has_error = true;
                    // 可选：发送到 onError stream
                }
            }
        }
    }
}
```

## 边界情况处理

### 1. subscribe 后立即 unsubscribe

```typescript
const sub = service.onData(handler);
sub.unsubscribe();  // 此时可能已经有事件在队列中等待

// 语义：队列中的事件不会触发 handler
// 因为 unsubscribe 会从 AsyncStream 中移除 subscriber
```

### 2. emit() 时 subscriber 列表变化

```typescript
service.onData((data) => {
    // 在 callback 中 unsubscribe
    sub.unsubscribe();
    // 当前 emit 循环会怎样？
});

// 语义：当前 emit 继续执行，下次 emit 不再触发已 unsubscribe 的 subscriber
// 实现：emit 时复制 subscriber 列表，遍历副本
```

### 3. nonCancellable + AsyncStream

```typescript
@nonCancellable
fn save(data: string, progress: AsyncStream<Progress>) -> Subscription;

// 语义：语法层面禁止
// nonCancellable 任务只能用一次性 callback
// 编译时报错
```

### 4. Context drop 时正在 emit()

```rust
// 语义：正在 emit 的事件继续执行完毕
// 但不再触发新的事件
// 实现：使用 AtomicBool 标记 context 是否已 drop
```

## 性能优化

### 1. Vec 替代 HashMap

```rust
// 使用 Vec 而非 HashMap
// 顺序遍历更快，内存局部性更好
pub struct AsyncStream<T> {
    subscribers: Vec<SubscriberEntry<T>>,
}
```

### 2. 批量 emit

```rust
// 一次 drain 多个事件
fn drain_events(&mut self, ctx: *mut JSContext) {
    let mut events = Vec::new();
    
    // 批量取出事件
    while let Some(event) = self.completions.pop() {
        events.push(event);
    }
    
    // 批量 emit
    for event in events {
        self.emit_event(ctx, event);
    }
}
```

### 3. 限制最大 subscriber 数量

```rust
const MAX_SUBSCRIBERS: usize = 1000;

pub unsafe fn subscribe(&mut self, cb: Root<Function>) -> Result<Subscription, Error> {
    if self.subscribers.len() >= MAX_SUBSCRIBERS {
        return Err(Error::TooManySubscribers);
    }
    // ...
}
```

## 代码生成示例

### IDL

```typescript
class HttpClient {
    fn fetch(url: string, result: AsyncStream<Response>) -> Subscription;
}
```

### 生成的 Rust 代码

```rust
// glue 生成
fn fetch(scope: &Scope, args: &[Local<Value>]) -> JSValue {
    // 1. 提取参数
    let url = args[0].to_string();
    let result_stream = args[1];  // AsyncStream<Response>
    
    // 2. 提取 callback
    let cb_root = Root::new(scope, result_stream);
    
    // 3. 创建 Subscription
    let subscription = Subscription::new(cb_root);
    
    // 4. 提交任务
    let task_manager = scope.context().async_task_manager();
    task_manager.submit_cancellable(move || {
        // 5. 异步执行
        let response = do_http_request(url);
        
        // 6. 返回结果（通过完成队列）
        AsyncValue::from(response)
    }, subscription.id());
    
    // 7. 返回 Subscription 句柄
    subscription.to_js()
}
```

## 测试策略

### 单元测试

```rust
#[test]
fn test_async_stream_subscribe_emit() {
    let stream = AsyncStream::<i32>::new();
    let cb = create_callback();
    
    unsafe {
        let sub = stream.subscribe(cb);
        stream.emit(&42);
        sub.unsubscribe();
    }
}

#[test]
fn test_async_stream_auto_cleanup() {
    let stream = AsyncStream::<i32>::new();
    let cb = create_callback();
    
    {
        let sub = stream.subscribe(cb);
        // sub 在这里 drop
    }
    
    // subscriber 应该已经被移除
    assert_eq!(stream.subscriber_count(), 0);
}
```

### 集成测试

```rust
#[test]
fn test_async_stream_with_task() {
    let mut ctx = Context::new(1024 * 1024).unwrap();
    
    ctx.eval(r#"
        let results = [];
        const sub = service.fetch('http://example.com', (response) => {
            results.push(response);
        });
        
        // 等待任务完成
        setTimeout(() => {
            assert(results.length > 0);
            sub.unsubscribe();
        }, 100);
    "#).unwrap();
}
```

## 实现计划

### Phase 1: 基础 AsyncStream
- [ ] 定义 AsyncStream<T> 结构
- [ ] 实现 subscribe/unsubscribe
- [ ] 实现 emit（单线程版本）
- [ ] 单元测试

### Phase 2: 线程安全
- [ ] 实现完成队列
- [ ] 实现 drain_events
- [ ] Worker 线程触发事件
- [ ] 集成测试

### Phase 3: 生命周期管理
- [ ] Subscription Drop 实现
- [ ] Context drop 清理
- [ ] FinalizationRegistry 兜底
- [ ] 边界情况测试

### Phase 4: 错误处理
- [ ] fail-fast 策略
- [ ] onError 回调
- [ ] 异常传播测试

### Phase 5: 性能优化
- [ ] Vec 替代 HashMap
- [ ] 批量 emit
- [ ] 性能测试

### Phase 6: IDL 集成
- [ ] 更新 IDL 语法
- [ ] 更新代码生成
- [ ] 端到端测试

## 验证命令

```bash
cargo test -p mquickjs-rs async_stream
cargo test -p ridl-tool async_stream_codegen
cargo run -- tests
```