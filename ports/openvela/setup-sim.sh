#!/usr/bin/env bash
# ports/openvela/setup-sim.sh — idempotent setup of the mquickjs `js` builtin
# in an OpenVela/NuttX sim tree, then build.
#
# Usage:
#   ports/openvela/setup-sim.sh build          # Make 轨（含 rust adapter 构建与 staging）
#   ports/openvela/setup-sim.sh build-cmake    # CMake 轨
#   ports/openvela/setup-sim.sh build-rust     # 仅构建 rust adapter 并 staging + 审计
#   ports/openvela/setup-sim.sh audit-symbols  # 仅符号审计（分配对拆绑守卫）
#   OPENVELA_DIR env var overrides the openvela tree (default ~/workspace/openvela).
#
# 非侵入原则（2026-10-09 用户裁定）：移植适配不得修改 openvela 树的公共基础
# 设施（defconfig / Kconfig / 构建脚本 / external 内容）。对 openvela 树的
# 全部写入仅限两类：
#   1. apps/system/mqjs 应用目录（apps 树的标准应用接入点，纯新增；真实
#      目录拷贝而非符号链接 —— Make 轨 $(wildcard) 能穿透符号链接，CMake
#      轨 nuttx_add_subdirectory() 用 file(GLOB */CMakeLists.txt) 发现应用，
#      不匹配符号链接目录，js builtin 会静默失联）
#   2. 构建产物目录（nuttx/ 的 .config 与目标文件、cmake_out/、apps/staging/
#      的适配器归档——均为可再生成的构建状态，非受跟踪内容）
# 其余一切适配物（合成 defconfig、CUSTOM_MODULE_PATH 覆盖目录、atom 头、
# 引擎源 staging、stdlib overlay）都收敛在本仓库 ports/openvela/ 内。
#
# 集成模型（源码级，参照 apps/interpreters/quickjs）：引擎 .c 由 openvela
# apps 构建用 NuttX 工具链编译，libc 绑定全部一致落在 NuttX 侧；引擎自身
# 工具链（mqjs_stdlib host 工具）负责 atom 表等宿主代码生成。
#
# RIDL stdlib（M1-R+）：镜像的 stdlib 是 **ridl 变体** —— 运行时 TU 来自
# deps/mquickjs-rs（mqjs_stdlib_impl.c 定义 strong `js_stdlib`，其表含 RIDL
# 扩展 = console → Rust adapter），寄存器 TU 来自本仓库 sim 应用聚合
# （ports/openvela/rust 为 RIDL 叶子，仅选 stdlib 模块）。宿主工具由
# mqjs_stdlib_template.c（非 base mqjs_stdlib.c）+ mquickjs_build.c 以
# -DMQUICKJS_ENABLE_RIDL_EXTENSIONS 构建。context 创建与 eval 走 Rust 侧
# （adapter 的 mqjs_rs_ridl_* 三件套）—— RIDL 胶水经 JSContext user_data
# 分派，而 user_data 只有 mquickjs_rs::Context::new 会安装。
#
# 双轨互斥：Make 轨与 CMake 轨共用同一 nuttx 树，切换方向需先
# distclean（Make→CMake 由本脚本自动执行；CMake→Make 由 configure 触发）。
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OPENVELA="${OPENVELA_DIR:-$HOME/workspace/openvela}"
ENGINE_SRC="$REPO_ROOT/deps/mquickjs"
RS_SRC="$REPO_ROOT/deps/mquickjs-rs"
APP_DIR="$REPO_ROOT/ports/openvela/app"
GEN_DIR="$APP_DIR/gen"
ENGINE_STAGE="$APP_DIR/engine_src"
CMAKE_OVERRIDE_DIR="$REPO_ROOT/ports/openvela/cmake"
RUST_DIR="$REPO_ROOT/ports/openvela/rust"
RUST_TARGET="x86_64-unknown-linux-gnu"
ADAPTER_A="$RUST_DIR/target/$RUST_TARGET/release/libmqjs_openvela_adapter.a"
STAGING_DIR="$OPENVELA/apps/staging"
# sim 应用的 RIDL 聚合目录（ridl-builder 按 app 包名归一化）：
RIDL_APP_ID="mqjs_openvela_adapter"
AGG_DIR="$RUST_DIR/target/ridl/apps/$RIDL_APP_ID/aggregate"

if [ ! -d "$OPENVELA/nuttx" ]; then
    echo "ERROR: openvela tree not found at $OPENVELA (set OPENVELA_DIR)" >&2
    exit 1
fi
if [ ! -f "$ENGINE_SRC/mquickjs.c" ]; then
    echo "ERROR: engine sources not found at $ENGINE_SRC" >&2
    exit 1
fi

# ---------------------------------------------------------------------------
# RIDL aggregate for the sim app (before anything consumes it)
# ---------------------------------------------------------------------------

ensure_ridl_tool() {
    if [ ! -x "$REPO_ROOT/target/debug/ridl-tool" ]; then
        echo "ridl-tool binary missing — building"
        (cd "$REPO_ROOT" && cargo build -q -p ridl-tool)
    fi
}

# Generates ports/openvela/rust/target/ridl/apps/<app>/aggregate/ from the
# adapter crate's RIDL module selection ([dependencies.stdlib] = console).
ensure_sim_aggregate() {
    if [ -f "$AGG_DIR/mquickjs_ridl_register.h" ] \
        && [ -f "$AGG_DIR/ridl_context_ext.rs" ]; then
        return 0
    fi
    echo "sim RIDL aggregate missing — generating (stdlib module only)"
    (cd "$REPO_ROOT" && cargo run -q -p ridl-builder -- aggregate \
        --cargo-toml "$RUST_DIR/Cargo.toml" --intent build)
    [ -f "$AGG_DIR/mquickjs_ridl_register.h" ] || {
        echo "ERROR: aggregate did not produce $AGG_DIR/mquickjs_ridl_register.h" >&2
        exit 1
    }
}

# ---------------------------------------------------------------------------
# Engine build outputs (base for bindgen metadata; ridl variant driven by the
# SIM aggregate so the adapter's bindgen headers and the image's generated
# stdlib share one source of truth).
# ---------------------------------------------------------------------------

# mquickjs-sys's build.rs hard-fails without the base variant's
# mquickjs_build_output.json (bindgen headers live there).
ensure_engine_variant() {
    local mode="$1"
    local variant_root="$REPO_ROOT/target/mquickjs-build/framework/$RUST_TARGET/$mode"
    local base_ok ridl_ok
    base_ok=0
    ridl_ok=0
    [ -f "$variant_root/base/mquickjs_build_output.json" ] && base_ok=1
    # The ridl include dir must match the CURRENT sim aggregate (not a stale
    # one from another app) — compare the register headers.
    if [ -f "$variant_root/ridl/include/mquickjs_ridl_register.h" ] \
        && cmp -s "$variant_root/ridl/include/mquickjs_ridl_register.h" \
                  "$AGG_DIR/mquickjs_ridl_register.h"; then
        ridl_ok=1
    fi

    if [ "$base_ok" -eq 1 ] && [ "$ridl_ok" -eq 1 ]; then
        return 0
    fi
    echo "engine build outputs stale/missing ($mode) — regenerating (base + ridl from sim aggregate)"
    (
        cd "$REPO_ROOT"
        if [ "$base_ok" -ne 1 ]; then
            # base variant: engine build metadata + bindgen headers (ridl-free)
            TARGET="$RUST_TARGET" PROFILE="$mode" \
                cargo run -q -p ridl-builder -- build-mquickjs
        fi
        # ridl variant: build the engine with the SIM app's aggregated
        # register header (NOTE: mirrors `ridl-builder prepare` steps 3+4
        # but scoped to this app; the romclass map step is skipped — the sim
        # aggregate has no module-mode modules, so no js_ext_romclass__*
        # symbols exist to map).
        cargo run -q -p mquickjs-build -- build \
            --mquickjs-dir "$ENGINE_SRC" \
            --ridl-register-h "$AGG_DIR/mquickjs_ridl_register.h" \
            --out "target/mquickjs-build/framework/$RUST_TARGET/$mode/ridl"
    )
}

# audit-symbols (plan D2, Phase 1 混绑教训的制度化):
#   adapter 归档引用的分配族符号必须成套 —— 有 malloc/calloc/realloc 之一
#   必有 free（分配对拆绑 = 堆损坏定时炸弹）。归档级检查分配对与内核分配器
#   误用；最终镜像存在时追加检查 malloc/free 绑定对称性与 js_stdlib 落地。
#   注：用 grep -Ex 全词匹配而非子串（避免误伤 Rust mangled 符号）。
audit_symbols() {
    local archive="${1:-}"
    local rc=0

    if [ -z "$archive" ]; then
        archive="$STAGING_DIR/$(basename "$ADAPTER_A")"
    fi
    if [ ! -f "$archive" ] && [ -f "$ADAPTER_A" ]; then
        archive="$ADAPTER_A"
    fi
    if [ ! -f "$archive" ]; then
        echo "AUDIT ERROR: adapter archive not found (run 'build-rust' first)" >&2
        return 2
    fi

    echo "AUDIT: $archive"
    local unds allocs free_present kernel_allocs
    unds="$(LC_ALL=C nm --undefined-only "$archive" 2>/dev/null \
        | awk '$1=="U" {print $2}' | sed 's/@.*//' | sort -u)"
    free_present="$(printf '%s\n' "$unds" | grep -cx free || true)"
    allocs="$(printf '%s\n' "$unds" \
        | grep -Ex 'malloc|calloc|realloc|reallocarray|aligned_alloc|posix_memalign|memalign|valloc|strdup|strndup' \
        | sort -u || true)"
    kernel_allocs="$(printf '%s\n' "$unds" \
        | grep -E '^(kmm|mm)_(malloc|free|calloc|realloc|memalign)' | sort -u || true)"

    if [ -n "$kernel_allocs" ]; then
        echo "AUDIT FAIL: kernel-only allocator symbols referenced (must use plain libc malloc/free):"
        printf '  %s\n' $kernel_allocs
        rc=1
    fi
    if [ -n "$allocs" ] && [ "$free_present" -eq 0 ]; then
        echo "AUDIT FAIL: allocation pair split — allocators present but free missing:"
        printf '  %s\n' $allocs
        rc=1
    fi
    if [ "$rc" -eq 0 ]; then
        if [ -n "$allocs" ]; then
            echo "AUDIT OK: allocation set (paired with free): $(echo $allocs)"
        else
            echo "AUDIT OK: no allocator symbols referenced"
        fi
    fi

    # Final-image level: allocation binding symmetry + js_stdlib resolution.
    # Skipped when the image predates the adapter archive (build order:
    # stage -> make -> audit), otherwise the stale image would fail on
    # symbols only the NEW adapter/image pair can resolve.
    local img="$OPENVELA/nuttx/nuttx" st_m st_f st_c st_r st_sl
    if [ -f "$img" ]; then
        if [ "$img" -ot "$archive" ]; then
            echo "AUDIT: final image predates the adapter archive — image-level checks skipped (re-run audit after make)"
        else
        defined_in_image() {
            LC_ALL=C nm --defined-only "$img" 2>/dev/null | awk -v s="$1" '$3==s{f=1} END{exit !f}'
        }
        st_m=host; st_f=host; st_c=host; st_r=host
        defined_in_image malloc && st_m=nuttx
        defined_in_image free   && st_f=nuttx
        defined_in_image calloc && st_c=nuttx
        defined_in_image realloc && st_r=nuttx
        echo "AUDIT: image binding — malloc:$st_m free:$st_f calloc:$st_c realloc:$st_r"
        if [ "$st_m" != "$st_f" ]; then
            echo "AUDIT FAIL: malloc/free split binding in final image (one NuttX, one host glibc)"
            rc=1
        fi
        st_sl="$(LC_ALL=C nm "$img" 2>/dev/null | awk '$3=="js_stdlib"{print $1}' | head -n1 || true)"
        if [ -z "$st_sl" ] || [ "$st_sl" = "U" ]; then
            echo "AUDIT FAIL: js_stdlib not resolved inside the image ($st_sl)"
            rc=1
        else
            echo "AUDIT: js_stdlib defined in image ($st_sl)"
        fi
        # RIDL console glue must be resolved too (Rust adapter provides it).
        local st_log
        st_log="$(LC_ALL=C nm "$img" 2>/dev/null | awk '$3=="js_global_singleton_console_log"{print $1}' | head -n1 || true)"
        if [ -z "$st_log" ] || [ "$st_log" = "U" ]; then
            echo "AUDIT FAIL: js_global_singleton_console_log not resolved inside the image ($st_log)"
            rc=1
        else
            echo "AUDIT: js_global_singleton_console_log (Rust RIDL console) defined in image"
        fi
        fi
    else
        echo "AUDIT: final image not built yet — archive-level checks only"
    fi

    if [ "$rc" -eq 0 ]; then
        echo "AUDIT: PASS"
    fi
    return $rc
}

build_rust_adapter() {
    ensure_ridl_tool
    ensure_sim_aggregate
    ensure_engine_variant release
    echo "building rust adapter (release, $RUST_TARGET)"
    (cd "$RUST_DIR" && cargo build --release --target "$RUST_TARGET")
    # Stage for the apps flat-image link: arch/sim/src/Makefile does
    #   EXTRA_LIBS += $(wildcard $(APPDIR)/staging/*.a)
    # and links them inside LDSTARTGROUP/LDENDGROUP, so the adapter's
    # JS_*/js_stdlib/JS_RIDL_StdlibInit references resolve from libapps.a
    # regardless of order.
    mkdir -p "$STAGING_DIR"
    cp -f "$ADAPTER_A" "$STAGING_DIR/"
    echo "adapter staged: $STAGING_DIR/$(basename "$ADAPTER_A")"
    audit_symbols "$STAGING_DIR/$(basename "$ADAPTER_A")"
}

# 1) stage engine + ridl-stdlib runtime sources + headers for the apps build.
#    - core: deps/mquickjs (unchanged Phase 1 set, minus base mqjs_stdlib.c)
#    - ridl stdlib runtime: deps/mquickjs-rs (mqjs_stdlib_impl.c defines the
#      strong js_stdlib; require.c implements require())
#    - host-tool template: deps/mquickjs-rs/mqjs_stdlib_template.c (staged
#      copy gets the rs overlay; it is compiled to a HOST tool only)
mkdir -p "$ENGINE_STAGE"
rm -f "$ENGINE_STAGE/mqjs_stdlib.c"   # base stdlib TU retired from the image
for f in mquickjs.c cutils.c dtoa.c libm.c \
         mquickjs.h mquickjs_priv.h cutils.h dtoa.h libm.h list.h; do
    cp -f "$ENGINE_SRC/$f" "$ENGINE_STAGE/$f"
done
for f in mqjs_stdlib_impl.c mqjs_stdlib_template.c; do
    cp -f "$RS_SRC/$f" "$ENGINE_STAGE/$f"
done
cp -f "$RS_SRC/require.c" "$ENGINE_STAGE/mqjs_require.c"

# 1b) stdlib overlay — compile-time registration of the rs probe functions,
#     applied to the STAGED TEMPLATE copy (the ridl variant's host tool
#     source; the template's js_global_object[] ends at the globalThis entry
#     followed by the JS_RIDL_EXTENSIONS injection point).
#     mquickjs 没有 QuickJS 式的运行时注册 API（JS_NewCFunctionParams 只能
#     按 stdlib 表下标造函数对象；AGENTS.md 核心约束：注册必须编译期），
#     因此两个桥接函数走引擎既有的 stdlib 声明表路径：把条目注入 staged
#     副本（本仓库构建产物，不碰 deps/mquickjs-rs、不碰 openvela 树），
#     step 3 的 host 工具据此把 rsVersion/rsSelfTest 烤进生成的
#     mqjs_ridl_stdlib.h；其表引用的 js_rs_* 钩子由 js_main.c 定义（非
#     static —— 生成表现在 mqjs_stdlib_impl.o 里，跨 TU 引用）。
MQJS_TEMPLATE_STAGED="$ENGINE_STAGE/mqjs_stdlib_template.c"
if ! grep -q 'js_rs_version' "$MQJS_TEMPLATE_STAGED"; then
    OVERLAY_ANCHOR='JS_PROP_NULL_DEF("globalThis", 0 ),'
    grep -qF "$OVERLAY_ANCHOR" "$MQJS_TEMPLATE_STAGED" || {
        echo "ERROR: stdlib overlay anchor not found — engine source drift, \
update the overlay in setup-sim.sh" >&2
        exit 1
    }
    sed -i "\|$OVERLAY_ANCHOR|a\\
    JS_CFUNC_DEF(\"rsVersion\", 0, js_rs_version),\\
    JS_CFUNC_DEF(\"rsSelfTest\", 0, js_rs_self_test)," "$MQJS_TEMPLATE_STAGED"
    echo "stdlib overlay applied: rsVersion/rsSelfTest -> $MQJS_TEMPLATE_STAGED"
fi

# 2) engine-native host toolchain, RIDL variant: build the mqjs_stdlib host
#    tool from the OVERLAID staged template + mquickjs_build.c (both with
#    -DMQUICKJS_ENABLE_RIDL_EXTENSIONS so the generated js_stdlib is a
#    strong definition and the RIDL extensions are expanded into
#    js_global_object[]), then generate the atom-table + stdlib headers.
#    (Mirrors mquickjs-build's ridl variant steps 1-4; headers resolve from
#    the pristine engine source dir and the sim app aggregate dir.)
ensure_ridl_tool
ensure_sim_aggregate
mkdir -p "$GEN_DIR"
HOST_FLAGS="-O2 -D_GNU_SOURCE -fno-math-errno -fno-trapping-math \
-I$ENGINE_SRC -I$AGG_DIR -DMQUICKJS_ENABLE_RIDL_EXTENSIONS"
cc $HOST_FLAGS -c "$MQJS_TEMPLATE_STAGED" -o "$GEN_DIR/mqjs_stdlib.host.o"
cc $HOST_FLAGS -c "$ENGINE_SRC/mquickjs_build.c" -o "$GEN_DIR/mquickjs_build.host.o"
cc $HOST_FLAGS -o "$GEN_DIR/mqjs_stdlib" \
    "$GEN_DIR/mqjs_stdlib.host.o" "$GEN_DIR/mquickjs_build.host.o"
"$GEN_DIR/mqjs_stdlib" -a > "$GEN_DIR/mquickjs_atom.h"
"$GEN_DIR/mqjs_stdlib" > "$GEN_DIR/mqjs_ridl_stdlib.h"
echo "atom headers generated: $GEN_DIR/mquickjs_atom.h mqjs_ridl_stdlib.h"
if ! grep -q 'js_rs_version' "$GEN_DIR/mqjs_ridl_stdlib.h"; then
    echo "ERROR: rs overlay missing from generated mqjs_ridl_stdlib.h" >&2
    exit 1
fi
if ! grep -q 'js_global_singleton_console_log' "$GEN_DIR/mqjs_ridl_stdlib.h"; then
    echo "ERROR: RIDL console missing from generated mqjs_ridl_stdlib.h \
(aggregate stale? delete $AGG_DIR and re-run)" >&2
    exit 1
fi

# 2a) rs hook declarations. The generated stdlib table references the
#     overlay hooks (js_rs_version/js_rs_self_test) by address from
#     mqjs_stdlib_impl.o WITHOUT declaring them (they are not RIDL
#     exports, so mquickjs_ridl_api.h doesn't know them). Every app TU
#     force-includes this generated header after the api header.
cat > "$GEN_DIR/mqjs_rs_hooks.h" <<'EOF'
/* Generated by ports/openvela/setup-sim.sh — do not commit.
 * Declarations for the rs probe hooks baked into the generated stdlib
 * table by the staged template overlay (step 2b). */
#ifndef MQJS_RS_HOOKS_H
#define MQJS_RS_HOOKS_H
JSValue js_rs_version(JSContext *ctx, JSValue *this_val, int argc, JSValue *argv);
JSValue js_rs_self_test(JSContext *ctx, JSValue *this_val, int argc, JSValue *argv);
#endif
EOF

# 2b) stage the aggregate's runtime register TU (JS_RIDL_StdlibInit + module
#     require table) for the apps build, next to the other staged TUs.
cp -f "$AGG_DIR/mquickjs_ridl_register.c" "$ENGINE_STAGE/mqjs_ridl_register.c"
cp -f "$AGG_DIR/mquickjs_ridl_api.h" "$ENGINE_STAGE/mquickjs_ridl_api.h"
cp -f "$AGG_DIR/mquickjs_ridl_register.h" "$ENGINE_STAGE/mquickjs_ridl_register.h"

# 3) composite defconfig（合成：openvela sim:nsh defconfig + 本移植片段）
#    落点必须是树内 configs/mqjs（纯新增目录，不碰任何受跟踪文件）：
#    Vela cmake 用 NUTTX_BOARD_ABS_DIR/../.. 回溯板级目录——树外绝对路径
#    会让板级解析链断裂（etc romfs 生成规则丢失、board 目录被链接成
#    boards/exclude_board）；树内 configs/mqjs 的父链就是真实 sim 板，
#    EXISTS 分支与 pair 形式等价解析。
COMPOSITE_DEFCONFIG_DIR="$OPENVELA/nuttx/boards/sim/sim/sim/configs/mqjs"
BASE_DEFCONFIG="$OPENVELA/nuttx/boards/sim/sim/sim/configs/nsh/defconfig"
if [ ! -f "$BASE_DEFCONFIG" ]; then
    echo "ERROR: base defconfig not found at $BASE_DEFCONFIG" >&2
    exit 1
fi
mkdir -p "$COMPOSITE_DEFCONFIG_DIR"
cp "$BASE_DEFCONFIG" "$COMPOSITE_DEFCONFIG_DIR/defconfig"
cat >> "$COMPOSITE_DEFCONFIG_DIR/defconfig" <<'EOF'

# mquickjs js builtin (ports/openvela) — composite defconfig, not in-tree
CONFIG_MQJS_JS=y
CONFIG_FS_HOSTFS=y
CONFIG_BOARDCTL=y
EOF

# 4) framework.mk / framework.cmake（两轨各自消费的机器本地路径；CMake 轨
#    的合成 defconfig 走 VELA_BUILD_BOARD_CONFIG 环境变量，无需单独文件）
cat > "$APP_DIR/framework.mk" <<EOF
# Generated by ports/openvela/setup-sim.sh — do not commit.
MQJS_GEN_DIR := $GEN_DIR
MQJS_ENGINE_SRC := $ENGINE_SRC
MQJS_AGG_DIR := $AGG_DIR
EOF
cat > "$APP_DIR/framework.cmake" <<EOF
# Generated by ports/openvela/setup-sim.sh — do not commit.
set(MQJS_GEN_DIR "$GEN_DIR")
set(MQJS_ENGINE_SRC "$ENGINE_SRC")
set(MQJS_AGG_DIR "$AGG_DIR")
set(MQJS_ADAPTER_A "$ADAPTER_A")
EOF

# 5) app dir 同步（apps 树标准接入点，纯新增）。真实目录拷贝而非符号链接：
#    CMake 轨 nuttx_add_subdirectory() 用 file(GLOB */CMakeLists.txt) 发现
#    应用，不匹配符号链接目录。必须在全部内容（engine_src staging、gen/
#    头、framework.*）就绪之后执行；app 内容变更后重跑本脚本即可同步。
#    cp -a 保留 mtime，未变更文件不触发 Make/CMake 重建。
rm -rf "$OPENVELA/apps/system/mqjs"
cp -a "$APP_DIR" "$OPENVELA/apps/system/mqjs"
echo "app dir synced: $OPENVELA/apps/system/mqjs (real copy, CMake-glob compatible)"

# 6) build
MODE="${1:-}"
case "$MODE" in
"build")
    # adapter staticlib 必须在镜像链接前进 staging（LDLIBS wildcard 消费）
    build_rust_adapter
    cd "$OPENVELA/nuttx"
    if [ ! -f .config ]; then
        ./tools/configure.sh sim:nsh
    fi
    # 片段追加到 .config（构建状态文件，非受跟踪基础设施）
    grep -q "CONFIG_MQJS_JS=y" .config || cat >> .config <<'EOF'
CONFIG_MQJS_JS=y
CONFIG_FS_HOSTFS=y
CONFIG_BOARDCTL=y
EOF
    make olddefconfig >/dev/null
    make -j"$(nproc)"
    echo "OK: $OPENVELA/nuttx/nuttx"
    # Final audit against the FRESH image (image-level checks ran stale
    # before make — see audit_symbols).
    audit_symbols "$STAGING_DIR/$(basename "$ADAPTER_A")"
    ;;
"build-cmake")
    # adapter 构建与轨道无关（产物进 apps/staging；CMake 轨的 .a 链接消费
    # 属 M2 范围，这里仅保证 release 变体可自动生成、staging 就绪不报错）
    build_rust_adapter
    # 与官方序列一致（distclean → lunch → m）。注意 configure 不在 lunch
    # 里，而在 _build_board 内部的 _do_cmake_generator（cmake -B，仅当
    # 构建目录缺失时执行——因此 rm -rf 保证全新 configure）；直接
    # `cmake --build` 会因 configure 从未执行而报 "is not a directory"。
    if [ -f "$OPENVELA/nuttx/Makefile" ] || [ -f "$OPENVELA/nuttx/.config" ]; then
        echo "NOTE: switching tracks — distclean (build artifacts only)"
        make -C "$OPENVELA/nuttx" distclean >/dev/null 2>&1 || true
    fi
    rm -rf "$OPENVELA/cmake_out"

    # envsetup 不兼容 nounset：在子 shell 中关闭后执行 lunch/构建。
    # VELA_BUILD_BOARD_CONFIG 必须在 lunch 之**后**导出：lunch 无论哪种
    # config 形式都会无条件把它覆盖为原始参数（sim:nsh）；它被 _wrap_build
    # 采纳为 cmake -DBOARD_CONFIG（绝对路径，nuttx/CMakeLists.txt 的
    # EXISTS 分支原生支持），从而指向本仓库的合成 defconfig 目录，
    # 零改动 openvela 树。
    (
        set +eu
        cd "$OPENVELA"
        source build/envsetup.sh
        lunch sim:nsh cmake_out/sim_nsh >/dev/null
        export VELA_BUILD_BOARD_CONFIG="$COMPOSITE_DEFCONFIG_DIR"
        m
    )
    echo "OK: $OPENVELA/cmake_out/sim_nsh/nuttx"
    ;;
"build-rust")
    build_rust_adapter
    ;;
"audit-symbols")
    audit_symbols "${2:-}"
    ;;
*)
    echo "setup complete (pass 'build' | 'build-cmake' | 'build-rust' | 'audit-symbols' to also build/audit)"
    ;;
esac
