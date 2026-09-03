---
name: knowledge-dream
description: 整理项目共享知识库 docs/knowledge/：读 kairos 增量为蒸馏素材、提议 promote/merge/删除候选、grep 验证、重建索引
user_invocable: true
allowed_tools: [Read, Write, Edit, Bash]
when_to_use: "用户要求整理知识库、dream、consolidate 项目记忆时；通常在累积若干 kairos 条目后定期触发"
---

# knowledge-dream — 项目知识库整理

## 触发方式

- `/knowledge-dream` — 完整 4 阶段
- "整理一下项目知识库" / "dream" / "consolidate knowledge"

## 执行流程

### Phase 1: Orient（建立全貌）

1. Read `docs/knowledge/MEMORY.md` 与 `docs/knowledge/README.md`
2. Read `docs/knowledge/RULES-KNOWLEDGE-MEMORY.md`
3. `ls docs/knowledge/*.md` — 统计每类 type 的文件数
4. 检查 `docs/knowledge/.consolidate-lock` mtime — 作为 since
5. 报告当前状态（按 type 分组的文件数 + MEMORY.md 行数 + 上次 dream 时间）

### Phase 2: Collect（收集增量素材）

**关键**：只读 kairos 增量，不读全库 commit log。

```bash
# 首次运行时创建 lock 文件（epoch 0），后续运行使用 -newer 过滤增量
[ -f docs/knowledge/.consolidate-lock ] || touch -t 197001010000 docs/knowledge/.consolidate-lock

# 列出 mtime > lock 的 kairos 文件（跨平台兼容）
find docs/knowledge/kairos -name "*.md" -newer docs/knowledge/.consolidate-lock 2>/dev/null

# 读取这些文件的全部 bullet 作为本次 dream 的"待蒸馏素材"
```

输出："本次将基于 N 条 kairos bullet 整理（自 YYYY-MM-DD HH:MM 以来）。"

如果没有增量 → 直接进 Phase 3 做现有条目的合并/验证，不强求新写入。

### Phase 3: Consolidate（提议候选）

**4 种候选清单**（每条都展示给用户确认，不直接改）：

#### A. Promote 候选（kairos → 正式条目）

扫 Phase 2 收集的 kairos bullet：哪些是"反复出现的事实/坑/约定"，应该升级为正式 `<type>_<slug>.md`？

**识别信号**：
- 同一坑被踩 2+ 次
- 同一设计决策被多次引用
- 排查路径被多次使用

起草条目（含 frontmatter + 正文），展示给用户确认。

#### B. Merge 候选（重复条目合并）

`rg` 扫描所有 `<type>_*.md` 的 description，识别语义重复的条目。

**识别信号**：
- 两个 `gotcha` 描述同一个 QuickJS API 的坑
- 两个 `pattern` 描述同一种代码约定的不同表述

建议合并方案，展示给用户确认。

#### C. Delete 候选（过时条目清理）

检查每个条目中提到的文件/函数/命令：

```bash
# 文件路径验证
grep -h "文件:" docs/knowledge/*.md | while read path; do
  [ -f "$path" ] || echo "STALE: $path (from <条目>)"
done

# 函数名验证
grep -h "函数:" docs/knowledge/*.md | while read fn; do
  rg -q "$fn" --type rust || echo "STALE: $fn (from <条目>)"
done
```

提议删除或更新过时条目。

#### D. 增强候选（缺失 Why/When）

扫描所有条目，识别：
- 缺少 **为什么** 段落的（reference 除外）
- 缺少 **何时使用** 段落的（reference 除外）
- **为什么** / **何时使用** 过于空泛的（"提高性能"、"避免错误"）

建议补充内容。

### Phase 4: Rebuild（重建索引）

用户确认 Phase 3 的候选后：

1. 执行写入/合并/删除操作
2. 重新扫描 `docs/knowledge/*.md`
3. 重建 `docs/knowledge/MEMORY.md`：
   - 按 type 分组
   - 每个条目一行：`- [标题](<type>_<slug>.md) — <description>`
   - 按 slug 字母序排列
4. 更新 `docs/knowledge/.consolidate-lock` 时间戳
5. 在 kairos 追加一行：`- HH:MM — knowledge-dream 整理完成：promote N / merge M / delete K`

## 输出格式

### Phase 1 输出示例

```
知识库现状：
- Architecture: 3 条
- Gotchas: 5 条
- Patterns: 2 条
- Debug: 1 条
- Decisions: 2 条
- References: 1 条
- MEMORY.md: 47 行
- 上次整理: 2026-09-01 10:30
```

### Phase 2 输出示例

```
收集增量素材：
本次将基于 12 条 kairos bullet 整理（自 2026-09-01 10:30 以来）。

增量文件：
- docs/knowledge/kairos/2026/09/2026-09-02.md
- docs/knowledge/kairos/2026/09/2026-09-03.md
```

### Phase 3 输出示例

```
提议候选：

[Promote] kairos → 正式条目
1. gotcha_quickjs_rom_ridl_conflict
   - kairos 中 2 次提到 ROM 机制与 RIDL 扩展冲突
   - 起草条目（见下方）

[Merge] 重复条目
1. pattern_ridl_edition 与 gotcha_ridl_edition_failure
   - 两者都描述 edition = "2024" 约定
   - 建议合并为 pattern_ridl_edition_2024

[Delete] 过时条目
1. reference_old_build_command
   - 引用的 xtask 已重命名为 ridl-builder

[Enhance] 缺失 Why/When
1. decision_root_over_global
   - **何时使用** 段落过于空泛，建议补充具体场景

确认执行以上操作？(y/n/partial)
```

## 注意事项

1. **增量为主**：优先处理 kairos 增量，不每次都扫全库
2. **用户主导**：所有候选都需用户确认，不自动执行
3. **验证为先**：删除前必须验证引用的文件/函数确实不存在
4. **保守合并**：不确定是否重复时，保留两个条目
