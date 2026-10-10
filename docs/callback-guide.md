# RIDL 同步回调使用指南（Callback Bridge）

## 概述

RIDL 的 `callback` 参数类型提供 **同步、多次触发（many-shot）** 的回调桥：
JS 传入函数 → SDK 经引擎 `JSGCRef` 安全持有（GC 标记 + 压缩重定位自动处理）
→ Rust/C 侧凭句柄**多次**同步调用。

适用场景：事件回调（UI 点击、传感器阈值、状态变更）——"C 事件点直接跑 JS"。
与之正交的 async bridge（fire-and-forget 完成通知）适用于异步任务的结果回传，
两者勿混用。

## RIDL 语法

```typescript
class Button {
    // 内联匿名回调
    fn setOnClick(cb: callback(code: i32)) -> void;

    // 触发入口：Rust impl 持句柄，模拟 C 事件点调用
    fn fire(code: i32) -> void;
}
```

也可用具名声明：

```typescript
callback ClickHandler(code: i32);

class Button {
    fn setOnClick(cb: ClickHandler) -> void;
}
```

## v1 参数白名单（回调参数）

| 类型 | C 侧形态 | 说明 |
|------|---------|------|
| `bool` / `i32` | `int` / `int32_t` | 立即值 |
| `f64` | `double` | 堆值（走 invoke_rooted） |
| `string` | `const char *` | 堆值 |
| `string?` | `const char *`（NULL = None） | error-first 约定 |

白名单之外的类型在**生成期**报 `unsupported` 定位错误。回调无返回值
（v1 语法即如此）。`Optional(callback)` 暂不支持。

## Rust impl 侧

glue 提取后把 `CallbackHandle` 传给 impl；触发时经
`ContextToken::current()` 拿 registry 调用：

```rust
use mquickjs_rs::CallbackHandle;

pub struct DefaultButton {
    on_click: Option<CallbackHandle>,
}

impl ButtonClass for DefaultButton {
    fn set_on_click<'ctx>(
        &mut self,
        _env: &mut mquickjs_rs::Env<'ctx>,
        cb: mquickjs_rs::CallbackHandle,
    ) {
        self.on_click = Some(cb);
    }

    fn fire(&mut self, code: i32) {
        let Some(h) = mquickjs_rs::context::ContextToken::current() else { return };
        let Some(cb) = self.on_click else { return };
        if let Err(e) = h.callbacks().invoke(cb, &[unsafe {
            mquickjs_rs::mquickjs_ffi::JS_NewInt32(h.ctx, code)
        }]) {
            // 错误可见性：Err(JsException(msg)) 打印上报；
            // ctx 的 pending exception 已由 registry 清除。
            eprintln!("button: callback failed: {e}");
        }
    }
}
```

## JS 侧

```javascript
var b = new Button();
var clicks = 0;

b.setOnClick(function (code) {
    clicks = clicks + code;
});

b.fire(2);
b.fire(3);   // many-shot：同一函数可多次触发

// 传非函数 → TypeError（glue 的 JS_IsFunction 校验）
// 回调体内 throw → registry 捕获清除，宿主不崩、ctx 可继续
```

## 语义保证

- **GC 安全**：注册的函数经 JSGCRef 持有，任意次 `JS_GC`（含堆压缩）后
  仍可调用；`unregister` 或 Context 销毁后释放。
- **异常受控**：回调体内 throw → `invoke` 返回
  `Err(CallbackError::JsException(msg))`，ctx 无残留异常（含嵌套 throw：
  `throw { toString(){...} }` 场景已覆盖）。
- **同步重入安全**：回调体内可再触发其它回调或注销自己（registry 锁
  不跨 `JS_Call`）。

## 端到端示例

`tests/global/callback_bridge/test_callback_bridge/`（Button + fire，
JS 用例 `tests/{basic,errors}.js`）。
