# 项目共享记忆 — 提取规则

面向 Claude / 编程助手实例：会话中遇到下面四个时刻之一，按本规则写入 `docs/knowledge/`。

## 写入时机（4 个时刻）

1. **commit 后** — 用户主动在对话中提及刚 commit 的内容、或运行了 post-commit 相关 skill 时
2. **用户纠正方法** — "别这样做"、"不是这个" → 多为 `pattern` / `gotcha`
3. **用户确认非显然方法有效** — "对，就这么做"、接受了你的非常规选择 → `decision`，**正反馈也写**，避免知识库只剩避雷
4. **遇到平台坑/反直觉行为** — `gotcha` / `debug`

判断标准：另一个工程师的 AI 实例遇到同样场景时，这条知识能帮它少走弯路吗？不能 → 不写。

## 类型

<types>
<type>
    <name>architecture</name>
    <description>设计决策、模块关系、为什么选 A 不选 B。</description>
    <when_to_save>方案被确认采纳、用户解释了一个非显然的取舍、读代码看不出"为什么"时</when_to_save>
    <body_structure>事实陈述（一句），然后是 **为什么：** 行（机制/约束/被否决的备选）和 **何时使用：** 行（做什么相关改动时该回想此条）。</body_structure>
    <examples>
    - GC Root + Traced 统一 tracing 设计：为什么用 context-level gc_mark 而非每个 Root 单独注册
    - base vs ridl 两套 QuickJS 输出：为什么需要这种拆分，何时选哪个
    - RIDL 模块检测机制：为什么按 src/*.ridl 存在判定而非 Cargo.toml 字段
    </examples>
</type>
<type>
    <name>gotcha</name>
    <description>平台坑、反直觉行为（QuickJS/Rust FFI/构建系统/Cargo）。</description>
    <when_to_save>触碰某 API/平台导致非预期结果、找到 workaround 时</when_to_save>
    <body_structure>现象陈述，然后是 **为什么：** 行（底层机制）和 **何时使用：** 行（什么场景下要警惕此坑）。</body_structure>
    <examples>
    - QuickJS ROM 机制与 RIDL 扩展冲突：当初实现时未充分理解 ROM，需要重新审视
    - Rust FFI 生命周期标注：裸指针跨 FFI 边界的生命周期陷阱
    - Cargo workspace member 意外重新编译：dep 路径写法影响增量构建
    </examples>
</type>
<type>
    <name>pattern</name>
    <description>代码约定、开发模式、RIDL 模块/测试的最佳实践。</description>
    <when_to_save>同类问题第二次出现、确立项目约定、发现 RIDL/测试开发反模式时</when_to_save>
    <body_structure>规则陈述，然后是 **为什么：** 行（避免什么）和 **何时使用：** 行（写什么类型代码时套用）。</body_structure>
    <examples>
    - RIDL 模块必须设 edition = "2024"：避免宏展开失败
    - GC 测试用 finalizer 验证回收：确保环可回收的标准验证方法
    - 引擎自检用例编写约定：selftest-gc-mark 的用例结构
    </examples>
</type>
<type>
    <name>debug</name>
    <description>调试方法、日志位置、关键命令、排查路径。</description>
    <when_to_save>排查中发现一个高效的诊断手段、定位到关键输出/命令时</when_to_save>
    <body_structure>排查动作（命令/路径），然后是 **为什么：** 行（症状映射到此动作的逻辑）和 **何时使用：** 行（出现什么症状时启用）。</body_structure>
    <examples>
    - GC 标记追踪：QUICKJS_GC_DEBUG=1 环境变量开启详细输出
    - RIDL 生成代码调试：查看 target/<profile>/build/*/out/ 下的生成文件
    - 引擎层 panic 定位：RUST_BACKTRACE=1 + 检查 mquickjs-sys FFI 边界
    </examples>
</type>
<type>
    <name>decision</name>
    <description>已验证的取舍。包括正反馈：用户确认你的非常规选择有效。</description>
    <when_to_save>用户接受了一个非显而易见的方案选择且未推翻、做完后回头确认走对了路时</when_to_save>
    <body_structure>选择陈述，然后是 **为什么：** 行（为什么这个选项胜出）和 **何时使用：** 行（什么场景下重复此选择）。</body_structure>
    <examples>
    - Root<T> 优于 Global<T>：跨 async 持有推荐 Root，Global 保留兼容
    - RootsRegistry 用 Mutex<Vec<Option>>：相比 Slab 更简单且性能足够
    - Context-level gc_mark 注册：一次注册遍历所有 roots，优于每个 Root 单独注册
    </examples>
</type>
<type>
    <name>reference</name>
    <description>外部资源指针：文档、内部工具、关键文件位置。</description>
    <when_to_save>用户提到一个外部资源及其用途时</when_to_save>
    <body_structure>资源位置 + 一句用途说明。无需 Why/When。</body_structure>
    <examples>
    - QuickJS 官方文档 GC 章节：https://bellard.org/quickjs/
    - ridl-builder prepare 命令：构建前准备，生成 RIDL 聚合代码
    - 引擎自检命令：cargo run -p ridl-builder -- selftest-gc-mark
    </examples>
</type>
</types>

## 不写入

- 代码本身能表达的（函数签名、文件结构、模块依赖）→ 读代码
- `git log` / `git blame` 能查到的 → 那里是权威
- `AGENTS.md` 已有的规则 → 不重复
- 仅对当前用户有用的偏好（"别在末尾总结"）→ 写个人 memory
- 临时调试状态、进行中的工作 → 不持久化
- 修复某 bug 的具体过程 → 修复在代码中，commit message 有上下文

即使用户明确要求保存上述内容，也要先反问"其中有什么*非显然*的部分？" — 那部分才值得记。

## 文件结构

每条知识 1 个文件：`docs/knowledge/<type>_<slug>.md`，type 取自上面六类。

frontmatter：

```markdown
---
name: <kebab-case-slug>
description: <一行钩子，决定未来召回相关性>
type: architecture | gotcha | pattern | debug | decision | reference
created: YYYY-MM-DD
sources: [<commit-hash 或 session-id>]
---

<事实陈述，一句>

**为什么：** ...
**何时使用：** ...
```

`reference` 类型可省 Why/When。其他五类必填。

## 写入流程

1. Read `docs/knowledge/MEMORY.md` 与 `docs/knowledge/README.md`
2. 用 description 和现有条目对照 → 已有相似条目则**更新**，不新建
3. 写入 `docs/knowledge/<type>_<slug>.md`
4. 在 `docs/knowledge/MEMORY.md` 对应 type 段落追加一行索引：`- [标题](<type>_<slug>.md) — <一句钩子>`
5. 同步追加一行到 `docs/knowledge/kairos/YYYY/MM/YYYY-MM-DD.md`：`- HH:MM — 写入 <type>_<slug>：<一句>`

## Kairos 日志（事件流）

`docs/knowledge/kairos/YYYY/MM/YYYY-MM-DD.md` 是 commit/会话级事件流，作为蒸馏增量素材。

写入时机：
- 每次写入 knowledge 条目 → 同步追加一行
- 没产生条目但 commit 有"项目级动作"（架构调整、新模块、依赖变更）→ 仅追加一行 kairos，不写正式条目

格式：`- HH:MM — <动作>：<一句话>`，单文件 ≤ 100KB。

## 引用前的验证（防 stale）

引用 docs/knowledge/ 中提到的具体文件 / 函数 / 命令前，必须：

- 文件路径 → Read 或 ls 验证存在
- 函数 / 类名 → grep 验证未被重命名 / 删除
- 命令 / flag → 在 help 输出或 README 中验证

"docs/knowledge/ 说 X 存在" ≠ "X 现在存在"。验证失败 → 不引用 + 标记为失效候选。

## 示例：该写

- 发现 Root<T> Drop 时未从 registry 移除导致内存泄漏 → `gotcha_root_drop_registry_leak.md`
- 用户确认 RootsRegistry 用 Mutex<Vec<Option>> 而非 Slab 的选择 → `decision_roots_registry_vec_option.md`
- 第二次遇到 RIDL 模块忘记设 edition 导致编译失败 → `pattern_ridl_edition_2024.md`
- 排查 GC 发现 QUICKJS_GC_DEBUG 环境变量可输出详细追踪 → `debug_gc_trace_env_var.md`
- QuickJS 的 ROM 机制与 RIDL 扩展的关系被确认 → `architecture_ridl_rom_mechanism.md`

## 示例：不该写

- `context.rs` 第 25 行定义了 ContextInner → 不写（读代码即知）
- 用户说"别在回复末尾总结" → 个人 memory
- 修了一个 Root<T> 的 use-after-free bug → 不写（修复在代码，commit msg 有上下文）
- `cargo run -p ridl-builder -- prepare` 能准备构建 → 不写（README.md 已提）
- 完整的 GC root 架构图 → `docs/planning/` 或更新现有文档

## 约束

- 单条目正文 ≤ 30 行；接近上限时拆分或精简
- `MEMORY.md` ≤ 200 行 / 25KB；接近上限删最旧/最不引用的条目
- 写入前先查重；更新优先于新增
- `decision` / `pattern` 类条目正反并存，不要让知识库只剩"避雷指南"

## 与 AGENTS.md 的边界

| 内容 | 去处 |
|------|------|
| "RIDL 模块必须 edition = 2024" | AGENTS.md（规则） |
| "忘记设 edition 导致宏展开失败" | docs/knowledge/gotcha |
| "代码必须通过测试" | AGENTS.md（规则） |
| "GC root cycle 测试用 finalizer 验证回收" | docs/knowledge/pattern |
| "不允许硬编码" | AGENTS.md（约束） |
| "硬编码模块名导致多 app 冲突的一次事故" | docs/knowledge/gotcha |

**AGENTS.md**：项目规则、约束、流程（源头真理）  
**docs/knowledge/**：踩坑经验、设计上下文、调试方法
