# 项目共享知识库

本目录存储项目级的共享知识，供所有协作者（人类和 AI）参考。

## 目的

当另一个工程师的 AI 实例遇到同样场景时，这里的知识能帮它少走弯路。

## 知识类型

### architecture
设计决策、模块边界、为什么选 A 不选 B。

**何时写入**：方案被确认采纳、解释了非显然的取舍、读代码看不出"为什么"时。

**示例**：
- GC Root + Traced 统一 tracing 设计
- base vs ridl 两套输出的原因
- RIDL 模块检测机制

### gotcha
平台坑、反直觉行为（QuickJS/Rust FFI/构建系统/Cargo）。

**何时写入**：触碰某 API/平台导致非预期结果、找到 workaround 时。

**示例**：
- QuickJS ROM 机制与 RIDL 扩展的关系
- Rust FFI 生命周期陷阱
- Cargo workspace member 意外行为

### pattern
代码约定、开发模式、RIDL 模块/测试的最佳实践。

**何时写入**：同类问题第二次出现、确立项目约定、发现反模式时。

**示例**：
- RIDL 模块开发规范
- GC 测试验证模式（finalizer + cycle）
- 引擎自检用例编写约定

### debug
调试方法、日志位置、关键命令、排查路径。

**何时写入**：排查中发现高效诊断手段、定位关键输出时。

**示例**：
- GC 标记追踪方法
- RIDL 生成代码调试
- 引擎层 panic 定位

### decision
已验证的取舍。**包括正反馈**：确认非常规选择有效时也要记录。

**何时写入**：接受了非显而易见的方案选择且未推翻、做完后回头确认走对了路时。

**示例**：
- Root<T> 优于 Global<T> 的选择
- Context-level gc_mark 而非每个 Root 单独注册
- RootsRegistry 用 Mutex<Vec<Option>> 而非其他结构

### reference
外部资源指针：文档、内部工具、关键文件位置。

**何时写入**：提到外部资源及其用途时。

**示例**：
- QuickJS 官方文档关键章节
- ridl-builder prepare 命令说明
- 关键测试用例位置

## 不写入

- 代码本身能表达的（函数签名、文件结构、模块依赖） → 读代码
- `git log` / `git blame` 能查到的 → 那里是权威
- `AGENTS.md` 已有的规则 → 不重复
- 仅对当前用户有用的偏好 → 写个人 memory
- 临时调试状态、进行中的工作 → 不持久化
- 修复某 bug 的具体过程 → 修复在代码中，commit message 有上下文

即使明确要求保存上述内容，也要先反问"其中有什么*非显然*的部分？" — 那部分才值得记。

## 文件结构

每条知识 1 个文件：`<type>_<slug>.md`

### Frontmatter

```markdown
---
name: kebab-case-slug
description: 一行钩子，决定未来召回相关性
type: architecture | gotcha | pattern | debug | decision | reference
created: YYYY-MM-DD
sources: [commit-hash 或 session-id]
---

<事实陈述，一句>

**为什么：** ...
**何时使用：** ...
```

`reference` 类型可省 Why/When。其他五类必填。

### 索引

所有条目在 `MEMORY.md` 中索引，按 type 分组：

```markdown
## Architecture
- [标题](architecture_slug.md) — 一句钩子
```

### Kairos 日志（事件流）

`kairos/YYYY/MM/YYYY-MM-DD.md` 记录 commit/会话级事件流，作为 dream 整理的增量素材。

格式：`- HH:MM — <动作>：<一句话>`

## 规则文档

- `RULES-KNOWLEDGE-MEMORY.md` - 知识提取规则（何时写、如何写）
- `RULES-CODE-QUALITY.md` - 代码质量检查规则（Rust 生态适配）

## 使用

### 写入时机

1. **commit 后** — 运行 `/post-commit-memory` 判定是否产生项目级知识
2. **用户纠正方法** — "别这样做"、"不是这个" → 多为 pattern / gotcha
3. **用户确认非显然方法有效** — "对，就这么做" → decision（正反馈也写）
4. **遇到平台坑/反直觉行为** — gotcha / debug

### 整理

定期运行 `/knowledge-dream` 整理知识库：
1. 收集 kairos 增量
2. 提议 promote（kairos → 正式条目）、merge（合并重复）、删除（过时）候选
3. 重建 MEMORY.md 索引

### 引用前验证

引用 docs/knowledge/ 中提到的具体文件/函数/命令前，必须验证：
- 文件路径 → Read 或 ls 验证存在
- 函数/类名 → grep 验证未被重命名/删除
- 命令/flag → 在 help 输出或 README 中验证

"docs/knowledge/ 说 X 存在" ≠ "X 现在存在"。验证失败 → 不引用 + 标记为失效候选。

## 约束

- 单条目正文 ≤ 30 行；接近上限时拆分或精简
- `MEMORY.md` ≤ 200 行 / 25KB；接近上限删最旧/最不引用的条目
- 写入前先查重；更新优先于新增
- `decision` / `pattern` 类条目正反并存，不要让知识库只剩"避雷指南"

## 与 AGENTS.md 的边界

| 内容 | 去处 |
|------|------|
| "RIDL 模块必须 edition = 2024" | AGENTS.md（规则） |
| "忘记设 edition 导致宏展开失败" | knowledge/gotcha |
| "代码必须通过测试" | AGENTS.md（规则） |
| "GC root cycle 测试用 finalizer 验证回收" | knowledge/pattern |
| "不允许硬编码" | AGENTS.md（约束） |
| "某次硬编码导致多 app 冲突" | knowledge/gotcha |

**AGENTS.md**：项目规则、约束、流程（源头真理）  
**knowledge/**：踩坑经验、设计上下文、调试方法
