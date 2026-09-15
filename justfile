# mquickjs-rs 构建和测试命令
# 用法: just <command>

# 默认命令
default: build

# 检查环境
doctor:
    @bash scripts/doctor.sh

# 构建项目
build:
    cargo run -p ridl-builder -- prepare
    cargo build

# 运行所有测试
#
# 逐包运行，便于定位失败来源；`cargo test --workspace` 现在同样是全绿的
# （变体选择已下沉到叶子二进制，见
#  docs/knowledge/architecture_base_vs_ridl_variant_selection.md）。
test: test-ridl test-mquickjs test-demo test-js

# 运行整个 workspace 的测试（等价覆盖，一次跑完）
test-workspace:
    cargo test --workspace

# 裸机可行性检查（实验性）
#
# 为 64 位裸机目标构建 C 引擎 + 以 no_std 构建 Rust 核心。
# 用 64 位目标是因为 mquickjs 的 JS_PTR64 硬编码为 64 位字长
# （见 docs/knowledge/assessment_core_nostd_port_cost.md）。
#
# 需要：rustup target add aarch64-unknown-none
nostd-check:
    cargo run -q -p mquickjs-build -- build \
        --mquickjs-dir deps/mquickjs \
        --target aarch64-unknown-none \
        --out target/mquickjs-build/framework/aarch64-unknown-none/debug/base
    cargo build -p mquickjs-rs \
        --target aarch64-unknown-none \
        --no-default-features --features no-std

# 运行 RIDL 工具链测试
test-ridl:
    cargo test -p ridl-tool

# 运行 mquickjs-rs 绑定测试
test-mquickjs:
    cargo test -p mquickjs-rs

# 运行 demo 应用测试
test-demo:
    cargo test -p mquickjs-demo

# 运行 JS 集成测试
test-js:
    cargo run -- tests

# 运行特定测试
test-async:
    cargo test -p mquickjs-rs --test async_stream
    cargo test -p mquickjs-rs --test async_value
    cargo test -p mquickjs-rs --test async_error

# 清理构建产物
clean:
    cargo clean
    rm -rf target/ridl
    rm -rf target/mquickjs-build

# 重新构建
rebuild: clean build

# 运行 demo 应用
run:
    cargo run --bin mquickjs-demo

# 生成 RIDL 聚合
aggregate:
    cargo run -p ridl-builder -- aggregate

# 构建工具
build-tools:
    cargo run -p ridl-builder -- build-tools

# 构建 mquickjs
build-mquickjs:
    cargo run -p ridl-builder -- build-mquickjs

# 检查代码
check:
    cargo check
    cargo clippy

# 格式化代码
fmt:
    cargo fmt

# 运行所有检查
lint: check fmt

# 创建新应用（需要指定名称）
new-app name:
    mkdir -p apps/{{name}}/src
    @echo "创建应用: {{name}}"
    @echo "请参考 QUICKSTART.md 完成配置"

# 创建新模块（需要指定名称）
new-module name:
    mkdir -p tests/global/{{name}}/test_{{name}}/src
    @echo "创建模块: {{name}}"
    @echo "请参考 QUICKSTART.md 完成配置"

# 显示帮助
help:
    @echo "mquickjs-rs 构建命令"
    @echo ""
    @echo "用法: just <command>"
    @echo ""
    @echo "命令:"
    @echo "  doctor        检查环境"
    @echo "  build         构建项目"
    @echo "  test          运行所有测试（逐包）"
    @echo "  test-workspace 运行整个 workspace 的测试"
    @echo "  nostd-check   裸机可行性检查（实验性）"
    @echo "  test-ridl     运行 RIDL 工具链测试"
    @echo "  test-mquickjs 运行 mquickjs-rs 绑定测试"
    @echo "  test-demo     运行 demo 应用测试"
    @echo "  test-js       运行 JS 集成测试"
    @echo "  test-async    运行异步相关测试"
    @echo "  clean         清理构建产物"
    @echo "  rebuild       重新构建"
    @echo "  run           运行 demo 应用"
    @echo "  aggregate     生成 RIDL 聚合"
    @echo "  build-tools   构建工具"
    @echo "  build-mquickjs 构建 mquickjs"
    @echo "  check         检查代码"
    @echo "  fmt           格式化代码"
    @echo "  lint          运行所有检查"
    @echo "  new-app       创建新应用"
    @echo "  new-module    创建新模块"
    @echo "  help          显示帮助"