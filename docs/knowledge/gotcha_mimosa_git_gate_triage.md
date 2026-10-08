---
name: gotcha-mimosa-git-gate-triage
description: Mimosa git-gate 高危拦截的分诊经验：本地 CLI argv 路径属接受风险，测试断言消息勿写成命令行样式
type: gotcha
created: 2026-10-08
sources: [2026-10-08 SDK 重定位提交]
---

**现象**：`git commit` 被 Mimosa L3 门禁拦截，报 high。两类典型：

1. **测试 JS 的 assert 消息写成命令行样式**（如
   `'expected require(test-module.single@1.0) to throw'`）被判"命令注入"。
   实为静态字面量反向断言，无任何 shell 参与。
2. **本地 CLI 构建工具的 argv 路径参数**（如 ridl-builder 的 `--out`/
   `--cargo-toml` → fs 操作）被判"path-traversal"。操作者即信任根，
   且工具已强制绝对路径，无信任边界可穿越。

**处置**：
- 类别 1 属正当可修：改写消息措辞避免命令行样式（如
  `'non-normalized module id must be rejected'`），语义不变、扫描器放行。
- 类别 2 属接受风险：**不要**为迎合扫描器添加假校验（违反仓库
  "禁止降级实现"规则）。经用户批准后对单条 commit 命令临时设
  `MIMOSA_GIT_GATE_FAILURE_MODE=open` 一次性放行，triage 结论写入
  commit message；门禁对后续操作恢复严格。

**Why**: SAST 对本地开发工具套用 web 威胁模型会产生结构性误报；
绕过安全控制必须显式、最小范围、经所有者批准并留痕。

**How to apply**: 遇 gate 拦截先逐条分诊（真漏洞 / 可正当修复的样式触发 /
接受风险），再选处置路径；不要默认绕过。
