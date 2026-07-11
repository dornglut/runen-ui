# RunenUI Agent Contract

> **Category: Current contract**

## Authority

Use this order when repository sources disagree:

1. explicit task/owner decisions;
2. accepted execution charter for the active program;
3. `docs/status-map.md`, `docs/feature-support-matrix.md`, `docs/roadmap.md`, and `docs/architecture.md`;
4. accepted ADRs and focused architecture documents;
5. current implementation and behavioral tests;
6. guides and historical records.

Historical tags and removed legacy material are never active implementation authority.

## Preflight and scope

- Start from current `master`; fetch/pull with fast-forward only and confirm branch, head, remote, and a clean worktree.
- Read every affected implementation, test, and authority document before editing. Verify documentation claims against code.
- Work in one coherent reviewed slice. Do not mix unrelated cleanup, architecture, and implementation.
- Preserve unrelated user changes. Never force-push, merge a PR, or use destructive Git commands without explicit authorization.
- CI validates source; it must never construct, rewrite, commit, or push implementation changes.

## Architecture rules

- Keep application state/action/update, host, runtime, renderer, and product ownership separate.
- Do not add broad controls before M2–M5, renderer backends before M6, interaction-state styling before mounted state, or editable text before M4/M5/M8 contracts.
- Do not add crates without a documented real ownership/dependency/consumer boundary.
- Do not import historical legacy crates or create a parallel path when replacing an old one.
- The project is pre-1.0; do not preserve prototype APIs through compatibility layers unless explicitly approved.
- Prefer behavioral/conformance tests. Do not add source-grep tests for forbidden symbols.
- Keep active status, feature support, roadmap, architecture/ADR, and retention records aligned with implementation changes.

## Validation and delivery

Run `cargo validate` before every commit and again before handoff. It is the shared local/CI baseline and includes stable formatting, locked tests, Clippy with denied warnings, MSRV tests, and Markdown relative-link checks. Also run slice-specific checks and `git diff --check`.

Critically review the complete diff for stale references, broken links, false support claims, accidental scope, and missing migrations. Commit coherent slices separately. Open draft PRs unless the owner explicitly requests ready-for-review, and never merge them yourself.
