# 规划：迁移 osbot AI 协作体系到 mquickjs-rs-demo

日期：2026-09-03

## 背景

osbot 项目建立了成熟的 AI 协作体系，包括：
1. Knowledge Memory 系统（6 类知识 + kairos 事件流）
2. Skill 系统（post-commit-memory、knowledge-dream）
3. Hook 系统（核心文件保护、安全检查）
4. 代码质量规则（安全、健壮性、性能、可维护性）

当前 mquickjs-rs-demo 仅有 AGENTS.md 单文件规则，缺乏：
- 结构化知识沉淀机制
- 自动化守护（hooks）
- 知识整理工具（dream）
- 代码质量自动检查

## 目标

将 osbot 的 AI 协作体系适配到 Rust/QuickJS 项目特点，建立：
1. 项目共享知识库（docs/knowledge/）
2. 自动化知识沉淀工具（skills）
3. 开发安全守护（hooks）
4. Rust 生态特定的质量规则

## 迁移范围

### Phase 1: 知识库基础设施（必须）

#### 1.1 目录结构

```
docs/knowledge/
├── MEMORY.md                    # 知识索引（≤200 行）
├── README.md                    # 使用说明
├── .consolidate-lock            # dream 时间戳
├── kairos/                      # 事件流
│   └── 2026/
│       └── 09/
│           └── 2026-09-03.md
└── <type>_<slug>.md            # 知识条目
```

#### 1.2 知识类型（6 类）

- **architecture**: 设计决策（GC root 设计、RIDL 机制）
- **gotcha**: 平台坑（QuickJS API、Rust FFI、构建系统）
- **pattern**: 代码约定（RIDL 模块开发、测试模式）
- **debug**: 调试方法（GC 追踪、引擎自检）
- **decision**: 已验证取舍（Root vs Global、Traced 字段设计）
- **reference**: 外部资源（文档、工具、关键文件）

#### 1.3 核心文件

```
.claude/docs/
├── RULES-KNOWLEDGE-MEMORY.md   # 知识提取规则
└── RULES-CODE-QUALITY.md       # 代码质量规则（Rust 适配）
```

### Phase 2: Skills（核心工具）

#### 2.1 post-commit-memory

```
.claude/skills/post-commit-memory/
└── skill.md
```

**功能**：commit 后自动判定是否产生项目级知识
- 读取 HEAD commit 的 diff 和 message
- 按 6 类判定是否该写正式条目
- 有项目级知识 → 起草条目；否则 → 仅追加 kairos

**判定标准**：
- 排除：代码可表达、git log 能查、AGENTS.md 已有、临时调试
- 该写：架构决策确认、踩坑 + workaround、约定确立、取舍验证

#### 2.2 knowledge-dream

```
.claude/skills/knowledge-dream/
└── skill.md
```

**功能**：整理知识库（4 阶段）
1. Orient：统计现状（按 type 分组的文件数 + 上次 dream 时间）
2. Collect：收集 kairos 增量（mtime > .consolidate-lock）
3. Consolidate：提议 promote/merge/删除候选
4. Rebuild：重建 MEMORY.md 索引

### Phase 3: Hooks（自动化守护）

#### 3.1 核心文件保护

```bash
.claude/hooks/protect-core-files.sh
```

保护文件列表（Rust 项目特定）：
- `deps/mquickjs-rs/src/context.rs`
- `deps/mquickjs-rs/src/roots.rs`
- `deps/mquickjs-rs/src/lib.rs`
- `deps/ridl-tool/src/*.rs`（核心生成逻辑）
- `ridl-builder/src/main.rs`

修改时注入警告，要求说明必要性。

#### 3.2 安全检查

```bash
.claude/hooks/check-unsafe-code.sh
```

Rust 特定安全检查：
- `unsafe` 块必须有安全注释
- FFI 调用必须有 Safety 文档
- 裸指针操作必须说明生命周期保证

#### 3.3 RIDL 规范检查

```bash
.claude/hooks/check-ridl-conventions.sh
```

检查：
- RIDL 模块 `edition = "2024"`
- class id 命名规范（ALL_CAPS）
- module 路径归一化

#### 3.4 commit 后审查触发

```bash
.claude/hooks/post-commit-review.sh
```

检测新 commit 并提示运行 `/post-commit-memory`。

#### 3.5 settings.json 配置

```json
{
  "language": "chinese",
  "permissions": {
    "ask": [
      "Bash(git push:*)",
      "Bash(rm -rf:*)",
      "Bash(cargo clean:*)"
    ]
  },
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Edit|Write",
        "hooks": [
          {"type": "command", "command": "...protect-core-files.sh"},
          {"type": "command", "command": "...check-unsafe-code.sh"},
          {"type": "command", "command": "...check-ridl-conventions.sh"}
        ]
      }
    ],
    "PostToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          {"type": "command", "command": "...post-commit-review.sh"}
        ]
      }
    ]
  }
}
```

### Phase 4: 代码质量规则（Rust 适配）

`.claude/docs/RULES-CODE-QUALITY.md`

#### 4.1 安全类（Rust 特定）

- **unsafe 块无注释**：`rg "unsafe\s*\{" --after 0 | rg -v "//.*Safety"`
- **裸指针解引用**：`rg "\*\s*(mut\s+)?[a-z_]+ as \*" -g '*.rs'`
- **transmute 使用**：`rg "std::mem::transmute|transmute::<" -g '*.rs'`
- **FFI 函数缺 Safety**：检查 `extern "C"` 函数文档

#### 4.2 健壮性（Rust 特定）

- **unwrap/expect**：`rg "\.unwrap\(\)|\.expect\(" -g '*.rs'`（已有）
- **panic!/unreachable!**：`rg "panic!|unreachable!" -g '*.rs'`
- **索引越界风险**：`rg "\[[0-9]+\]|\[.*\.\.]" -g '*.rs'`
- **生命周期标注缺失**：`rg "fn.*<'_>" -g '*.rs'`

#### 4.3 QuickJS 特定

- **JSValue 手动 free**：`rg "JS_FreeValue|JS_DupValue" -g '*.rs'`（引擎用 tracing GC，不应手动管理）
- **跨 Context 使用 Root**：检查 `Root::new` 的 ctx_id 校验
- **Global 滥用**：提示优先使用 `Root<T>`

#### 4.4 RIDL 生成代码

- **硬编码模块名**：`rg "\"GLOBAL\"|\"console\"" -g 'ridl-*.rs'`
- **注册函数漏调**：检查生成的 `register_*` 是否在 main 调用

### Phase 5: 初始化知识库

#### 5.1 从现有文档提取

将 `docs/planning/2026/2026-01-25-v1-c3-mquickjs-rs-root-and-traced.md` 转化为：

- `architecture_gc_root_traced_unified_tracing.md` - Root<T> + Traced<T> 设计
- `decision_root_over_global.md` - 为什么推荐 Root 而非 Global
- `pattern_ridl_gc_mark_auto_gen.md` - RIDL 自动生成 gc_mark

#### 5.2 从 AGENTS.md 提取

- `gotcha_quickjs_rom_mechanism.md` - ROM 机制与 RIDL 关系（用户说明）
- `pattern_ridl_module_detection.md` - 只对含 *.ridl 的 crate 聚合
- `architecture_mquickjs_base_vs_ridl.md` - base/ridl 两套输出的设计
- `reference_ridl_builder_prepare.md` - 准备命令与构建流程

#### 5.3 从 git log 近期 commit 提取

扫描最近 20 个 commit，识别：
- GC 相关改动 → `gotcha` / `architecture`
- RIDL 工具修复 → `gotcha` / `pattern`
- 测试添加 → `debug` / `pattern`

### Phase 6: 与 AGENTS.md 的关系

**保留 AGENTS.md**，作为"源头真理"（Source of Truth）：
- Core Constraints 保持权威
- Working Conventions 保持权威

**knowledge/ 的定位**：
- AGENTS.md：项目规则、约束、流程
- knowledge/：踩坑经验、设计上下文、调试方法

**划分原则**：
| 内容 | 去处 |
|------|------|
| "RIDL 模块必须 edition = 2024" | AGENTS.md（规则） |
| "忘记设 edition 导致宏展开失败" | knowledge/gotcha |
| "代码必须通过测试" | AGENTS.md（规则） |
| "GC root cycle 测试用 finalizer 验证回收" | knowledge/pattern |
| "不允许硬编码" | AGENTS.md（约束） |
| "某次硬编码导致多 app 冲突" | knowledge/gotcha |

AGENTS.md 在 frontmatter 引用 knowledge：
```markdown
---
knowledge_base: docs/knowledge/MEMORY.md
---
```

## 交付清单

### 必须交付

- [ ] `docs/knowledge/` 目录结构
- [ ] `docs/knowledge/README.md` - 使用说明
- [ ] `docs/knowledge/MEMORY.md` - 初始索引
- [ ] `.claude/docs/RULES-KNOWLEDGE-MEMORY.md` - 提取规则
- [ ] `.claude/docs/RULES-CODE-QUALITY.md` - Rust 质量规则
- [ ] `.claude/skills/post-commit-memory/skill.md`
- [ ] `.claude/skills/knowledge-dream/skill.md`
- [ ] `.claude/hooks/protect-core-files.sh`
- [ ] `.claude/hooks/check-unsafe-code.sh`
- [ ] `.claude/hooks/check-ridl-conventions.sh`
- [ ] `.claude/hooks/post-commit-review.sh`
- [ ] `.claude/settings.json` - hooks 配置
- [ ] 初始知识条目（从现有文档/commit 提取，≥5 条）

### 可选交付

- [ ] `.claude/hooks/check-gc-test-coverage.sh` - GC 测试覆盖检查
- [ ] `.claude/skills/ridl-module-checker/skill.md` - RIDL 模块规范检查器
- [ ] 更多 Rust 生态特定的质量规则（clippy 集成等）

## 实施顺序

1. **Phase 1（基础设施）** - 创建目录结构和核心文档
2. **Phase 5（初始化）** - 从现有文档提取首批知识条目
3. **Phase 2（Skills）** - 实现 post-commit-memory 和 knowledge-dream
4. **Phase 3（Hooks）** - 实现核心文件保护和安全检查
5. **Phase 4（质量规则）** - 完善 Rust 特定的质量检查规则
6. **Phase 6（整合）** - 更新 AGENTS.md，说明与 knowledge 的关系

## 验证

### 功能验证

1. **post-commit-memory**：做一个小改动 commit，验证能正确判定是否该写 knowledge
2. **knowledge-dream**：运行 `/knowledge-dream`，验证能正确整理 kairos 增量
3. **核心文件保护**：尝试修改 `context.rs`，验证能看到警告
4. **unsafe 检查**：写一个 unsafe 块无注释，验证被检测

### 质量验证

1. 从现有文档提取的知识条目数量 ≥ 5
2. 每个条目有完整的 frontmatter（name, description, type, created）
3. 每个条目有 **为什么** 和 **何时使用** 段落（reference 除外）
4. MEMORY.md 索引完整且分类正确
5. kairos 日志格式正确（`- HH:MM — <动作>：<一句话>`）

### 集成验证

1. 运行 `cargo test` - 所有测试通过
2. 运行 `cargo run -- tests` - JS 集成测试通过
3. git status 干净（除了新增的 .claude/ 和 docs/knowledge/）
4. hooks 不阻碍正常开发（仅警告，不拒绝）

## 风险与对策

### 风险 1：知识条目质量不一致

**对策**：
- 严格按 RULES-KNOWLEDGE-MEMORY.md 写入
- post-commit-memory skill 自带判定逻辑
- knowledge-dream 定期整理合并

### 风险 2：hooks 误报或漏报

**对策**：
- hooks 仅警告，不阻止（用户最终决定）
- 核心文件列表保守，只列真正关键的
- 定期根据实际使用调整规则

### 风险 3：与 Rust 生态工具冲突

**对策**：
- 质量检查与 clippy 互补，不重复
- unsafe 检查基于约定，不替代编译器
- 保留 `#[allow(clippy::...)]` 的优先级

### 风险 4：迁移工作量大

**对策**：
- 分阶段实施，Phase 1-2 优先
- 初始知识条目只提取明确的（≥5 条即可）
- hooks 逐步添加，先核心保护，后细节检查

## 后续演进

### 短期（1-2 周）

- 团队成员熟悉 `/post-commit-memory` 工作流
- 累积 10+ kairos 条目后首次运行 `/knowledge-dream`
- 根据实际使用调整 hooks 触发条件

### 中期（1-2 月）

- 知识库达到 20+ 条目
- 补充更多 Rust 生态特定的质量规则
- 可选交付项（GC 测试覆盖检查等）

### 长期（持续）

- 知识库成为团队的"集体记忆"
- 新人通过 knowledge/ 快速了解项目坑点
- AI 实例通过 knowledge/ 避免重复踩坑

## 状态

- 状态：待确认
- 预计工作量：4-6 小时（分阶段实施）
- 阻塞项：无
