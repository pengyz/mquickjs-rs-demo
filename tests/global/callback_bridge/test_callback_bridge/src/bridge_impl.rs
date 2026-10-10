//! Button 的 Rust 实现：演示回调桥的 impl 侧形状。
//!
//! `set_on_click` 从 glue 收到 `CallbackHandle`（glue 已把 JS 函数注册进
//! CallbackRegistry，JSGCRef 持有）；`fire` 模拟 C 事件点——经 registry
//! 同步调用已注册的 JS 回调（many-shot；异常受控，不穿透 C 栈）。

use crate::api::ButtonClass;
use mquickjs_rs::CallbackHandle;

pub struct DefaultButton {
    on_click: Option<CallbackHandle>,
}

impl DefaultButton {
    pub fn new() -> Self {
        Self { on_click: None }
    }
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
        let Some(h) = mquickjs_rs::context::ContextToken::current() else {
            eprintln!("callback bridge: no live context");
            return;
        };
        let Some(cb) = self.on_click else {
            eprintln!("callback bridge: no callback registered");
            return;
        };
        // 立即值参数：调用现场从 C 值转换（切片 1 invoke 的 args 契约）。
        if let Err(e) = h.callbacks().invoke(cb, &[unsafe {
            mquickjs_rs::mquickjs_ffi::JS_NewInt32(h.ctx, code)
        }]) {
            // 错误可见性（切片 2 定下的策略）：Err(JsException(msg)) 打印
            // 上报；ctx 的 pending exception 已由 registry 清除。
            eprintln!("callback bridge: invoke failed: {e}");
        }
    }
}
