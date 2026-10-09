# 专项调查：test_async_value_object 间歇性 SIGSEGV（P1）

> 状态：**已解决 2026-10-09**（根因不是 GC——见下" Resolution"；修复压测
> 50/50 零崩溃，达到本计划验收标准）

## Resolution（2026-10-09）

**根因**：`AsyncValue::to_js` 的 Json 路径调用 `JS_Call(ctx, 0)`——**零推参**。
JS_Call 从 ctx->sp 读完整调用帧（func_obj/this/args），栈上残留垃圾被当函数
帧解释：看似"GC 生命周期问题"实为**残缺的 C 调用约定**。崩与不崩取决于栈
残留内容（故 flaky、且孤立跑单测时垃圾恰好无害而通过）。创建的 JSON 参数
字符串也从未入栈（`let _js_json_str` 弃用）。

**修复**（async_value.rs to_js Json 路径）：
1. 按调用约定逆序推参：arg → func → this，`JS_Call(ctx, 1)`
2. 顺序调整：`JS_NewString`（分配可触发 GC）提前到属性取回之前——推上
   ctx 栈（被栈扫描保护）之前的裸值不可跨 GC，这是修复中顺带消除的真实
   GC 隐患
3. 测试从"只要不 panic"强化为**确定性往返断言**（to_js → from_js 等价），
   object 与 nested 两用例

**验收**：单测 13/13；全套件 50 轮零崩溃（原 ~10-17% 崩溃率）；workspace
618/0；JS 语料 31/31。全库 JS_Call 调用点审计：仅此一处损坏（array.rs 与
from_js stringify 均为正确约定）。

## 原现象（存档）

`cargo test --workspace` 间歇性 SIGSEGV（rc=139），锁定崩溃用例：

- 位置：`deps/mquickjs-rs/tests/async_value.rs:137`，`test_async_value_object`
- 路径：`AsyncValue::from_js`/`to_js` 的**对象值**路径
- 频率：约 10-17%/次套件运行（workspace 二进制 20 跑 3 崩；隔离构建 30 跑 5 崩）
- **预存证明**：对抗复核在 worktree@HEAD（不含 3.4 改动）30 跑 3 崩
- 复现法：`cargo test -p mquickjs-rs --test async_value -- --test-threads=1`（单线程可稳定归位崩溃点）

## 初步定性

GC 生命周期类内存错误嫌疑最大：AsyncValue 对象路径涉及跨调用持有 JSValue
（Root/Traced 域）。与"每次 JS_GC 都压缩堆"的知识（gotcha_mquickjs_gc_compaction_and_finalizer）
交叉：若某路径持有的是裸 JSValue 而非 JSGCRef，压缩后即悬垂——间歇性
（取决于测试序列内是否恰好触发压缩）与现象吻合。

## 调查切入点（建议顺序）

1. `--test-threads=1` 稳定复现后，用 `JS_GC` 压缩放大（测试内显式多次 JS_GC）
2. 审查 AsyncValue 对象路径的持有形态：Root<T>/Traced<T> 还是裸 JSValue
3. 对照 d57d6e3（Traced relocation）与 async 桥 UB 修复（55a765b）的历史范围，
   确认该路径是否在修复覆盖外
4. 若确认为裸 JSValue 持有：按 JSGCRef 终态方案改造 + 补压缩场景回归测试

## 验收

- 该测试在 `--test-threads=1` 与并发两种模式下连续 50 次零崩溃
- 压缩放大场景（显式多次 JS_GC）专项测试入套件
