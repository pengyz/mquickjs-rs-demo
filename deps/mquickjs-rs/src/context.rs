#[cfg(feature = "no-std")]
use alloc::{boxed::Box, format, string::String, string::ToString, vec, vec::Vec};
use core::cell::RefCell;
use core::ffi::{CStr};
use alloc::ffi::{CString};
use core::ffi::c_void;
use alloc::sync::Arc;

use crate::handles::local::{Local, Value};
use crate::mquickjs_ffi;

pub struct ContextInner {
    // NOTE: host per-context extensions (initialized by application-generated ridl_context_init).
    // Type-erased to avoid coupling mquickjs-rs to generated RIDL types.
    ridl_ext_ptr: core::cell::UnsafeCell<*mut c_void>,
    ridl_ext_drop: core::cell::UnsafeCell<Option<unsafe fn(*mut c_void)>>,

    pub(crate) roots: crate::roots::RootsRegistry,

    pub(crate) alive: core::sync::atomic::AtomicBool,
    
    /// Async task manager for RIDL async cancellation semantics.
    ///
    /// 以 `Arc` 共享：异步任务在 worker 线程持有它，JS 线程通过 `drain_completions`
    /// 访问同一个实例。**必须共享同一份**——按位拷贝 `AsyncTaskManager`
    /// （内含 `Mutex`）既是 UB，也会产生两把独立的锁而破坏互斥。
    #[cfg(feature = "std")]
    pub async_task_manager: Arc<crate::async_task::AsyncTaskManager>,
}

impl ContextInner {
    pub(crate) fn new() -> Self {
        Self {
            ridl_ext_ptr: core::cell::UnsafeCell::new(core::ptr::null_mut()),
            ridl_ext_drop: core::cell::UnsafeCell::new(None),
            roots: crate::roots::RootsRegistry::new(),
            alive: core::sync::atomic::AtomicBool::new(true),
            #[cfg(feature = "std")]
            async_task_manager: Arc::new(crate::async_task::AsyncTaskManager::new()),
        }
    }

    /// Safety: must only be set once per ContextInner.
    pub unsafe fn set_ridl_ext(&self, ptr: *mut c_void, drop_fn: unsafe fn(*mut c_void)) {
        let p = unsafe { &mut *self.ridl_ext_ptr.get() };
        debug_assert!(p.is_null());
        *p = ptr;

        let d = unsafe { &mut *self.ridl_ext_drop.get() };
        debug_assert!(d.is_none());
        *d = Some(drop_fn);
    }

    pub fn ridl_ext_ptr(&self) -> *mut c_void {
        unsafe { *self.ridl_ext_ptr.get() }
    }
}

impl Drop for ContextInner {
    fn drop(&mut self) {
        // Safety: Drop happens when all Arcs are gone. Must not call any JS API.
        let p = unsafe { *self.ridl_ext_ptr.get() };
        let drop_fn = unsafe { *self.ridl_ext_drop.get() };
        if let (Some(f), false) = (drop_fn, p.is_null()) {
            unsafe { f(p) };
        }
    }
}

/// 除 context 头部与 class 表之外，留给引擎堆/栈的最小字节数。
///
/// 这是**可用性下限**（保证 context 建起来后还有空间做基本求值），
/// 不是引擎的精确阈值 —— 引擎自身只断言总大小 >= 1024。
const MIN_HEAP_BYTES: usize = 8 * 1024;

pub struct Context {
    pub ctx: *mut mquickjs_ffi::JSContext,
    #[allow(dead_code)]
    pub(crate) inner: Arc<ContextInner>,
    _memory: AlignedHeap,
}

/// 交给 mquickjs 的 context 内存块。
///
/// # 为什么不能直接用 `Vec<u8>`
///
/// 引擎在 `JS_NewContext2` 中要求内存起始地址满足
/// `assert(((uintptr_t)mem_start & (mem_align - 1)) == 0)`
/// —— 64 位下 `mem_align = 8`（见 `mquickjs.c`）。
///
/// 而 `Vec<u8>` **不保证**任何超过 1 字节的对齐：实践中系统分配器对
/// 大块内存会过度对齐（glibc x86-64 为 16），所以一直是"碰巧可用"。
/// 一旦分配器行为变化，debug 构建会 assert 失败、release 构建（assert 被
/// 编译掉）则会静默错位。
///
/// 这里用 `Vec<u64>` 承载，显式保证 8 字节对齐，把该契约变成类型层面的保证。
struct AlignedHeap {
    words: Vec<u64>,
}

impl AlignedHeap {
    /// 分配至少 `bytes` 字节、8 字节对齐的零初始化内存。
    fn new(bytes: usize) -> Self {
        let words = bytes.div_ceil(8);
        Self {
            words: vec![0u64; words],
        }
    }

    fn as_mut_ptr(&mut self) -> *mut u8 {
        self.words.as_mut_ptr() as *mut u8
    }

    fn len(&self) -> usize {
        self.words.len() * 8
    }
}

/// Borrow-like handle reconstructed from JSContext user_data.
/// It must NOT free the JSContext.
pub struct ContextToken {
    pub ctx: *mut mquickjs_ffi::JSContext,
    pub inner: Arc<ContextInner>,
}

#[cfg(not(feature = "no-std"))]
thread_local! {
    static TLS_CURRENT_CTX: RefCell<Vec<ContextToken>> = RefCell::new(Vec::new());
}
#[cfg(feature = "no-std")]
static TLS_CURRENT_CTX: crate::TlsCell<RefCell<Vec<ContextToken>>> =
    crate::TlsCell::new(RefCell::new(Vec::new()));

pub struct CurrentGuard {
    _private: (),
}

impl Drop for CurrentGuard {
    fn drop(&mut self) {
        TLS_CURRENT_CTX.with(|s| {
            let mut stack = s.borrow_mut();
            let _ = stack.pop();
        });
    }
}

impl ContextToken {
    /// Safety: ctx must be alive, and ctx user_data must have been set by mquickjs-rs Context.
    pub unsafe fn from_js_ctx(ctx: *mut mquickjs_ffi::JSContext) -> Option<Self> {
        if ctx.is_null() {
            return None;
        }
        let p = unsafe { mquickjs_ffi::JS_GetContextUserData(ctx) };
        if p.is_null() {
            return None;
        }

        // user_data holds a raw Arc<ContextInner> pointer (created by Arc::into_raw).
        let inner_ptr = p as *const ContextInner;
        unsafe { Arc::increment_strong_count(inner_ptr) };
        let inner = unsafe { Arc::from_raw(inner_ptr) };
        Some(Self { ctx, inner })
    }

    pub fn enter_current(&self) -> CurrentGuard {
        TLS_CURRENT_CTX.with(|s| {
            let mut stack = s.borrow_mut();
            stack.push(ContextToken {
                ctx: self.ctx,
                inner: self.inner.clone(),
            });
        });
        CurrentGuard { _private: () }
    }

    pub fn with_current<R>(&self, f: impl FnOnce() -> R) -> R {
        let _g = self.enter_current();
        f()
    }

    pub fn current() -> Option<ContextToken> {
        TLS_CURRENT_CTX.with(|s| {
            let stack = s.borrow();
            stack.last().map(|h| ContextToken {
                ctx: h.ctx,
                inner: h.inner.clone(),
            })
        })
    }
}

impl Context {
    pub fn token(&self) -> ContextToken {
        ContextToken {
            ctx: self.ctx,
            inner: self.inner.clone(),
        }
    }
    
    /// Get a reference to the async task manager
    /// 返回 context 级 `AsyncTaskManager` 的共享句柄。
    ///
    /// 返回 `Arc` 克隆（而非引用）是刻意的：async 任务需要把它移动到 worker
    /// 线程，只有共享同一份实例，worker 推入的完成项才能被 JS 线程的
    /// `drain_completions` 观察到。
    #[cfg(feature = "std")]
    pub fn async_task_manager(&self) -> Arc<crate::async_task::AsyncTaskManager> {
        self.inner.async_task_manager.clone()
    }

    pub fn new(memory_capacity: usize) -> Result<Self, Box<dyn core::error::Error>> {
        extern "C" {
            static js_stdlib: mquickjs_ffi::JSSTDLibraryDef;
        }

        // 前置校验缓冲区大小。
        //
        // 引擎的 JSContext 以柔性数组成员 class_proto[] 结尾，heap_base 紧随
        // 2 * class_count 个 JSValue 之后。若缓冲区不足，heap_base 会越过
        // stack_top —— 引擎**不会**返回错误，而是直接内存损坏（实测为
        // SIGSEGV）。引擎内部只有 `assert(mem_size >= 1024)`，既未考虑
        // class_count，该断言在 release 下也会被编译掉。
        //
        // 因此这里按 stdlib 的实际 class_count 计算真实下限。
        let min_required = unsafe {
            let class_count = js_stdlib.class_count as usize;
            let header = mquickjs_ffi::JS_ContextHeaderSize();
            let class_tables = 2 * class_count * core::mem::size_of::<mquickjs_ffi::JSValue>();
            header + class_tables + MIN_HEAP_BYTES
        };

        if memory_capacity < min_required {
            return Err(format!(
                "memory_capacity too small: {memory_capacity} bytes, \
                 need at least {min_required} bytes \
                 (context header + class tables for {} classes + {} bytes heap)",
                unsafe { js_stdlib.class_count },
                MIN_HEAP_BYTES,
            )
            .into());
        }

        let mut memory = AlignedHeap::new(memory_capacity);

        let ctx = unsafe {
            mquickjs_ffi::JS_NewContext(
                memory.as_mut_ptr() as *mut c_void,
                memory.len(),
                &js_stdlib,
            )
        };

        if ctx.is_null() {
            return Err("Failed to create JSContext".into());
        }

        let inner = Arc::new(ContextInner::new());

        // Store an Arc clone inside JSContext user_data.
        // Finalizer will drop this clone; Context::drop will drop its own Arc.
        unsafe extern "C" fn user_data_finalizer(
            _ctx: *mut mquickjs_ffi::JSContext,
            user_data: *mut c_void,
        ) {
            if user_data.is_null() {
                return;
            }
            // Safety: user_data created by Arc::into_raw.
            let arc = unsafe { Arc::from_raw(user_data as *const ContextInner) };
            drop(arc);
        }

        let arc_ptr = Arc::into_raw(inner.clone()) as *mut c_void;
        unsafe {
            mquickjs_ffi::JS_SetContextUserData(ctx, arc_ptr, Some(user_data_finalizer));
        }

        // NOTE: 这里**不再**注册 JS_SetContextGCMark。
        //
        // 自建 root registry 通过 JS_SetContextGCMark 只能参与 mark 阶段，
        // 其持有的 JSValue 不参与 gc_compact_heap 的重定位 → 压缩后悬垂。
        // 现改用引擎的 JSGCRef（见 crate::roots），它在 mark
        // (mquickjs.c:12319-12323) 与重定位 (mquickjs.c:12592-12596) 两个阶段
        // 都被扫描，是 mquickjs 下唯一压缩安全的跨 GC 持有机制。

        Ok(Context {
            ctx,
            inner,
            _memory: memory,
        })
    }

    pub fn eval_jsvalue(&mut self, code: &str) -> Result<mquickjs_ffi::JSValue, String> {
        let handle = self.token();
        let _g = handle.enter_current();

        let c_code = CString::new(code).map_err(|e| e.to_string())?;
        let filename = CString::new("eval.js").unwrap();

        let result = unsafe {
            mquickjs_ffi::JS_Eval(
                self.ctx,
                c_code.as_ptr(),
                code.len(),
                filename.as_ptr(),
                mquickjs_ffi::JS_EVAL_RETVAL as i32,
            )
        };

        // 检查返回值是否为异常
        // 在mquickjs中，JS_TAG_EXCEPTION是特殊的tag
        let tag = (result as u32) & ((1 << mquickjs_ffi::JS_TAG_SPECIAL_BITS) - 1);
        if tag == mquickjs_ffi::JS_TAG_EXCEPTION as u32 {
            let exception = unsafe { mquickjs_ffi::JS_GetException(self.ctx) };

            // 创建一个临时缓冲区用于JS_ToCString
            let mut cstr_buf = mquickjs_ffi::JSCStringBuf { buf: [0; 5] };
            let error_ptr =
                unsafe { mquickjs_ffi::JS_ToCString(self.ctx, exception, &mut cstr_buf) };

            if !error_ptr.is_null() {
                let error_str = unsafe { CStr::from_ptr(error_ptr).to_string_lossy().into_owned() };

                return Err(error_str);
            } else {
                return Err("Unknown error".to_string());
            }
        }

        Ok(result)
    }

    pub fn eval(&mut self, code: &str) -> Result<String, String> {
        let result = self.eval_jsvalue(code)?;

        // 创建一个临时缓冲区用于JS_ToCString
        let mut cstr_buf = mquickjs_ffi::JSCStringBuf { buf: [0; 5] };
        let result_ptr = unsafe { mquickjs_ffi::JS_ToCString(self.ctx, result, &mut cstr_buf) };

        if !result_ptr.is_null() {
            let result_str = unsafe { CStr::from_ptr(result_ptr).to_string_lossy().into_owned() };
            Ok(result_str)
        } else {
            Ok("undefined".to_string())
        }
    }

    /// 创建一个新的字符串值
    pub fn create_string<'a>(
        &self,
        scope: &crate::handles::scope::Scope<'a>,
        rust_str: &str,
    ) -> Result<Local<'a, Value>, String> {
        let c_str = CString::new(rust_str).map_err(|e| e.to_string())?;
        let js_value = unsafe { mquickjs_ffi::JS_NewString(self.ctx, c_str.as_ptr()) };

        if (js_value as u32) & ((1u32 << (mquickjs_ffi::JS_TAG_SPECIAL_BITS as u32)) - 1)
            == (mquickjs_ffi::JS_TAG_EXCEPTION as u32)
        {
            return Err("Failed to create string".to_string());
        }

        Ok(scope.value(js_value))
    }

    /// 创建一个新的数字值
    pub fn create_number<'a>(
        &self,
        scope: &crate::handles::scope::Scope<'a>,
        num: f64,
    ) -> Result<Local<'a, Value>, String> {
        let js_value = unsafe { mquickjs_ffi::JS_NewFloat64(self.ctx, num) };

        if (js_value as u32) & ((1u32 << (mquickjs_ffi::JS_TAG_SPECIAL_BITS as u32)) - 1)
            == (mquickjs_ffi::JS_TAG_EXCEPTION as u32)
        {
            return Err("Failed to create number".to_string());
        }

        Ok(scope.value(js_value))
    }

    /// 创建一个新的布尔值
    pub fn create_boolean<'a>(
        &self,
        scope: &crate::handles::scope::Scope<'a>,
        boolean: bool,
    ) -> Result<Local<'a, Value>, String> {
        let js_value = mquickjs_ffi::js_mkbool(boolean);
        Ok(scope.value(js_value))
    }

    /// 创建一个新的对象
    pub fn create_object<'a>(
        &self,
        scope: &crate::handles::scope::Scope<'a>,
    ) -> Result<Local<'a, Value>, String> {
        let obj = unsafe { mquickjs_ffi::JS_NewObject(self.ctx) };
        if (obj as u32) & ((1u32 << (mquickjs_ffi::JS_TAG_SPECIAL_BITS as u32)) - 1)
            == (mquickjs_ffi::JS_TAG_EXCEPTION as u32)
        {
            return Err("Failed to create object".to_string());
        }
        Ok(scope.value(obj))
    }

    /// 将值转换为Rust字符串
    pub fn get_string(&self, value: Local<'_, Value>) -> Result<String, String> {
        let mut cstr_buf = mquickjs_ffi::JSCStringBuf { buf: [0; 5] };
        let result_ptr =
            unsafe { mquickjs_ffi::JS_ToCString(self.ctx, value.as_raw(), &mut cstr_buf) };

        if result_ptr.is_null() {
            return Err("Failed to convert Value to string".to_string());
        }

        Ok(unsafe { CStr::from_ptr(result_ptr) }
            .to_string_lossy()
            .into_owned())
    }

    /// 获取数字值
    pub fn get_number(&self, value: Local<'_, Value>) -> Result<f64, String> {
        let mut result = 0.0;
        let ret = unsafe { mquickjs_ffi::JS_ToNumber(self.ctx, &mut result, value.as_raw()) };

        if ret != 0 {
            return Err("Failed to convert Value to number".to_string());
        }

        Ok(result)
    }

    /// 获取布尔值
    pub fn get_boolean(&self, value: Local<'_, Value>) -> Result<bool, String> {
        let mut result = 0i32;
        let ret = unsafe { mquickjs_ffi::JS_ToInt32(self.ctx, &mut result, value.as_raw()) };

        if ret != 0 {
            return Err("Failed to convert Value to boolean".to_string());
        }

        Ok(result != 0)
    }

    /// Drain completed async tasks and invoke callbacks
    ///
    /// This method should be called from the JS main thread to process
    /// completed async tasks and invoke their callbacks.
    ///
    /// # Safety
    /// - Must be called from the JS main thread
    /// - Must be called after ridl_context_init
    #[cfg(feature = "std")]
    pub unsafe fn drain_completions(&self) {
        let completions = self.inner.async_task_manager.drain_completions();

        for item in completions {
            // Look up callback for this task
            let callback_raw = self.inner.async_task_manager.take_callback(item.task_id);

            if let Some(cb_raw) = callback_raw {
                // Call the JS callback with (error, result) arguments
                match item.result {
                    Ok(value) => {
                        // Success: call callback(null, value)
                        let c_value = alloc::ffi::CString::new(value.as_str()).unwrap_or_default();
                        let js_value = mquickjs_ffi::JS_NewString(self.ctx, c_value.as_ptr());
                        let js_null = mquickjs_ffi::JS_NULL;

                        mquickjs_ffi::JS_PushArg(self.ctx, js_null);
                        mquickjs_ffi::JS_PushArg(self.ctx, js_value);
                        mquickjs_ffi::JS_PushArg(self.ctx, cb_raw);
                        mquickjs_ffi::JS_PushArg(self.ctx, mquickjs_ffi::JS_UNDEFINED);
                        mquickjs_ffi::JS_Call(self.ctx, 2);
                    }
                    Err(error_msg) => {
                        // Error: call callback(error, null)
                        let c_error = alloc::ffi::CString::new(error_msg.as_str()).unwrap_or_default();
                        let js_error = mquickjs_ffi::JS_NewString(self.ctx, c_error.as_ptr());
                        let js_null = mquickjs_ffi::JS_NULL;

                        mquickjs_ffi::JS_PushArg(self.ctx, js_error);
                        mquickjs_ffi::JS_PushArg(self.ctx, js_null);
                        mquickjs_ffi::JS_PushArg(self.ctx, cb_raw);
                        mquickjs_ffi::JS_PushArg(self.ctx, mquickjs_ffi::JS_UNDEFINED);
                        mquickjs_ffi::JS_Call(self.ctx, 2);
                    }
                }
            }

            // Mark task as completed
            self.inner.async_task_manager.complete_task(item.task_id);
        }
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        // no-std 模式下没有异步子系统，无需取消处理。
        #[cfg(feature = "std")]
        {
            // Mark context as dropping and cancel all cancellable async tasks
            self.inner.async_task_manager.mark_context_dropping();
            let cancelled_tasks = self.inner.async_task_manager.cancel_all_cancellable();

            if !cancelled_tasks.is_empty() {
                // Log cancelled tasks for debugging
                eprintln!(
                    "Context drop: cancelled {} cancellable async tasks",
                    cancelled_tasks.len()
                );
            }

            // Check for non-cancellable tasks that are still running
            let non_cancellable_count = self.inner.async_task_manager.non_cancellable_task_count();
            if non_cancellable_count > 0 {
                eprintln!(
                    "Context drop: {} non-cancellable async tasks are still running",
                    non_cancellable_count
                );
            }
        }
        
        self.inner
            .alive
            .store(false, core::sync::atomic::Ordering::Release);

        unsafe {
            mquickjs_ffi::JS_FreeContext(self.ctx);
        }
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::new(1024 * 1024).expect("Failed to create default Context") // 默认 1MB
    }
}
