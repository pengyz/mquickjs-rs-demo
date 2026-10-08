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
#   1. apps/system/mqjs 符号链接（apps 树的标准应用接入点，纯新增）
#   2. 构建产物目录（nuttx/ 的 .config 与目标文件、cmake_out/、apps/staging/
#      的适配器归档——均为可再生成的构建状态，非受跟踪内容）
# 其余一切适配物（合成 defconfig、CUSTOM_MODULE_PATH 覆盖目录、atom 头、
# 引擎源 staging、stdlib overlay）都收敛在本仓库 ports/openvela/ 内。
#
# 集成模型（源码级，参照 apps/interpreters/quickjs）：引擎 .c 由 openvela
# apps 构建用 NuttX 工具链编译，libc 绑定全部一致落在 NuttX 侧；引擎自身
# 工具链（mqjs_stdlib host 工具）负责 atom 表等宿主代码生成。
#
# 双轨互斥：Make 轨与 CMake 轨共用同一 nuttx 树，切换方向需先
# distclean（Make→CMake 由本脚本自动执行；CMake→Make 由 configure 触发）。
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OPENVELA="${OPENVELA_DIR:-$HOME/workspace/openvela}"
ENGINE_SRC="$REPO_ROOT/deps/mquickjs"
APP_DIR="$REPO_ROOT/ports/openvela/app"
GEN_DIR="$APP_DIR/gen"
ENGINE_STAGE="$APP_DIR/engine_src"
CMAKE_OVERRIDE_DIR="$REPO_ROOT/ports/openvela/cmake"
COMPOSITE_DEFCONFIG_DIR="$GEN_DIR/nsh"
RUST_DIR="$REPO_ROOT/ports/openvela/rust"
RUST_TARGET="x86_64-unknown-linux-gnu"
ADAPTER_A="$RUST_DIR/target/$RUST_TARGET/release/libmqjs_openvela_adapter.a"
STAGING_DIR="$OPENVELA/apps/staging"

if [ ! -d "$OPENVELA/nuttx" ]; then
    echo "ERROR: openvela tree not found at $OPENVELA (set OPENVELA_DIR)" >&2
    exit 1
fi
if [ ! -f "$ENGINE_SRC/mquickjs.c" ]; then
    echo "ERROR: engine sources not found at $ENGINE_SRC" >&2
    exit 1
fi

# ---------------------------------------------------------------------------
# Rust adapter (Phase 2a / M1-R)
# ---------------------------------------------------------------------------

# Ensure the engine build outputs exist for the cargo profile the adapter is
# built with.  MQJS_ENGINE_LINK=external only removes engine LINKING from the
# Rust build (the image provides engine objects at source level); but
#   - mquickjs-sys's build.rs hard-fails without the base variant's
#     mquickjs_build_output.json (bindgen headers live there), and
#   - mquickjs-rs's bindgen unconditionally force-includes
#     mquickjs_ridl_api.h, which only exists in the ridl variant.
# Both variants are regenerated here so the adapter release build never
# fails for lack of hand-made engine artifacts (Task 2a.1 follow-up).
# NOTE: this mirrors `ridl-builder prepare` minus its build_tools step,
# which is cargo-debug-profile-coupled (panics under PROFILE=release) and
# would rewrite the tracked .cargo/config.toml.
ensure_engine_variant() {
    local mode="$1"
    local variant_root="$REPO_ROOT/target/mquickjs-build/framework/$RUST_TARGET/$mode"
    if [ -f "$variant_root/base/mquickjs_build_output.json" ] \
        && [ -f "$variant_root/ridl/include/mquickjs_ridl_api.h" ]; then
        return 0
    fi
    echo "engine build outputs missing ($mode) — regenerating (base + ridl variants)"
    (
        cd "$REPO_ROOT"
        # base variant: engine build metadata + bindgen headers (ridl-free)
        TARGET="$RUST_TARGET" PROFILE="$mode" \
            cargo run -q -p ridl-builder -- build-mquickjs
        # ridl variant: aggregate RIDL modules, then build the engine with
        # the aggregated register header (same steps as `prepare` steps 3+4)
        local regen_log register_h
        regen_log="$(mktemp)"
        cargo run -q -p ridl-builder -- aggregate 2>"$regen_log" || {
            cat "$regen_log" >&2; rm -f "$regen_log"; exit 1; }
        register_h="$(sed -n 's|^wrote \(.*mquickjs_ridl_register\.h\)$|\1|p' "$regen_log" | tail -n1)"
        rm -f "$regen_log"
        if [ -z "$register_h" ]; then
            echo "ERROR: aggregate did not report mquickjs_ridl_register.h" >&2
            exit 1
        fi
        cargo run -q -p mquickjs-build -- build \
            --mquickjs-dir "$ENGINE_SRC" \
            --ridl-register-h "$register_h" \
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
    local img="$OPENVELA/nuttx/nuttx" st_m st_f st_c st_r st_sl
    if [ -f "$img" ]; then
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
    else
        echo "AUDIT: final image not built yet — archive-level checks only"
    fi

    if [ "$rc" -eq 0 ]; then
        echo "AUDIT: PASS"
    fi
    return $rc
}

build_rust_adapter() {
    ensure_engine_variant release
    echo "building rust adapter (release, $RUST_TARGET)"
    (cd "$RUST_DIR" && cargo build --release --target "$RUST_TARGET")
    # Stage for the apps flat-image link: arch/sim/src/Makefile does
    #   EXTRA_LIBS += $(wildcard $(APPDIR)/staging/*.a)
    # and links them inside LDSTARTGROUP/LDENDGROUP, so the adapter's
    # JS_*/js_stdlib references resolve from libapps.a regardless of order.
    mkdir -p "$STAGING_DIR"
    cp -f "$ADAPTER_A" "$STAGING_DIR/"
    echo "adapter staged: $STAGING_DIR/$(basename "$ADAPTER_A")"
    audit_symbols "$STAGING_DIR/$(basename "$ADAPTER_A")"
}

# 1) app dir symlink（apps 树标准接入点，纯新增）
ln -sfn "$APP_DIR" "$OPENVELA/apps/system/mqjs"

# 2) stage engine sources + headers for the apps build
mkdir -p "$ENGINE_STAGE"
for f in mquickjs.c cutils.c dtoa.c libm.c mqjs_stdlib.c \
         mquickjs.h mquickjs_priv.h cutils.h dtoa.h libm.h list.h; do
    cp -f "$ENGINE_SRC/$f" "$ENGINE_STAGE/$f"
done

# 2b) stdlib overlay — compile-time registration of the rs probe functions.
#     mquickjs 没有 QuickJS 式的运行时注册 API（JS_NewCFunctionParams 只能
#     按 stdlib 表下标造函数对象；AGENTS.md 核心约束：注册必须编译期），
#     因此两个桥接函数走引擎既有的 stdlib 声明表路径：把条目注入 **staged
#     副本**（本仓库构建产物，不碰 deps/mquickjs、不碰 openvela 树）的
#     js_global_object[]，step 3 的 host 工具据此把 rsVersion/rsSelfTest
#     烤进生成的 mqjs_stdlib.h，其表引用的 js_rs_* 钩子由 js_main.c 在
#     include 之前定义 —— 与 print/gc/load 完全同模式。
MQJS_STDLIB_STAGED="$ENGINE_STAGE/mqjs_stdlib.c"
if ! grep -q 'js_rs_version' "$MQJS_STDLIB_STAGED"; then
    OVERLAY_ANCHOR='JS_CFUNC_DEF("clearTimeout", 1, js_clearTimeout),'
    grep -qF "$OVERLAY_ANCHOR" "$MQJS_STDLIB_STAGED" || {
        echo "ERROR: stdlib overlay anchor not found — engine source drift, \
update the overlay in setup-sim.sh" >&2
        exit 1
    }
    sed -i "\|$OVERLAY_ANCHOR|a\\
    JS_CFUNC_DEF(\"rsVersion\", 0, js_rs_version),\\
    JS_CFUNC_DEF(\"rsSelfTest\", 0, js_rs_self_test)," "$MQJS_STDLIB_STAGED"
    echo "stdlib overlay applied: rsVersion/rsSelfTest -> $MQJS_STDLIB_STAGED"
fi

# 3) engine-native host toolchain: build the mqjs_stdlib host tool and
#    generate the atom-table headers (mirrors deps/mquickjs/Makefile rules)
#    — compiled from the OVERLAID staged copy so the generated stdlib
#    carries the rs probe functions (headers resolve from the pristine
#    engine source dir).
mkdir -p "$GEN_DIR"
HOST_FLAGS="-O2 -D_GNU_SOURCE -fno-math-errno -fno-trapping-math -I$ENGINE_SRC"
cc $HOST_FLAGS -c "$ENGINE_STAGE/mqjs_stdlib.c" -o "$GEN_DIR/mqjs_stdlib.host.o"
cc $HOST_FLAGS -c "$ENGINE_SRC/mquickjs_build.c" -o "$GEN_DIR/mquickjs_build.host.o"
cc $HOST_FLAGS -o "$GEN_DIR/mqjs_stdlib" \
    "$GEN_DIR/mqjs_stdlib.host.o" "$GEN_DIR/mquickjs_build.host.o"
"$GEN_DIR/mqjs_stdlib" -a > "$GEN_DIR/mquickjs_atom.h"
"$GEN_DIR/mqjs_stdlib" > "$GEN_DIR/mqjs_stdlib.h"
echo "atom headers generated: $GEN_DIR/mquickjs_atom.h mqjs_stdlib.h"
if ! grep -q 'js_rs_version' "$GEN_DIR/mqjs_stdlib.h"; then
    echo "ERROR: rs overlay missing from generated mqjs_stdlib.h" >&2
    exit 1
fi

# 4) composite defconfig（合成：openvela sim:nsh defconfig + 本移植片段）
#    目录形态必须为 <config>/defconfig（NuttX cmake 按
#    NUTTX_DEFCONFIG = <BOARD_CONFIG>/defconfig 解析），生成到本仓库
#    gen/，不动 openvela 的 defconfig。
BASE_DEFCONFIG="$OPENVELA/nuttx/boards/sim/sim/sim/configs/nsh/defconfig"
if [ ! -f "$BASE_DEFCONFIG" ]; then
    echo "ERROR: base defconfig not found at $BASE_DEFCONFIG" >&2
    exit 1
fi
mkdir -p "$GEN_DIR/nsh"
cp "$BASE_DEFCONFIG" "$GEN_DIR/nsh/defconfig"
cat >> "$GEN_DIR/nsh/defconfig" <<'EOF'

# mquickjs js builtin (ports/openvela) — composite defconfig, not in-tree
CONFIG_MQJS_JS=y
CONFIG_FS_HOSTFS=y
CONFIG_BOARDCTL=y
EOF

# 5) framework.mk（Make 轨消费的机器本地路径；CMake 轨直接用 gen/ 合成
#    defconfig，无需单独文件）
cat > "$APP_DIR/framework.mk" <<EOF
# Generated by ports/openvela/setup-sim.sh — do not commit.
MQJS_GEN_DIR := $GEN_DIR
MQJS_ENGINE_SRC := $ENGINE_SRC
EOF

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
    ;;
"build-cmake")
    # adapter 构建与轨道无关（产物进 apps/staging；CMake 轨的 .a 链接消费
    # 属 M2 范围，这里仅保证 release 变体可自动生成、staging 就绪不报错）
    build_rust_adapter
    # 与已验证成功的官方序列一致（distclean → rm cmake_out → lunch 静默
    # 完成全部 cmake configure → cmake --build），唯一差异是 lunch 前导出
    # VELA_BUILD_BOARD_CONFIG 指向本仓库的合成 defconfig 目录
    # （envsetup.sh:231 会采用该值），从而零改动 openvela 树。
    # 注意：gen/nsh/defconfig 的目录形态是 NuttX cmake 的解析契约
    # （NUTTX_DEFCONFIG = <BOARD_CONFIG>/defconfig）。
    if [ -f "$OPENVELA/nuttx/Makefile" ] || [ -f "$OPENVELA/nuttx/.config" ]; then
        echo "NOTE: switching tracks — distclean (build artifacts only)"
        make -C "$OPENVELA/nuttx" distclean >/dev/null 2>&1 || true
    fi
    rm -rf "$OPENVELA/cmake_out"

    export VELA_BUILD_BOARD_CONFIG="$COMPOSITE_DEFCONFIG_DIR"
    # envsetup 不兼容 nounset：在子 shell 中关闭后执行 lunch/构建
    (
        set +eu
        cd "$OPENVELA"
        source build/envsetup.sh
        lunch sim:nsh cmake_out/sim_nsh >/dev/null
        cmake --build cmake_out/sim_nsh -j"$(nproc)"
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
