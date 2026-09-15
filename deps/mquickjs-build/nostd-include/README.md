# 裸机 libc 桩头文件

供 `mquickjs-build --target <triple>` 交叉编译 mquickjs 引擎时使用。

引擎是 freestanding 的，只用到约 15 个 libc 函数；在没有
`arm-none-eabi` 之类完整 sysroot 的环境下，这些最小声明足以让
`clang --target=... -ffreestanding` 完成编译。

| 头文件 | 说明 |
|---|---|
| `assert.h` | `NDEBUG` 下置空 |
| `ctype.h` / `string.h` / `stdlib.h` | 声明，由链接期提供实现 |
| `inttypes.h` | `PRIx32` 等宏 |
| `math.h` | 多数映射到 clang 内建函数 |
| `setjmp.h` | 引擎实际不使用，仅为满足 include |
| `stdio.h` | dump/调试路径用；可整体桩掉 |

生成器工具（`mquickjs_build.c` 的 ROM 构建）**始终用宿主编译器**，
不使用本目录。

背景见 `docs/knowledge/assessment_core_nostd_port_cost.md`。
