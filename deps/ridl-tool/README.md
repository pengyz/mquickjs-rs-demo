# ridl-tool 概览

ridl-tool 是用于解析/校验 RIDL 并生成代码的 CLI，当前在本仓库作为子模块使用，并由顶层 `build.rs` 调用。

## 目录
- `src/`：核心代码
  - `parser/`：RIDL 语法解析
  - `validator/`：语义校验
  - `generator/`：模板生成（Askama）
- `templates/`：生成模板（Rust/C 相关）
- `doc/`：设计方案、技术选型、语法规范等文档

## 用法（本仓库集成）
- 顶层 `build.rs` 收集 RIDL 文件并调用 ridl-tool：
  - `module <ridl> <out_dir>`：生成 `<module>_glue.rs` / `<module>_impl.rs`
  - `aggregate <ridl...> <out_dir>`：生成 `ridl_symbols.rs`、`mquickjs_ridl_register.h`
- 生成产物会被复制到项目根与 `generated/`。

## 注意
- 模板/生成逻辑变更后需重新构建以刷新输出。
- 设计文档详见 `doc/` 目录（语法规范、聚合设计、技术选型等）。
