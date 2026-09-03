#!/bin/bash
# Hook: PreToolUse (Edit|Write)
# 核心文件保护 — 修改保护文件时注入警告，要求 Claude 说明必要性

INPUT=$(cat /dev/stdin)
TOOL_NAME=$(echo "$INPUT" | jq -r '.tool_name // empty')
FILE_PATH=""

if [ "$TOOL_NAME" = "Edit" ]; then
  FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty')
elif [ "$TOOL_NAME" = "Write" ]; then
  FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty')
fi

[ -z "$FILE_PATH" ] && exit 0

# 核心保护文件列表（相对路径匹配）
PROTECTED_FILES=(
  "deps/mquickjs-rs/src/context.rs"
  "deps/mquickjs-rs/src/roots.rs"
  "deps/mquickjs-rs/src/lib.rs"
  "deps/mquickjs-rs/src/mod.rs"
  "deps/ridl-tool/src/main.rs"
  "deps/ridl-tool/src/aggregate.rs"
  "deps/ridl-tool/src/codegen.rs"
  "ridl-builder/src/main.rs"
  "AGENTS.md"
  "Cargo.toml"
)

for pattern in "${PROTECTED_FILES[@]}"; do
  if [[ "$FILE_PATH" == *"$pattern"* ]]; then
    cat <<EOF
⚠️ CORE FILE PROTECTION: 你正在修改核心保护文件 [$pattern]
此文件承载系统关键链路，改动影响范围大。请确认：
1. 变更是否已最小化范围？
2. commit message 中是否会说明修改必要性？
如非必要，请寻找其他方式实现需求。
EOF
    exit 0
  fi
done

exit 0
