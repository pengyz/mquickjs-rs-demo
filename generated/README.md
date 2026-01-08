# generated 目录说明

此目录存放由 build.rs 调用 ridl-tool 生成的产物，包含：
- `<module>_glue.rs` / `<module>_impl.rs`：每个 RIDL 模块的胶水与实现骨架
- `ridl_symbols.rs`：聚合符号引用，保证链接符号不被裁剪
- `mquickjs_ridl_register.h`：注册宏头文件

注意：
- 文件会在构建时被覆盖，请勿手动修改。
- 源定义位于 `ridl_modules/` 下，对生成逻辑的调整需修改 RIDL/模板/生成器。
