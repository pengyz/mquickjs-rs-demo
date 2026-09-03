---
name: post-commit-memory
description: commit 后判定是否产生项目级知识，是则起草条目让用户确认，否则仅追加一行 kairos
user_invocable: true
allowed_tools: [Read, Write, Edit, Bash]
when_to_use: "用户刚做完一次 commit、希望沉淀本次 commit 的项目级知识时；或对话中用户明确要求 /post-commit-memory"
---

# post-commit-memory — commit 后的判定写入

## 触发方式

- `/post-commit-memory` — 处理 HEAD commit
- `/post-commit-memory <commit-hash>` — 处理指定 commit
- 对话中用户提到刚 commit 的内容并希望沉淀

## 执行流程

### Phase 1: 收集

1. `git show <commit> --stat`（默认 HEAD）— 看变更面
2. `git log -1 --format=%B <commit>` — 拿 commit message
3. 必要时 `git show <commit> -- <file>` 看具体 diff
4. Read `docs/knowledge/MEMORY.md` 与 `docs/knowledge/README.md` 了解现有索引与规则
5. Read `docs/knowledge/RULES-KNOWLEDGE-MEMORY.md` 复习 6 类 type 与排除项

### Phase 2: 判定

按下列顺序排除"不该写正式条目"的内容：

- 仅修复 bug、commit message 已自解释 → 跳过正式条目，仅 kairos
- 改动是 git log/git blame 已能查到的"X 改成 Y" → 跳过
- AGENTS.md 或现有 docs/knowledge/ 已覆盖 → 跳过
- 仅个人偏好 → 写个人 memory 不写这里
- 临时调试状态 / 进行中工作 → 跳过

剩下的内容按"该写"清单判断：

- 架构决策被确认 → `architecture`
- 平台坑被踩（QuickJS/Rust FFI/Cargo/构建系统）→ `gotcha`
- 代码约定被确立（RIDL 模块/测试开发模式）→ `pattern`
- 排查路径被验证 → `debug`
- 已验证的取舍（含正反馈）→ `decision`
- 外部资源指针 → `reference`

判定标准：另一个工程师的 AI 实例遇到同样场景时，这条能帮它少走弯路吗？不能 → 跳过。

### Phase 3: 输出

**情况 A — 有项目级知识**：

1. 起草条目 frontmatter + 正文（包含 **为什么** 和 **何时使用**）
2. 展示给用户确认
3. 用户确认后：
   - 写入 `docs/knowledge/<type>_<slug>.md`
   - 在 `docs/knowledge/MEMORY.md` 追加索引行
   - 在 `docs/knowledge/kairos/YYYY/MM/YYYY-MM-DD.md` 追加事件

**情况 B — 无项目级知识**：

仅在 `docs/knowledge/kairos/YYYY/MM/YYYY-MM-DD.md` 追加一行：

```
- HH:MM — commit <short-hash>：<commit message 首行摘要>
```

### Phase 4: 报告

简短总结：
- 情况 A：写入了哪个 type 的条目、slug 是什么
- 情况 B：已记录到 kairos，无需正式条目

## 示例

### 示例 1：有项目级知识

```
Commit: e40763a "test: add engine gc selftest to README"

判定：这是一个 reference，记录了重要的测试命令。

起草条目：
---
name: selftest-gc-mark-command
description: 引擎 GC 标记自检命令，验证 gc_mark 回调正确性
type: reference
created: 2026-09-03
sources: [e40763a]
---

引擎 GC 标记自检命令：`cargo run -p ridl-builder -- selftest-gc-mark`

用于验证引擎层 gc_mark 回调是否正确枚举 JSValue 引用。

确认写入？(y/n)
```

### 示例 2：无项目级知识

```
Commit: b9bd9da "chore: fmt"

判定：仅格式化代码，无项目级知识。

已追加到 kairos/2026/09/2026-09-03.md：
- 14:30 — commit b9bd9da：chore: fmt
```

## 注意事项

1. **保守判定**：不确定是否该写时，选择"仅 kairos"
2. **查重优先**：写入前检查 MEMORY.md，有相似条目则建议更新而非新建
3. **正文质量**：**为什么** 和 **何时使用** 必须具体，避免空泛
4. **用户确认**：起草后必须等用户确认，不自动写入
