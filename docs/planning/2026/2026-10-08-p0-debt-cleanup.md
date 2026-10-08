# P0 正确性债清理计划（四项）

> 状态：✅ 已完成（2026-10-08 实施并全量验证通过；424/153/33 测试 + 31/31 JS）  
> 实施注记：项 1 伴生发现并修复三处静默值损坏面——class js_fields 字符串裸嵌  
> glue（无转义）、C proto var 同款、**rust_glue 模板缺 `escape="none"`**  
> （HTML 转义把 `"` 变 `&quot;`，编译通过但值错，历史仅因插值全为标识符而  
> 未暴露）。项 4 实测 grammar 关键字表与 validator 保留字表不一致  
> （`callback` 仅后者有；`proto`/`opaque`/`Traced` 仅前者有）——两表合一  
> 另立待办。  
> 来源：SDK 定位评估与 TODO 处置过程中确认的遗留债，用户拍板"先做1"。

## 1. class js_fields 类型矩阵不一致（validator vs glue）

- 现状：validator 允许 `{Bool, I32, I64, F32, F64, String, Null}`，glue 模板
  只实现 `{String, I32, Bool, Null}`，其余 `unreachable!()` 运行时 panic；
  C 侧 proto var 模板同为四类型。仓库无任何 .ridl 使用 I64/F32/F64。
- 裁定：**validator 收窄到端到端真实可用的 `{i32, bool, string, null}`**
  （与 singleton var 一致），错误信息列支持集。扩类型属新特性另立
  （需 JS 数值转换设计与 ROM props 枚举核查）。
- TDD：validator 测试——三类各一条断言拒绝 + 消息含支持集；现有测试无
  使用这些类型的断言（Phase C 已核实），预期无回归。

## 2. runner 命令语义与文档

- 现状：`cargo run -p mquickjs-demo -- tests`（AGENTS.md/README/QUICKSTART
  现行写法）只扫 tests/ 树 26 项，漏 ridl-modules stdlib 3 项；无参才全量。
- 裁定：文档统一改为**无参**形式（全量 31 项）；显式路径参数保留现有
  CWD 相对语义并在 tests/README.md 注明（定向跑某目录时有用）。

## 3. 文档腐化

- 根 Cargo.toml 注释"test modules not linked into the demo app"失实
  （apps/demo/Cargo.toml 实际全量依赖测试模块）→ 改为事实描述。
- `ridl-modules/stdlib/stdlib.ridl`（顶层，566B，2026-01-24）是 v0 前
  设计稿：构建只用 `src/stdlib.ridl`（348B，strict 模式 console），
  全仓无引用 → 删除（git 历史可溯）。

## 4. validator 位置信息（剩余 3 处 TODO）

- 现状：Phase C/D 已消化大半（8→3）；剩余为保留字检查
  （validator/mod.rs:846-847）无行号，因 Interface/Enum/StructDef 的
  AST 节点无 `pos` 字段（Class/Singleton/ModuleDeclaration 已有）。
- 修法：三个节点补 `pos: Option<SourcePos>`（serde default 向后兼容），
  parser 捕获 pest span，保留字检查线程 pos（有则填、无则维持 0），
  删除过时 TODO 注释（:121）。
- TDD：保留字用于 interface/enum/struct 名称时错误信息含 line:col。

## 验证门槛

cargo build；ridl-tool / mquickjs-rs / mquickjs-demo 测试无回归；
无参 JS 全量 31/31；prepare 正常。

## 明确不做

- js_fields 扩类型支持（f64 族）——新特性另立
- 显式路径参数改 workspace 解析——保留定向扫描语义，仅文档说明
