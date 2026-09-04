#!/bin/bash
# mquickjs-rs 环境检查脚本
# 用法: bash scripts/doctor.sh

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# 检查结果
ERRORS=0
WARNINGS=0

# 辅助函数
check_ok() {
    echo -e "  ${GREEN}✓${NC} $1"
}

check_warn() {
    echo -e "  ${YELLOW}⚠${NC} $1"
    WARNINGS=$((WARNINGS + 1))
}

check_error() {
    echo -e "  ${RED}✗${NC} $1"
    ERRORS=$((ERRORS + 1))
}

echo "=========================================="
echo "  mquickjs-rs 环境检查"
echo "=========================================="
echo ""

# 1. 检查 Rust
echo "检查 Rust 工具链..."
if command -v rustc &> /dev/null; then
    RUST_VERSION=$(rustc --version | grep -oP '\d+\.\d+\.\d+')
    RUST_MAJOR=$(echo $RUST_VERSION | cut -d. -f1)
    RUST_MINOR=$(echo $RUST_VERSION | cut -d. -f2)
    
    if [ "$RUST_MAJOR" -ge 1 ] && [ "$RUST_MINOR" -ge 70 ]; then
        check_ok "Rust 版本: $RUST_VERSION (>= 1.70)"
    else
        check_error "Rust 版本过低: $RUST_VERSION (需要 >= 1.70)"
    fi
else
    check_error "未找到 Rust，请安装: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
fi

# 2. 检查 C 编译器
echo ""
echo "检查 C 编译器..."
if command -v gcc &> /dev/null; then
    GCC_VERSION=$(gcc --version | head -n1)
    check_ok "GCC: $GCC_VERSION"
elif command -v clang &> /dev/null; then
    CLANG_VERSION=$(clang --version | head -n1)
    check_ok "Clang: $CLANG_VERSION"
else
    check_error "未找到 C 编译器，请安装 gcc 或 clang"
fi

# 3. 检查 Git
echo ""
echo "检查 Git..."
if command -v git &> /dev/null; then
    GIT_VERSION=$(git --version)
    check_ok "$GIT_VERSION"
else
    check_error "未找到 Git"
fi

# 4. 检查 submodule
echo ""
echo "检查 Git submodule..."
if [ -d "deps/mquickjs/.git" ] || [ -f "deps/mquickjs/.git" ]; then
    check_ok "mquickjs submodule 已初始化"
else
    check_error "mquickjs submodule 未初始化，请运行: git submodule update --init --recursive"
fi

# 5. 检查 libclang（可选）
echo ""
echo "检查 libclang（可选）..."
if command -v llvm-config &> /dev/null; then
    LLVM_VERSION=$(llvm-config --version)
    check_ok "LLVM: $LLVM_VERSION"
elif [ -f "/usr/lib/libclang.so" ] || [ -f "/usr/lib/x86_64-linux-gnu/libclang.so" ]; then
    check_ok "libclang 已安装"
else
    check_warn "未找到 libclang，bindgen 可能无法工作"
    echo "    Ubuntu/Debian: sudo apt install libclang-dev"
    echo "    macOS: brew install llvm"
fi

# 6. 检查 just（可选）
echo ""
echo "检查 just（可选）..."
if command -v just &> /dev/null; then
    check_ok "just 已安装"
else
    check_warn "未找到 just（可选），安装: cargo install just"
fi

# 7. 检查构建输出
echo ""
echo "检查构建输出..."
if [ -d "target/ridl" ]; then
    check_ok "RIDL 构建输出存在"
else
    check_warn "RIDL 构建输出不存在，请运行: cargo run -p ridl-builder -- prepare"
fi

# 8. 检查 mquickjs 构建输出
echo ""
echo "检查 mquickjs 构建输出..."
if [ -d "target/mquickjs-build" ]; then
    check_ok "mquickjs 构建输出存在"
else
    check_warn "mquickjs 构建输出不存在，请运行: cargo run -p ridl-builder -- prepare"
fi

# 总结
echo ""
echo "=========================================="
echo "  检查完成"
echo "=========================================="
echo ""

if [ $ERRORS -gt 0 ]; then
    echo -e "${RED}发现 $ERRORS 个错误，$WARNINGS 个警告${NC}"
    echo "请修复错误后重试。"
    exit 1
elif [ $WARNINGS -gt 0 ]; then
    echo -e "${YELLOW}发现 $WARNINGS 个警告${NC}"
    echo "警告不会阻止构建，但可能影响某些功能。"
    exit 0
else
    echo -e "${GREEN}所有检查通过！${NC}"
    echo "可以开始构建: just build"
    exit 0
fi