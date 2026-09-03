#!/bin/bash
# Hook: PreToolUse (Edit|Write)
# 检查 RIDL 模块规范

INPUT=$(cat /dev/stdin)
TOOL_NAME=$(echo "$INPUT" | jq -r '.tool_name // empty')
FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty')

# 只检查 RIDL 测试模块的 Cargo.toml
[[ "$FILE_PATH" != */tests/*/Cargo.toml ]] && exit 0
[[ "$FILE_PATH" != *test_* ]] && exit 0

if [ "$TOOL_NAME" = "Edit" ]; then
  NEW_STRING=$(echo "$INPUT" | jq -r '.tool_input.new_string // empty')
elif [ "$TOOL_NAME" = "Write" ]; then
  NEW_STRING=$(echo "$INPUT" | jq -r '.tool_input.content // empty')
fi

[ -z "$NEW_STRING" ] && exit 0

# 检查是否设置了 edition = "2024"
if ! echo "$NEW_STRING" | grep -q 'edition.*=.*"2024"'; then
  cat <<EOF
⚠️ RIDL MODULE CHECK: RIDL 模块缺少 edition = "2024"
根据项目约定（AGENTS.md），所有 RIDL 模块必须设置：
  edition = "2024"
否则可能导致宏展开失败。
EOF
fi

exit 0
