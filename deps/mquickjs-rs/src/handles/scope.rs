use std::cell::RefCell;
use std::marker::PhantomData;

use crate::context::ContextToken;

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct ContextId(pub(crate) u64);

struct CurrentEntry {
    ctx: *mut crate::mquickjs_ffi::JSContext,
    id: ContextId,
}

thread_local! {
    static TLS_CURRENT: RefCell<Vec<CurrentEntry>> = RefCell::new(Vec::new());
}

pub struct EnterGuard<'ctx> {
    expected_ctx: *mut crate::mquickjs_ffi::JSContext,
    expected_id: ContextId,
    _p: PhantomData<&'ctx ContextToken>,
}

impl Drop for EnterGuard<'_> {
    fn drop(&mut self) {
        TLS_CURRENT.with(|s| {
            let mut st = s.borrow_mut();
            let top = st.pop().expect("enter/exit stack underflow");
            assert_eq!(top.id, self.expected_id, "Context enter/exit out of order");
            assert_eq!(
                top.ctx, self.expected_ctx,
                "Context enter/exit out of order"
            );
        })
    }
}

pub struct Scope<'ctx> {
    pub(crate) h: &'ctx ContextToken,
    _g: EnterGuard<'ctx>,
}

impl<'ctx> Scope<'ctx> {
    pub fn ctx(&self) -> *mut crate::mquickjs_ffi::JSContext {
        self.h.ctx
    }

    pub fn ctx_raw(&self) -> *mut crate::mquickjs_ffi::JSContext {
        self.h.ctx
    }

    pub fn context_id(&self) -> ContextId {
        ContextId(std::sync::Arc::as_ptr(&self.h.inner) as usize as u64)
    }

    pub fn from_handle(h: &'ctx ContextToken) -> Self {
        let id = ContextId(std::sync::Arc::as_ptr(&h.inner) as usize as u64);
        TLS_CURRENT.with(|s| {
            s.borrow_mut().push(CurrentEntry { ctx: h.ctx, id });
        });

        let g = EnterGuard {
            expected_ctx: h.ctx,
            expected_id: id,
            _p: PhantomData,
        };

        Scope { h, _g: g }
    }
}

impl ContextToken {
    pub fn enter_scope(&self) -> Scope<'_> {
        let ctx = self.ctx;
        let id = ContextId(std::sync::Arc::as_ptr(&self.inner) as usize as u64);
        TLS_CURRENT.with(|s| {
            s.borrow_mut().push(CurrentEntry { ctx, id });
        });

        let g = EnterGuard {
            expected_ctx: ctx,
            expected_id: id,
            _p: PhantomData,
        };

        Scope { h: self, _g: g }
    }
}
