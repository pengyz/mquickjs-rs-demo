---
name: ridl-async-cancellation-design
description: RIDL 异步任务取消语义设计——默认可取消，必须显式标记 nonCancellable
type: decision
created: 2026-09-04
updated: 2026-09-04
sources: [conversation 2026-09-04]
---

## RIDL 异步取消语义

**核心决策**：默认可取消，用户必须显式标记 `@nonCancellable`。

### 装饰器语法

```typescript
// 默认：可取消（无需装饰器）
fn fetch(url: string, cb: AsyncCallback) -> void;

// 显式标记：不可取消
@nonCancellable
fn saveData(data: string, cb: AsyncCallback) -> void;

// 超时取消
@timeout(5000)
fn updateCache(key: string, cb: AsyncCallback) -> void;
```

### 语义

| 装饰器 | 行为 |
|---|---|
| （无） | 可取消：context drop 时立即取消 |
| `@nonCancellable` | 不可取消：必须完成，即使 context drop |
| `@timeout(ms)` | 超时后自动取消 |

### 设计哲学

- **默认可取消**：与主流框架一致（Kotlin、Swift、Rust、C#、Go）
- **显式不可取消**：`@nonCancellable` 必须手动标记，明确承担资源占用风险
- **无块级配置**：不提供模块级/块级默认覆盖，避免隐式行为
- **用户责任**：标记 `@nonCancellable` 意味着任务必须完成，可能阻塞 context 销毁

### 参考

- Kotlin: 默认可取消，`NonCancellable` 显式块
- Swift: 默认可取消，显式优先级
- Rust/Tokio: 默认可取消，显式 `CancellationToken`
- Go: 默认可取消，`context.Background()` 显式不可取消
- **本项目**: 默认可取消，`@nonCancellable` 显式标记（与主流一致）
