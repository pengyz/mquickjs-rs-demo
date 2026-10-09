# 专项调查：test_async_value_object 间歇性 SIGSEGV（P1）

> 状态：待排期（Phase 3.5 收尾时由对抗复核发现，**预存问题、非 QEMU 工作引入**）
> 日期：2026-10-09

## 现象

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
