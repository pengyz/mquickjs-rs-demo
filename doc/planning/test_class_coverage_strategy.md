# test_class RIDL 模块：Class 功能测试覆盖策略

> 状态：规划中（global-only 版本）

## 背景与目标

我们将新增一个专门用于 **class 功能测试** 的 RIDL module：`test_class`。

目标：在 `cargo run -- tests` 的 JS 集成用例中，系统性验证 class 相关：
- RIDL 语法组合在生成/注册/运行时是否正确；
- QuickJS ABI/表项签名对齐（constructor/method/getter/finalizer）；
- 关键错误路径是否返回 JS exception（而不是崩溃或 silent UB）。

## 当前约束（必须显式写入测试）

- **当前只支持 global 模式**（class 暴露到 `globalThis`）。
- **module 模式尚未完整实现**：
  - 暂不做“模块导出可见性/导入语义/模块命名空间”相关断言。
  - JS 测试文件中需明确标注：`// NOTE: global-only; TODO(module): ...`，待 module 支持后补测。

## 覆盖维度拆解（矩阵）

### 1) Class 定义语法维度（以当前项目支持为准，后续会根据扫描结果裁剪）

- module 声明：无（GLOBAL）/有（**暂不做可见性断言，但会覆盖 module path 参与生成的命名/slot key 等**）
- constructor：有/无；参数 0/1/多
- method：0/1/多参数；不同返回值类型；错误入参
- property（instance）：readonly / readwrite；不同类型；setter 校验
- property（proto）：proto readonly / proto readwrite；未初始化读取；多实例共享语义

> 注：如果 RIDL 暂不支持某些语法（例如 varargs、static、可选参数等），则改为“解析/生成期应报错”的负向测试或暂缓。

### 2) 运行时行为维度（JS 黑盒验证）

- 构造：`new Class(...)` 成功、初始化正确、多实例隔离
- this/receiver 校验：非法 receiver 调用应抛错（例如 "invalid receiver"）
- 参数校验：缺参/错参应抛错（而非崩溃）
- 方法：
  - `instance.method(...)` 必须可调用（强约束）
  - 约束：方法应挂载在 prototype 上（`Object.getPrototypeOf(instance)` 上可见）
- 属性（instance）：
  - `readonly`：写入必须抛错或无效（按当前 glue 约定，但必须有稳定行为，不允许 silent UB）
  - `readwrite`：必须可读写（强约束）；写入后读取一致；可验证默认值
  - 约束：属性应通过 getter/setter 形式挂载在 prototype 上（`Object.getOwnPropertyDescriptor(proto, "prop")` 非空）
- proto：
  - 未初始化访问的错误路径
  - setter/getter 后一致性
  - 多实例共享（若 proto 语义为共享）

> 说明：本轮已将 "class.methods" 与 "class.properties" 都纳入 class prototype 的 JSPropDef 表生成，
> 并区分 proto property 与 instance property 的 getter/setter 命名映射，作为强约束测试的基础。

### 3) 关键组合（优先覆盖）

1. GLOBAL class + instance method + finalizer（稳定性/ABI）
2. 带 module decl 的 class + proto property（覆盖 slot_key/normalize 相关路径；不做可见性断言）
3. constructor 参数 + method 参数 + property setter 参数（多类型混合）
4. proto property + singleton 并存（ctx slot 分配不冲突）
5. 多个 class 并存（class id 分配与 JS_CLASS_* 宏正确）

### 4) 负向测试（必须有）

- 运行时错误路径：
  - invalid receiver
  - 缺参/错参
  - proto 未初始化
- 解析/生成期错误（若我们具备 compile-fail 测试框架，则纳入；否则先记录在 TODO）

## 用例组织（落地到 repo）

### RIDL module 结构（建议）

- `ridl-modules/test_class/`
  - `Cargo.toml`
  - `src/lib.rs`（或遵循现有 ridl module 模板）
  - `src/*.ridl`：按主题拆分
    - `basic.ridl`：constructor/method/property 基本覆盖
    - `proto.ridl`：proto 相关
    - `normalize.ridl`：module path/命名边界（global-only 下仅覆盖生成命名/slot，不做可见性）

### JS 测试用例（建议文件）

- `tests/smoke_test_class_basic.js`
- `tests/smoke_test_class_proto.js`
- `tests/smoke_test_class_errors.js`

每个文件头部必须包含：

```js
// NOTE: 当前仅支持 global 模式（globalThis 暴露）。
// TODO(module): 支持 module 模式后，补充导出可见性/导入语义相关断言。
```

## 验收标准

- `cargo run -p ridl-builder -- prepare` 成功
- `cargo test` 通过（期望零 warning）
- `cargo run -- tests` 新增用例全部通过
- 不引入硬编码白名单/特殊 casing（新增模块仅放入 `ridl-modules/` 即生效）

## TODO（待 module 支持后补测）

- class 在模块导出中的可见性与命名空间
- `import`/`export` 语义与多模块冲突处理
- module path normalize 对导出路径的影响

## TODO（proto 属性强约束，待约定标准化后启用）

- proto property 的存储/生命周期由模块提供 C ABI（`ridl_create_proto_*` / `ridl_drop_proto_*` / `ridl_proto_get_*` / `ridl_proto_set_*`）
- 在约定与示例模块（demo_default）对齐并稳定后：
  - 恢复 `test_class` 的 proto property 用例
  - 增加“未初始化访问必须抛错”与“初始化后读写一致”的强约束断言
