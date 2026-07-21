# RunenUI Agent Contract

> **Category: Current contract**

## Authority

Use this order when repository sources disagree:

1. accepted ADR behavior;
2. the accepted execution charter for the active program;
3. the conformance matrix and its permanent observable acceptance rows;
4. stable public architecture contracts;
5. current implementation and behavioral tests;
6. roadmap, status map, and feature/support records;
7. pull-request descriptions, historical reports, and other historical records.

Explicit repository-owner decisions may accept or reject work, but they do not silently rewrite an accepted behavioral contract. Record any contract change in the owning ADR, charter, matrix, or architecture document.

Historical tags and removed legacy material are never active implementation authority.

## Work tracking and pickup

The [work-tracking contract](docs/work-tracking.md) defines the operational split between roadmap, conformance matrix, GitHub issues, pull requests, and current-status documents. The M4 operational milestone is [public issue #3](https://github.com/Crystonix/runen-ui/issues/3).

Use this pickup sequence:

1. Read the umbrella issue.
2. Open the active slice issue.
3. Verify exact accepted `main`, branch, pull request, and head.
4. Read linked ADRs, the accepted charter, matrix rows, and stable architecture contracts.
5. Inspect current source, tests, and unresolved review findings.
6. Execute only the active slice or explicitly linked prerequisite issue.
7. Update the issue after every accepted head, review correction, and merge.
8. Never begin the next slice from an unmerged feature branch.

A future thread should need only the repository, umbrella issue, and active slice issue to locate current work.

## Preflight and scope

- Start from current `main`; fetch/pull with fast-forward only and confirm branch, accepted base, merge base, head, remote head, and a clean worktree.
- Read every affected implementation, test, authority document, umbrella issue, and active slice issue before editing. Verify documentation claims against code and accepted merge state.
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
- Keep active status, feature support, roadmap, architecture/ADR, matrix, issues, and retention records aligned with accepted implementation changes.

## Validation and delivery

Format intentional Rust changes with `cargo +stable fmt --all`. Run `cargo validate` before every commit and again before handoff. It is the locked, read-only shared local/CI baseline and includes stable formatting checks, locked tests, Clippy with denied warnings, MSRV tests, repository metadata checks, and workspace-root Markdown relative-link checks. Also run slice-specific checks, the matrix uniqueness/status/schema/count audit, public API and removed-symbol audits, unsafe-code audit, cross-document truth audit, exact base/head/remote verification, clean-worktree verification, and `git diff --check`.

Do not reuse validation or exact-head CI claims from an earlier head. Critically review the complete diff for stale references, broken links, false support claims, accidental scope, duplicate authority, premature later-slice APIs, and missing migrations. Commit coherent slices separately. Open draft PRs unless the owner explicitly requests ready-for-review, and never merge them yourself.
