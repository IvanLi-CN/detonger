# Decision Memo: Repo layout + tooling for detonger (Rust reinit)

## Context

- What are we building?
  - A Rust workspace with a BLE printer library + a CLI tool.
- Where does it run?
  - First-class: macOS（本计划仅做 macOS）。
- Constraints (time, ops, perf, security, cost)?
  - 需要快速迭代与可回归；硬件依赖导致 CI 不能强依赖真实设备；未来可能加入 Web 程序，目录结构需提前避免冲突。

## Evidence (from references)

- Similar projects:
  - `skills/style-playbook/references/projects/televy-backup.md`（Rust workspace：清晰 crate 边界）
  - `skills/style-playbook/references/projects/isolappurr-usb-hub.md`（device mono-repo：`web/`/`tools/` 共存）
  - `skills/style-playbook/references/projects/display-ambient-light-desktop.md`（Rust + Web 混合结构：边界清晰）
- Relevant tags:
  - `skills/style-playbook/references/tags/rust-workspace.md`
  - `skills/style-playbook/references/tags/device-mono-repo.md`
  - `skills/style-playbook/references/tags/developer-utility.md`

## Recommendation (default)

- Choice:
  - Cargo workspace at repo root
  - Rust crates under `crates/`
  - Reserve a top-level directory for future non-Rust apps: `web/`
- Why this matches the established style:
  - 参考 Rust workspace 的习惯：尽早把 library/CLI 边界拆清（`televy-backup`）。
  - device mono-repo 允许多域共存，但要把边界与入口写在 README/Justfile 中（`isolappurr-usb-hub`）。
- Risks:
  - 未来 Web 目录命名如果后续反复变化，会导致迁移与引用漂移。
- Mitigations:
  - `web/` 仅预留不实现；在 README 记录该约定，避免后续目录漂移。

## Alternatives (1–2)

### Option A: Single crate (no workspace)

- Pros:
  - 结构最简单，初始化成本低。
- Cons:
  - library/CLI 边界不清晰；未来加入 Web 或其他工具时更容易纠缠。
- When to choose:
  - 明确只做一个一次性 CLI、且不会长期演进。

### Option B: Workspace + apps/ directory (apps/web reserved)

- Pros:
  - 对未来多语言/多应用扩展更直观（`apps/*`）。
- Cons:
  - 与既有 device mono-repo 中常见的 `web/`/`desktop/` 顶层目录习惯不完全一致，需要在 README 强约定。
- When to choose:
  - 未来会有多个非 Rust app（不仅 Web），且希望统一归档。

## Follow-ups

- Open questions:
  - None（本计划已冻结为 macOS-only，且 `web/` 作为预留目录）
- Next actions:
  - 在 PLAN 的 blockers 决策后，将计划状态从 `待设计` 冻结到 `待实现`，再进入实现阶段。
