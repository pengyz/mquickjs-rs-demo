# test_literals（过渡：绕开 singleton var_member）

本模块用于覆盖 RIDL 的字面量相关语法，尤其是 string literal escapes 的端到端行为（parser -> generator -> JS）。

## 当前过渡策略

ridl-builder 聚合链路当前在解析 singleton 内的 `var ... = literal` 时失败（expected singleton_member）。
为了不阻塞整体迁移与新增覆盖，本模块暂时不使用 singleton 的 var/proto var 成员，而改用方法返回值来承载字面量断言。

## 后续补齐

待聚合链路/语法对齐支持 singleton var_member 后，应补齐：
- singleton var init 的 string literal / null literal
- proto var init 的 string literal / null literal
- 与 nullable/any 的组合
