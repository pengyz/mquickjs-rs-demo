# test_types（global_mode）

本模块用于覆盖 RIDL 类型系统相关的端到端行为（parser -> generator -> 注册 -> JS 调用）。

## 覆盖点（第一阶段 / Phase A）

本阶段先收敛到最小可回归的 V1 default 基线：

- bool/i32/f64：参数与返回 roundtrip
- any：透传（primitive 保持类型/值，object 保持引用 identity）

## 暂不支持（待补齐到 V1 合规）

- union：当前 ridl-tool 无法生成 Rust 类型（`unsupported ridl type in rust_type_from_idl: Union(...)`）
- nullable 参数（Option<T>）：v1 glue 当前无法生成参数转换
- string 参数：v1 glue 当前把 string param 转为 `*const c_char`，但生成 trait 使用 `String`，导致不匹配

## 记录：mode 语法限制（过渡）

当前 ridl-tool 仅支持 `mode strict;`，不支持 `mode default;`/`mode loose;` 之类的显式写法。
但 strict 模式又禁止在非 variadic 参数中使用 `any`。

为了继续覆盖 any/null 语义，本模块暂时不写 mode_decl（省略 `mode ...;`），使其走解析器的默认 FileMode::Default。

后续如引入 `mode default;` 的显式语法与语义，应将本模块改回显式 mode。 
