# test_struct_enum（暂缓）

当前该模块用于覆盖 RIDL 的 enum/struct（含 msgpack struct）语法与端到端生成/注册/JS 侧调用。

## 暂缓原因

目前 ridl-builder 聚合链路在解析包含 `mode ...;` 的 RIDL 文件时失败，导致包含 enum/struct 的用例无法进入端到端流程。

在聚合链路对齐到完整支持 `mode` + enum/struct 之前：
- 该模块不纳入 app 依赖与聚合范围
- 不添加/不执行该模块下的 JS 用例

## 后续补齐

待聚合链路修复后，应补齐：
- enum 定义与取值
- struct/json/msgpack/protobuf struct 定义与字段类型组合
- struct/enum 在 class/singleton/fn/var 场景下的引用与序列化/反序列化路径
