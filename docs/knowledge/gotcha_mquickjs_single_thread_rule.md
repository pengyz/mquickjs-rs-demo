---
name: mquickjs-single-thread-rule
description: mquickjs JS 状态单线程铁律——异步必须用 libuv 模型（线程池+完成队列+drain），禁止 worker 线程触碰 JS
type: gotcha
created: 2026-09-04
sources: [deps/mquickjs/mquickjs.c JS_Call/JS_PushArg, async_bridge.rs 线程问题]
---

## JS 引擎单线程铁律

mquickjs 的 JSContext 状态（调用栈 `ctx->sp`、堆、GC、调用递归计数）**无锁保护**，
只能在创建它的线程上操作。worker 线程触碰 JSValue/JS_Call/GC = 数据竞争 UB。

## 异步的正确模型（分层）

### Layer 1：线程内异步（默认，零线程风险）
```
submit(job) → job 队列 → drain_completed_jobs(ctx) → JS_Call(callback)
```
work 在 JS 线程执行（必须非阻塞/快速）。适用：延迟回调、
非阻塞 I/O 集成、状态机分步。不依赖线程，适配 RTOS/单核 MCU。

### Layer 2：线程池 offload（显式 @offload）
```
submit(work) → threadpool → 完成队列(Mutex) → drain → JS_Call
```
work 在 worker 线程执行。适用：CPU 密集、仅阻塞 API 的操作。

**两层共用**：完成队列 + drain API + Root 生命周期 + 装饰器语义。
callback 永远只在 JS 主线程的安全点（drain）被调用。

### 线程安全规则

| 资源 | JS 主线程 | Worker |
|---|---|---|
| JSContext/JSValue/GC | ✅ 唯一 | ❌ |
| Root<T> 管理 | ✅ | ❌（不跨线程，只传 task_id） |
| 工作闭包/结果 | drain 接收 | ✅（须 Send + 'static） |

### 关键 API

- `JS_PushArg`（栈式压参，注意逆序：先 args 后 func 后 this）+ `JS_Call(ctx, n)` — 主线程调用 JS 函数
- 宿主需定期调用 `drain_completed_jobs(ctx)`（类似 uv_run / JS_ExecutePendingJob）

### nonCancellable 线程语义

- 可取消：context drop → 丢弃待处理完成项 + 释放 Root；worker 结果作废
- @nonCancellable：context drop → **join worker**（落盘必须完成），callback 跳过
- @timeout：drain 忽略超时后到达的结果

### 反面教材（2026-09-04 发现）

async_bridge.rs v1：worker 线程直接 `callback(result)`（线程不安全）+
`noop_waker` 忙轮询 Future（真 I/O 死循环）→ 已废弃。
重做为分层模型：默认线程内 job，`@offload` 显式选择线程池。
