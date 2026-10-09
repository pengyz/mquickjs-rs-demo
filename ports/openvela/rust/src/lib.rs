//! OpenVela adapter for mquickjs-rs (std mode).
//!
//! openvela sim 是宿主 Linux 进程（glibc 动态链接），Rust std 在此环境
//! 完全可用——无需 GlobalAlloc 桥、panic_handler 或 eh_personality 桩
//! （这些是裸机 MCU no_std 变体的需求，变体保留在 git 历史中，
//! 见 commit d945524 的 lib.rs）。
//!
//! 本 crate 提供 C 导出面，供 `js` builtin 的 stdlib overlay 注入：
//! - `mqjs_rs_version()` — 版本字符串
//! - `mqjs_rs_self_test()` — Context/eval 全链路自检
//!
//! 构建模式：`MQJS_ENGINE_LINK=external`（由本 crate 的 `.cargo/config.toml`
//! 固定设置）——引擎 C 对象由 openvela 源码级集成提供，本 staticlib 只含
//! Rust 对象，`JS_*`/`js_stdlib` 等符号留待镜像链接期解析。
//!
//! std 注意事项：mquickjs-rs 以默认 features（std + ridl-extensions）构建，
//! RIDL stdlib（console 等）依赖的 std I/O（println! → stdout）在 sim 上
//! 原生工作（stdout 即 NSH 控制台）。

/// Returns a static NUL-terminated version string. Never freed.
#[unsafe(no_mangle)]
pub extern "C" fn mqjs_rs_version() -> *const u8 {
    b"mquickjs-rs std (openvela port)\0".as_ptr()
}

/// End-to-end self test: create a Context (2 MiB heap), eval "1+1", verify
/// result == "2", drop. Returns 0 on success, 1 on failure.
///
/// `Context::new(memory_capacity)` 分配内部 8 字节对齐堆块（Vec<u64> →
/// std allocator → glibc），`Context::eval` 返回完成值的字符串形式
/// （`Result<String, String>`）；`Context` 的 `Drop` 负责 `JS_FreeContext`
/// 并随作用域结束自动释放 2 MiB 块。
#[unsafe(no_mangle)]
pub extern "C" fn mqjs_rs_self_test() -> i32 {
    match mquickjs_rs::Context::new(2 * 1024 * 1024) {
        Ok(mut ctx) => match ctx.eval("1+1") {
            Ok(result) => {
                if result.trim() == "2" {
                    0
                } else {
                    1
                }
            }
            Err(_) => 1,
        },
        Err(_) => 1,
    }
}
