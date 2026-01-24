# Repository Rules (Source of Truth)

## Core Constraints
- Treat a crate as a RIDL module **only if** the dependency path's `src/` directory contains at least one `*.ridl` file; otherwise exclude it from registry-driven RIDL aggregation.
- mquickjs constraint: QuickJS C API registration cannot happen at runtime; registration must be compile-time.
- No hardcoding / no degraded implementations:
  - Never implement aggregation/registration/initialization via hardcoded singleton names (e.g. "console") or module allowlists.
  - Never introduce temporary modifications, ad-hoc hacks, or other degraded implementations.
  - If experiments/validation/comparison tests are needed, **clone a separate copy in a parallel directory** and run experiments there; do not contaminate the current workspace.
- `build.rs` rule: do not hardcode relative paths for `rerun-if-changed` (e.g. `println!("cargo:rerun-if-changed=../../deps/ridl-tool")`); fixes must use a general mechanism.
- RIDL class id naming:
  - If globally registered: `module_name = GLOBAL`.
  - If `module` is declared: use normalized module path (replace non `[A-Za-z0-9_]` with `_`) as part of the class id.
  - Class ids must be ALL CAPS.
- Multi-app support:
  - Normalize app-id by replacing any non `[A-Za-z0-9_]` characters with `_` (including `-` -> `_`).
  - Select the app package by matching `cargo metadata` package `manifest_path` **exactly** to the provided `--cargo-toml`.
- Crate naming: the build orchestrator crate currently named `xtask` should be renamed to `ridl-builder`; the aggregated module selection snapshot must be `ridl-manifest.json` (not `ridl-plan.json`).
- Engine constraint (mquickjs): `JSValue` heap objects/strings are managed by tracing GC (not ref-counting). Public API has no `JS_FreeValue` / `JS_DupValue`. Generated/FFI code typically must not explicitly free `JSValue`.
- Convention: all RIDL modules (including RIDL test module crates under `tests/`) must set `edition = "2024"` in `Cargo.toml`.

## Working Conventions
- For any requirement, think deeply first and produce a concrete plan. Store plans under `docs/planning/` (one plan per requirement) and mark the plan as completed when done.
- Do not start implementation until the plan has been discussed and the user explicitly confirms the requirement is ready.
- After the plan is approved/confirmed, proceed to execute it by default. If you hit problems, think through solutions and iterate. If more than 5 attempts still can’t meaningfully unblock progress, stop and ask the user how they want to proceed.
- When blocked: reason first, avoid guessing. After several failed attempts, summarize the blocker clearly for user review and decide the next step together.
- Every change requires tests. Write/review tests early and ask the user to review tests explicitly.
- All tests must pass; if tests fail, report the reason first and wait for the user’s decision on how to proceed.
- After finishing a feature, run JS integration cases under `tests/` (in addition to `cargo test`). Command: `cargo run -- tests`.
- After completing a feature, update related docs to keep docs and code consistent. If you detect inconsistency, report it first and wait for user confirmation before making corrective doc changes.
- Documentation requirement: in-repo documents (especially design/planning docs under `docs/planning/`) must be written in Chinese.
- For each large module, maintain a `README.md` describing purpose; add design/implementation docs when complexity warrants.
- Shell commands are allowed in this repo session (any CLI commands). Still follow safety rules: for commands that modify the filesystem/codebase/system state, briefly explain purpose and impact first.
- Git commit message format: subject/title line < 50 columns; body lines < 88 columns.
- After finishing a plan’s implementation, do not commit by default; ask the user whether they want a commit.
- AGENTS.md is the single source of truth for working rules in this repo.

## Mi Code Added Memories
- 用户说明：当前 RIDL 会被编译进 ROMClass 的 props 与 proto_props；当初实现时并未充分理解 ROM 机制，因此需要重新审视 RIDL 扩展机制与 ROM/标准库的关系与编译阶段考量。
