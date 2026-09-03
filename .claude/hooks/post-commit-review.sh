#!/bin/bash
# Hook: PostToolUse (Bash)
# 检测新 commit 并提示运行 /post-commit-memory

INPUT=$(cat /dev/stdin)
TOOL_NAME=$(echo "$INPUT" | jq -r '.tool_name // empty')

[ "$TOOL_NAME" != "Bash" ] && exit 0

COMMAND=$(echo "$INPUT" | jq -r '.tool_input.command // empty')

# 检测 git commit 命令
if echo "$COMMAND" | grep -q "git commit"; then
  # 检查是否真的产生了新 commit
  if git rev-parse HEAD >/dev/null 2>&1; then
    cat <<EOF
📝 POST-COMMIT REMINDER: 检测到新 commit
建议运行 /post-commit-memory 沉淀本次 commit 的项目级知识。
如果这次 commit 包含：
- 架构决策、平台坑、代码约定
- 调试方法、已验证的取舍
请运行该命令判定是否该写入知识库。
EOF
  fi
fi

exit 0
