# test_js_fields（过渡：绕开 singleton var_member/proto var_member）

本模块用于覆盖 RIDL extensions 中的 js-only fields（var/proto var）与初始化语义。

## 当前过渡策略

ridl-builder 聚合链路当前不接受 singleton 内的 `var ... = literal`（解析期报 expected singleton_member）。
为了继续推进 nullable/string literal 的端到端覆盖，本模块暂时仅通过方法返回值验证：
- any 可透传 null
- nullable 参数/返回对 null 的行为

## 后续补齐

待聚合链路修复后，应补齐：
- singleton var/proto var 的 init 规则（null/string literal/any/optional 等）
- strict 模式下的校验差异
- 错误定位（非法 init literal / 不支持类型）
