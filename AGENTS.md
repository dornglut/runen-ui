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

The M5 readiness gate #55 is also accepted. Its exact reviewed head
`15c90424a0fbae4312b0cb0c5fb76932b3ce1ee1` passed exact-head CI #902 and was
guarded-squash-merged in PR #56 as
`d2f8fabd33860ec1510f82d5792b5bd8f2db8f43`. Reviewed head and squash share
exact tree `3be7ed95d5879c5d4dc9639583c5ef8490522267`, and accepted-main push CI
#903 passed at that squash. The accepted readiness authority freezes semantic
focus, support/availability, surface-scoped action targeting, publication
atomicity/failure semantics, and the renderer/semantic cutover, and removes
route-bound semantic LogicalScroll authoring while preserving accepted M4 routed
scrolling.

M5B #48 semantic tree publication and incremental updates is fully accepted and
reconciled. Exact reviewed feature head
`3b9db8b37098786cc0d53d38ae5d597c3460c38b` passed exact-head CI #1082 and was
guarded-squash-merged in PR #58 as
`43d23aefb81757a516ae569b3e86b9e0f2c71e23`; reviewed head and squash share
exact tree `1708d2536c6f1d202ac58dd7cb5f3cc97a438517`. The connector-origin merge did
not emit the normal push workflow event, so the exact squash was independently
revalidated through the unchanged read-only PR CI path in temporary PR #60; CI
#1084 attempt 2 passed and PR #60 was closed unmerged. The mandatory M5B
reconciliation was explicitly owner-accepted at exact reviewed head
`c154e91b5ba693a27eb61a4745d4184193088d5b`, passed exact-head CI #1089, and
was guarded-squash-merged in PR #61 as
`afb7f8f363a8df3eb51be1a9bc5f0f180f84190b`; accepted-main CI #1090 passed.

M5C #49 semantic action ingress and accessibility resolution is now
owner-accepted and feature-merged. Exact reviewed head
`504899b79059eb94ad4474d67bba1e27eb30b374` passed exact-head CI #1170 /
`31889342640` and was guarded-squash-merged in PR #62 as
`846c4e6adfdcd9236586f1b9978f63e71ff4fb86`. Reviewed head and squash share
exact tree `dfa7cb71166a3f333b560508a7e82fbeb45df000`, and accepted-main push CI
#1171 / `31903354382` passed at that exact squash.

The mandatory post-M5C authority/current-contract reconciliation is the sole
current M5 gate. Its branch is `codex/m5c-authority-reconciliation`, created from
exact accepted feature squash `846c4e6adfdcd9236586f1b9978f63e71ff4fb86`.
M5C #49 remains open for this reconciliation only, and M5D #50 remains blocked
until the reconciliation is exact-head validated, critically reviewed,
explicitly owner-accepted, guarded-merged, and accepted-main verified. Do not
begin M5D early. Do not expose a public semantic-to-`MountedNodeId` shortcut,
recreate semantic LogicalScroll, add a parallel semantic queue/default/trace
authority, or weaken exact semantic queue-front/post-callback revalidation.

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
