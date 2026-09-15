//! Context 内存对齐契约测试。
//!
//! mquickjs 的 `JS_NewContext2` 要求内存起始地址 8 字节对齐（64 位下）：
//!
//! ```c
//! #ifdef JS_PTR64
//!     mem_align = 8;
//! #else
//!     mem_align = 4;
//! #endif
//!     assert(((uintptr_t)mem_start & (mem_align - 1)) == 0);
//! ```
//!
//! 而 `Vec<u8>` **不保证**任何超过 1 字节的对齐 —— 实践中靠系统分配器对
//! 大块内存的过度对齐"碰巧"满足；且该 `assert` 在 release 下会被编译掉。
//!
//! `Context::new` 因此改用显式 8 字节对齐的承载类型（`Vec<u64>`）。
//! 本文件锁定该契约。

/// 引擎把 `mem_start` 直接当作 `JSContext *`，因此 `ctx` 的地址就是
/// 内存起始地址 —— 它必须 8 字节对齐。
#[test]
fn context_memory_is_eight_byte_aligned() {
    for capacity in [
        16 * 1024,      // 刚好越过下限
        16 * 1024 + 1,  // 非 8 的倍数
        1024 * 1024,
        1000 * 1000,    // 非 8 的倍数
    ] {
        let ctx = mquickjs_rs::Context::new(capacity)
            .unwrap_or_else(|e| panic!("capacity={capacity} 创建失败: {e}"));

        let addr = ctx.ctx as usize;
        assert_eq!(
            addr % 8,
            0,
            "capacity={capacity}: ctx 地址 {addr:#x} 未满足 8 字节对齐"
        );

        // 确认该 context 确实可用（而非仅地址对齐）
        drop(ctx);
    }
}

/// 非 8 倍数的容量不应导致失败或越界（引擎会向下取整 mem_size）。
#[test]
fn odd_capacity_is_accepted() {
    let mut ctx = mquickjs_rs::Context::new(16 * 1024 + 1).expect("create context");
    let v = ctx.eval_jsvalue("1 + 1").expect("eval");
    // JS_TAG_INT = 0 ⇒ (v & 1) == 0，值为 v >> 1
    assert_eq!((v & 1), 0, "应返回整数");
    assert_eq!((v as i64) >> 1, 2);
}

/// **过小的容量必须返回 Err，而不是崩溃。**
///
/// 修复前：`Context::new(1024)` 会让引擎的 heap_base 越过 stack_top，
/// 直接 SIGSEGV（引擎不返回错误，且其内部断言在 release 下被编译掉）。
#[test]
fn too_small_capacity_returns_error_instead_of_crashing() {
    for capacity in [0, 1024, 4096, 8192] {
        match mquickjs_rs::Context::new(capacity) {
            Ok(_) => panic!("capacity={capacity} 应被拒绝（缓冲区不足）"),
            Err(e) => {
                let msg = e.to_string();
                assert!(
                    msg.contains("too small"),
                    "capacity={capacity} 的错误信息应说明缓冲区过小，实际: {msg}"
                );
            }
        }
    }
}
