# test_types（global_mode）

本模块用于覆盖 RIDL 类型系统相关的端到端行为（parser -> generator -> 注册 -> JS 调用）。

## 覆盖点（第一阶段）

- nullable：`T?`
- any 可透传 null（`any_nullable=true`）
- union 可空：`(A | B | null)?` 以及 `A | B | null` 的规范化

## 记录：mode 语法限制（过渡）

当前 ridl-tool 仅支持 `mode strict;`，不支持 `mode default;`/`mode loose;` 之类的显式写法。
但 strict 模式又禁止在非 variadic 参数中使用 `any`。

为了继续覆盖 any/null 语义，本模块暂时不写 mode_decl（省略 `mode ...;`），使其走解析器的默认 FileMode::Default。

后续如引入 `mode default;` 的显式语法与语义，应将本模块改回显式 mode。 
