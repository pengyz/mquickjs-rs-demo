## Mi Code Added Memories
- In this project, treat a crate as a RIDL module only if the dependency path's src/ directory contains at least one *.ridl file; otherwise exclude it from registry-driven RIDL aggregation.
- In this project (mquickjs), C API registration cannot happen at runtime; registration must be done at compile time. This constraint is the root reason for the symbol keep-alive + build-time aggregation design; avoid suggesting runtime QuickJS C API registration.
- User will close VSCode and work directly in terminal for this repo/session (to reduce concurrent cargo/rust-analyzer build conflicts like ETXTBSY).
- 禁止做“简化硬编码”：任何聚合/注册/初始化逻辑不得通过硬编码 singleton 名称（例如 "console"）或模块白名单来实现，必须走标准、可扩展的通用机制（新增模块仅放入 ridl-modules/ 即可生效）。

## Working Conventions
- For any requirement, think deeply first and produce a concrete plan. Store plans under `doc/planning/` (one plan per requirement) and mark the plan as completed when done.
- After the plan is approved/confirmed, proceed to execute it by default. If you hit problems, think through solutions and iterate. If more than 5 attempts still can’t meaningfully unblock progress, stop and ask the user how they want to proceed.
- Do not start implementation until the plan has been discussed and the user explicitly confirms the requirement is ready.
- When blocked: reason first, avoid guessing. After several failed attempts, summarize the blocker clearly for user review and decide the next step together.
- Every change requires tests. Write/review tests early and ask the user to review tests explicitly. All tests must pass; if tests fail, report the reason first and wait for the user's decision on how to proceed.
- After finishing a feature, run JS integration cases under `tests/` (in addition to `cargo test`). Command: `cargo run -- tests`.
- After completing a feature, update related docs to keep docs and code consistent. If you detect inconsistency, report it first and wait for user confirmation before making corrective doc changes.
- 文档要求：项目内文档（尤其是 doc/planning/ 下的设计/规划文档）使用中文编写。
- For each large module, maintain a `README.md` describing purpose; add design/implementation docs when complexity warrants.
- Default permission: shell commands are allowed (not limited to git/rmdir; any CLI tool is allowed).
- Git commit message format: subject/title line < 50 columns; body lines < 88 columns.
- After finishing a plan’s implementation, do not commit by default; ask the user whether they want a commit.
- AGENTS.md is the single source of truth for working rules in this repo.
