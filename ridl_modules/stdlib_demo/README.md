# stdlib_demo 模块说明

示例 RIDL 模块，演示 JS ↔ Rust 绑定生成流程。

## 目录结构
- `stdlib_demo.ridl`：接口定义
- `stdlib_demo_glue.rs` / `stdlib_demo_impl.rs`：生成的胶水与实现骨架
- `src/lib.rs`：导出模块（按需）
- 生成产物同步拷贝：项目根、`generated/`

## 功能示例
- `js_say_hello()`：返回字符串示例，可在 JS 侧调用

## 构建与运行
- `cargo build`（根目录）会触发 ridl-tool 生成并复制 glue/impl/符号文件。
- JS 运行：`cargo run -- <your.js>`，JS 可调用 `js_say_hello()`。

## 开发提示
- 修改接口：编辑 `.ridl` 后重新构建。
- 修改实现：在 `stdlib_demo_impl.rs` 填充业务逻辑。
- 不要手改生成文件；改 RIDL/模板/生成器后重建。
