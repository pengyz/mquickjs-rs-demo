# gotcha: OpenVela/NuttX CMake 轨适配五连坑

**类型**：gotcha（平台坑）
**日期**：2026-10-09
**影响**：CMake 轨集成 `js` builtin 时，五处机制与 Make 轨根本不同，逐个咬人。

## 1. `lunch` 无条件覆盖 `VELA_BUILD_BOARD_CONFIG`

`build/envsetup.sh` 的 `lunch` 无论 config 形式（`sim:nsh` 或路径），结尾都
`export VELA_BUILD_BOARD_CONFIG=$boardconfig`（原始参数）。合成 defconfig 的
覆盖必须在 **lunch 之后**导出，才能被 `_wrap_build` 采纳为 `cmake -DBOARD_CONFIG`。

## 2. configure 不在 lunch 里

`lunch` 只设环境变量；`cmake -B` 在 `_build_board` 内部的 `_do_cmake_generator`
（仅当构建目录缺失时执行）。官方入口是 `lunch ...; m`。直接 `cmake --build`
会因 configure 从未执行而报 `Error: <dir> is not a directory`（cmake 3.22 原话）。

## 3. BOARD_CONFIG 指向树外路径会打断板级解析链

Vela cmake 用 `NUTTX_BOARD_ABS_DIR/../..` 逐级回溯板级目录。树外绝对路径
（如本仓库的合成 defconfig）会让回溯落空：romfs `etc` 生成规则消失、板目录
被链接成 `boards/exclude_board`。**正解**：合成 defconfig 落在树内
`boards/<...>/configs/<name>`（纯新增目录，不动受跟踪文件），父链即真实板。

## 4. `nuttx_add_application` 的 COMPILE_FLAGS 不接受多词 flag

COMPILE_FLAGS 列表逐元素喂给 `target_compile_options`：带引号的多词元素
（`"-include x.h"`）被整体 shell-quote 成**含空格的单文件名**；不配对的写法
（`-include` 与文件名分列）则丢掉第二个 `-include`。**正解**：调用后直接
`target_compile_options(<target> PRIVATE "SHELL:-include x.h")`。

## 5. CMake 应用发现不穿透符号链接

`apps/system/CMakeLists.txt` 的 `nuttx_add_subdirectory()` 用
`file(GLOB */CMakeLists.txt)`，不匹配符号链接目录（Make 轨的
`$(wildcard)` 能穿透）。app 接入点必须是**真实目录拷贝**（`cp -a` 保
mtime，未变更文件不触发重建），且拷贝须在全部 staging/头生成之后。

## 关联教训

- Vela cmake 轨全局 `-Werror`：Make 轨只警告的 shadow/unused 在 CMake 轨
  全部变 error。引擎清一次源（11 处）即可双轨对齐。
- 生成的聚合 register.c 若含空表（如 proto_vars 为 0 时的
  `ridl_proto_var_entries`），在 -Werror 下直接失败——生成器必须门控。
- `JS_PUSH_VALUE(ctx, v)` 宏 token 拼接 `v ## _ref`，**要求**函数内有同名
  `JSGCRef` 槽位；嵌套 PUSH/POP 严格顺序时可复用同一槽位，删"看似死代码"
  的声明前先 grep 宏展开。

**关联**：[[nuttx-prebuilt-archive-mixed-binding]]、[[openvela-port-phase0]]
