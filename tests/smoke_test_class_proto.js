// NOTE: 当前仅支持 global 模式（globalThis 暴露）。
// TODO(module): 支持 module 模式后，补充导出可见性/导入语义相关断言。
// TODO(proto): 当前 test_class 暂不覆盖 proto property，因为 proto 需要模块提供 C ABI 的
// ridl_create_proto_* / ridl_drop_proto_* / ridl_proto_get_* 等导出，这部分约定需要先标准化。
