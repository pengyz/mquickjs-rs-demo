# 异步参数快照机制实现计划

## 目标
实现 `AsyncValue` 类型，支持 JS→Rust 快照和 Rust→JS 还原，确保异步执行期间零 JS 访问。

## 核心约束
- 异步执行函数不允许调用 JS 对象
- 必须操作 Rust 对象
- Rust 对象在 callback 结束后由框架转换到 JS 层
- `any`/`object` 参数必须在提交时"冻结"成 Rust 拥有的数据

## 实现步骤

### Phase 1: AsyncValue 类型定义
- [ ] 在 mquickjs-rs/src/async_value.rs 定义 AsyncValue 枚举
- [ ] 实现 Send + 'static 约束
- [ ] 定义辅助方法 (is_null, as_string, etc.)

### Phase 2: JS→Rust 转换 (from_js)
- [ ] 实现 from_js(scope, value) -> AsyncValue
- [ ] 处理原语类型: null, undefined, bool, int, float, string
- [ ] 处理复合类型: array (递归), object (递归)
- [ ] 处理特殊类型: Date/RegExp → Json
- [ ] 处理不可序列化类型: function/Symbol → Unsupported
- [ ] 实现类型校验: 检测到 Unsupported 时抛 TypeError

### Phase 3: Rust→JS 转换 (to_js)
- [ ] 实现 to_js(ctx, async_value) -> JSValue
- [ ] 处理原语类型: 直接创建 JSValue
- [ ] 处理复合类型: 递归创建 Array/Object
- [ ] 处理 Json 类型: JSON.parse 还原
- [ ] 处理 Null/Undefined: 返回对应 JSValue

### Phase 4: AsyncTaskManager 集成
- [ ] 修改 submit_cancellable 签名: 接收 AsyncValue 参数
- [ ] 添加 callback Root 管理
- [ ] 实现 drain_completed_jobs: 还原结果并调用 callback
- [ ] 实现资源清理: 任务完成/取消时释放 Root

### Phase 5: Glue 模板更新
- [ ] 更新模板生成: 提取参数并调用 from_js
- [ ] 添加类型校验代码生成
- [ ] 更新 callback 调用代码生成

### Phase 6: 测试
- [ ] 单元测试: AsyncValue 类型转换
- [ ] 集成测试: 完整异步流程
- [ ] 边界测试: 不可序列化类型报错

## 验证命令
```bash
cargo test -p mquickjs-rs async_value
cargo test -p ridl-tool async_codegen
cargo run -- tests
```