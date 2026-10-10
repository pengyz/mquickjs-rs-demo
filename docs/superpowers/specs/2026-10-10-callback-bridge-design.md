# C3 设计：RIDL 同步回调桥（many-shot callback）

> 状态：**已完成 2026-10-10**（切片 1 c1b8c2f / 切片 2 095d4d9 / 切片 3 6f47714；
> ridl-tool 435/0、workspace 635/0、语料 33/33）。实现中的两个额外解析修复
> （grammar callback 关键字、parse_type callback_type 早返回）见
> docs/knowledge/pattern_ridl_callback_bridge.md。
> 日期：2026-10-10
> 背景：对抗复核 C3 发现 RIDL 回调 codegen 是 no-op stub（空 AsyncCallback
> 闭包，无人调用）。本设计把回调做成真能力——这是 LVGL 绑定（生态方案阶段
> B）的前置，也是 SDK 的通用能力缺口（任何"C 事件 → JS 函数"场景）。

## 1. 目标与非目标

**目标**：RIDL 方法参数中的 callback 类型获得**同步、多次触发（many-shot）、
异常受控**的真实实现：JS 传入函数 → 被 GC 安全持有 → C/Rust 侧凭句柄多次
调用 → 结果与异常可控。

**非目标**：
- 不改 async bridge（fire-and-forget 异步完成通知是另一个正交能力，保留）
- v1 不支持回调返回值（grammar 的 callback_def 本就无返回类型，与事件语义一致）
- v1 不做跨线程触发（mquickjs 单线程约束；跨线程入队属 async bridge 领域）

## 2. 语义模型（对齐对抗复核要求的形状）

```
JS 侧:  obj.setOnClick(function(ev) { ... });     ← 传函数
         ── glue 提取: JS_IsFunction 校验 → Local<Function>
         ── 注册: CallbackRegistry.register(fn) → CallbackHandle(u32)
         │   （函数值经 JSGCRef 持有——复用 Traced 体系，GC 安全）
         ── Rust impl 收到: fn set_on_click(&self, cb: CallbackHandle)
         ── impl 把 handle 交给 C 侧（作为 user_data / 存入 C 结构体）

C 侧:   事件触发 → mqjs_callback_invoke(ctx, handle)   ← 生成的 C trampoline
         ── CallbackRegistry.invoke(handle): 取出持有的 JS 函数
         ── 转换参数 → JS_Call → 异常捕获（上报 stderr/回调错误策略，不穿透 C）
         ── 多次触发 = 多次 invoke（many-shot）

注销:   handle.drop()（显式）或 Context 销毁时整表清理
         （JSGCRef 释放 → 函数可被 GC）
```

**关键决策（推荐即默认）**：

| 决策点 | 推荐 | 理由 |
|--------|------|------|
| 调用时机 | **同步**（C 事件点直接跑 JS） | LVGL 事件语义；单线程下无竞态；异步桥保留给真异步 |
| Rust impl 收到什么 | **CallbackHandle(u32)** + registry invoke API | 句柄可存可传（LVGL user_data 形状）；直接闭包无法跨 C 边界 |
| 持有机制 | **JSGCRef（复用 Traced/Root 体系）** | 引擎自动标记+重定位；与 GC 终态架构一致 |
| 异常策略 | invoke 返回 Result；异常打印并清除，不穿透 C | C 栈上不能让 JS 异常长跳穿透；错误可见 |
| v1 参数类型 | 回调参数支持基础类型（i32/f64/string/bool/object） | 覆盖 LVGL 事件（event code i32、目标句柄）；复杂类型后续 |
| C trampoline | 按回调类型生成 `mqjs_cb_<name>_invoke(ctx, handle, ...)` | LVGL user_data 传 handle，C 侧一行调用 |

## 3. 实现切片（TDD）

### 切片 1：SDK CallbackRegistry（deps/mquickjs-rs/src/callbacks.rs，新模块）
- `CallbackRegistry::new(ctx)` / `register(Local<Function>) -> CallbackHandle`
  / `invoke(handle, &[JSValue]) -> Result<(), CallbackError>` / `unregister(handle)`
- 内部：Vec<Option<JSGCRef>> 槽位表（复用 traced.rs 的 JSGCRef 封装）+ 空槽
  复用；ctx 销毁时整表 Drop（JSGCRef 自动释放）
- 单元测试：注册→GC 不回收→invoke 触发 JS 副作用→unregister→GC 回收；
  invoke 不存在的 handle → Err；异常 JS 函数 → Err 且 ctx 无残留异常

### 切片 2：RIDL codegen（filters.rs 替换 stub + C trampoline 模板）
- 参数提取：替换空闭包——`CallbackHandle` 提取（IsFunction 校验保留，
  注册进 registry，handle 传给 impl）
- impl 方法签名：callback 参数生成 `cb: CallbackHandle`
- C 侧：为每个 callback_def 生成 trampoline 声明+实现（args 转换链复用
  现有 emit 参数提取逻辑的反向）
- ridl-tool 渲染测试：断言生成的 glue 含 handle 提取、无空闭包残留；
  trampoline C 文本断言
- **生成物真编译测试**（GC 计划的教训）：渲染产物进临时 crate cargo build

### 切片 3：端到端 RIDL 模块（tests/global/callback_bridge/）
- RIDL：`class button { opaque {...} fn setOnClick(cb: callback(i32)); }`
- Rust impl：存 handle，暴露 `fire(i32)`（模拟 C 事件触发 → invoke）
- JS 测试：传函数 → 触发两次（many-shot 断言）→ JS 侧计数 == 2；
  传非函数 → SyntaxError/TypeError；异常回调 → 不崩、可继续
- 并入现有哨兵协议（可选第 5 哨兵）与 openvela 构建验证

## 4. 验收标准

- [ ] 切片 1-3 测试全绿；workspace 618+ 全绿无回归
- [ ] openvela 侧构建通过（sim Make），新能力对现有四哨兵零影响
- [ ] 生成代码无空闭包残留；trampoline 有真编译验证
- [ ] 知识沉淀：callbacks.rs 模式 + 与 async bridge 的边界（decision 条目）

## 5. 风险

| 风险 | 对策 |
|------|------|
| JSGCRef 槽位与 GC 压缩交互（重定位正确性） | 复用已验证的 Traced 封装；切片 1 单测含压缩场景 |
| codegen 的 args 反向转换链复杂度 | v1 收窄到基础类型；复杂类型报明确"unsupported"错误 |
| LVGL 真实集成缺口（lifetime 失效等） | 本设计只做桥；LVGL 对象失效属阶段 B 范围，不混入 |
