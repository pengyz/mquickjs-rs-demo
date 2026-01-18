# compile-fail tests

这些用例用于固化 mquickjs-rs 的 **HandleScope/EscapableHandleScope 类型系统约束**：

- 未 escape 的 handle 不能离开其创建的 scope
- 只能通过 `EscapableHandleScope::escape(...)` 把值提升到外层 scope

测试框架：trybuild
