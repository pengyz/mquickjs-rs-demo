---
name: ridl-async-cancellation-design
description: RIDL 异步任务取消语义设计——默认 nonCancellable，必须显式标记 cancellable
type: decision
created: 2026-09-04
sources: [conversation 2026-09-04]
---

## RIDL 异步取消语义

**核心决策**：默认 `@nonCancellable`，用户必须显式标记 `@cancellable`。

### 装饰器语法

```typescript
// 默认：不可取消（无需装饰器）
fn saveData(data: string, cb: AsyncCallback) -> void;

// 显式标记：可取消
@cancellable
fn fetch(url: string, cb: AsyncCallback) -> void;

// 超时取消
@timeout(5000)
fn updateCache(key: string, cb: AsyncCallback) -> void;
```

### 语义

| 装饰器 | 行为 |
|---|---|
| （无） | `@nonCancellable`：必须完成，即使 context drop |
| `@cancellable` | 可取消：context drop 时立即取消 |
| `@timeout(ms)` | 超时后自动取消 |

### 设计哲学

- **取消是关键决策**：不允许偷懒，必须逐方法显式声明
- **默认安全**：无装饰器 = 不可取消，防止数据丢失
- **无块级配置**：不提供模块级/块级默认覆盖，避免隐式行为
- **用户责任**：标记 `@cancellable` 意味着用户理解并接受取消风险

### 参考

- Kotlin: `NonCancellable` 显式块
- Swift: 默认可取消，显式优先级
- Rust/Tokio: 默认可取消，显式 `CancellationToken`
- **本项目**: 默认不可取消，显式 `@cancellable`（反主流，但适合嵌入式数据安全场景）
