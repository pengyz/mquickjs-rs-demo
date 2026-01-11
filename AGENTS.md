## Mi Code Added Memories
- In this project, treat a crate as a RIDL module only if the dependency path's src/ directory contains at least one *.ridl file; otherwise exclude it from registry-driven RIDL aggregation.
- In this project (mquickjs), C API registration cannot happen at runtime; registration must be done at compile time. This constraint is the root reason for the symbol keep-alive + build-time aggregation design; avoid suggesting runtime QuickJS C API registration.

## Working Conventions
- For any requirement, think deeply first and produce a concrete plan. Store plans under `doc/planning/` (one plan per requirement) and mark the plan as completed when done.
- Do not start implementation until the plan has been discussed and the user explicitly confirms the requirement is ready.
- When blocked: reason first, avoid guessing. After several failed attempts, summarize the blocker clearly for user review and decide the next step together.
- Every change requires tests. Write/review tests early and ask the user to review tests explicitly. All tests must pass; if tests fail, report the reason first and wait for the user's decision on how to proceed.
- After completing a feature, update related docs to keep docs and code consistent. If you detect inconsistency, report it first and wait for user confirmation before making corrective doc changes.
- For each large module, maintain a `README.md` describing purpose; add design/implementation docs when complexity warrants.
- AGENTS.md is the single source of truth for working rules in this repo.
