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

The [work-tracking contract](docs/work-tracking.md) defines the operational split between roadmap, conformance matrices, GitHub issues, pull requests, and current-status documents. Determine the active milestone from the accepted roadmap/status records, then use that milestone's GitHub umbrella/pickup issue. M4's public issue #3 is completed historical coordination after final M4 closure; the M5 successor pickup surface is [public issue #45](https://github.com/dornglut/runen-ui/issues/45).

Use this pickup sequence:

1. Read the accepted roadmap/status records and the active milestone umbrella issue.
2. Open the active slice issue or readiness/audit task named by that umbrella.
3. Verify exact accepted `main`, branch, pull request, and head.
4. Read linked ADRs, any accepted charter/matrix rows, and stable architecture contracts.
5. Inspect current source, tests, and unresolved review findings.
6. Execute only the active slice or explicitly linked prerequisite issue.
7. Update the issue after every reviewed green checkpoint, material review correction, readiness transition, and merge.
8. Never begin the next slice or milestone from an unmerged feature or authority branch.

M5A semantic contribution and independent identity is complete. Its feature was
owner-accepted at exact head `8377ced53c08d7b5be3020368ceddd3ee81294a5`,
passed exact-head CI #889, and was guarded-squash-merged in PR #53 as
`e3c304600ec1777cd17a1973946a43c765df1c31`. Its mandatory post-merge
reconciliation was explicitly owner-accepted at exact head
`66c2e2a5e2adf3709f93e8d45821a5844986dc0c`, guarded-squash-merged in PR #54
as `d7189d9d145b20edc6ad931ead1589f6277373d2`, proved exact reviewed/squash
tree identity, and passed accepted-main CI #898 at that squash. Issue #47 is
closed.

Issue #55 is the sole active M5 execution gate. It freezes semantic publication,
focus, support/availability, surface-scoped action targeting, publication
atomicity/failure semantics, and the renderer/semantic cutover before M5B. It
also owns one bounded pre-1.0 vocabulary correction: remove route-bound semantic
LogicalScroll while preserving accepted M4 routed scrolling. #55 is not M5B or
M5C implementation. M5B #48 remains blocked until #55 is exact-head validated,
critically reviewed, explicitly owner-accepted, merged/reconciled as required,
and accepted-main verified.

A future thread should need only the repository, umbrella issue, and active slice/audit issue to locate current work.

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

Format intentional Rust changes with `cargo +stable fmt --all`. Run `cargo validate` before every commit and again before handoff. It is the locked, read-only shared local/CI baseline and includes stable formatting checks, locked tests, Clippy with denied warnings, MSRV tests, repository metadata checks, and workspace-root Markdown relative-link checks. Also run slice-specific checks, the applicable conformance-matrix uniqueness/status/schema/count audit, public API and removed-symbol audits, unsafe-code audit, cross-document truth audit, exact base/head/remote verification, clean-worktree verification, and `git diff --check`.

Do not reuse validation or exact-head CI claims from an earlier head. Critically review the complete diff for stale references, broken links, false support claims, accidental scope, duplicate authority, premature later-slice APIs, and missing migrations. Commit coherent slices separately. Open draft PRs unless the owner explicitly requests ready-for-review, and never merge them without explicit owner authorization.
