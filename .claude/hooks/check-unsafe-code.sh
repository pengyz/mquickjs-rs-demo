#!/bin/bash
# Hook: PreToolUse (Edit|Write)
# 检查 unsafe 代码是否有 Safety 注释

INPUT=$(cat /dev/stdin)
TOOL_NAME=$(echo "$INPUT" | jq -r '.tool_name // empty')
FILE_PATH=""

if [ "$TOOL_NAME" = "Edit" ]; then
  FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty')
  NEW_STRING=$(echo "$INPUT" | jq -r '.tool_input.new_string // empty')
elif [ "$TOOL_NAME" = "Write" ]; then
  FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty')
  NEW_STRING=$(echo "$INPUT" | jq -r '.tool_input.content // empty')
fi

# 只检查 Rust 文件
[[ "$FILE_PATH" != *.rs ]] && exit 0
[ -z "$NEW_STRING" ] && exit 0

# 检查新写入/编辑的内容是否包含 unsafe 块且缺少 Safety 注释
if echo "$NEW_STRING" | grep -q "unsafe\s*{"; then
  if ! echo "$NEW_STRING" | grep -q "//.*Safety\|/\*.*Safety"; then
    cat <<EOF
⚠️ UNSAFE CODE CHECK: 检测到 unsafe 块但缺少 Safety 注释
Rust unsafe 代码必须有注释说明：
1. 为什么这段代码是安全的？
2. 调用者需要保证什么前提条件？
请在 unsafe 块前添加 // Safety: <说明>
EOF
  fi
fi

# 检查是否使用了 JS_FreeValue/JS_DupValue（QuickJS 特定）
if echo "$NEW_STRING" | grep -q "JS_FreeValue\|JS_DupValue"; then
  cat <<EOF
⚠️ QUICKJS GC CHECK: 检测到手动管理 JSValue
本项目的 QuickJS 使用 tracing GC，不应手动调用：
- JS_FreeValue (会导致 double-free)
- JS_DupValue (不必要的引用计数)
请移除这些调用，让 GC 自动管理生命周期。
EOF
fi

exit 0
