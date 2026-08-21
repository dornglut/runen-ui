# Work Tracking

> **Category: Current contract**

RunenUI separates durable architecture authority from volatile execution state.
This document defines where each kind of truth lives and the required pickup
workflow for contributors and automated agents.

## Authority boundaries

### Roadmap

[`roadmap.md`](roadmap.md) owns milestone order, dependencies, durable included
scope, explicit non-goals, exit criteria, and the long-term production sequence.
It must not become a branch or pull-request log.

### Conformance matrices

[`architecture/m4-conformance-matrix.md`](architecture/m4-conformance-matrix.md)
owns permanent M4 behavioral acceptance IDs, observable requirements, positive
proof ownership, negative proof ownership, trace proof ownership, delivery slice,
status, and gate classification. After M5C acceptance all 237 M4 rows, including
the inherited `ACCESS-01` and `ACCESS-02` M5 gates, are owner-accepted.

[`architecture/m5-conformance-matrix.md`](architecture/m5-conformance-matrix.md)
owns the separate M5-specific behavioral acceptance inventory. It deliberately
does not duplicate inherited M4 `ACCESS-01` and `ACCESS-02`; M5C completes those
rows in the M4 matrix while M5-specific semantic identity, product, action,
testing, migration, and closure observations remain in the M5 matrix. Final M5
truth is 53/53 owner-accepted.

[`architecture/m6-conformance-matrix.md`](architecture/m6-conformance-matrix.md)
owns the separate M6 renderer-neutral paint/hit observable inventory accepted by
M6A0. It contains 36 permanent rows across M6A–M6D. Architecture acceptance does
not promote behavior: all 36 rows remain `blocked` until their implementation and
proof slices satisfy the normal lifecycle.

Configured M4+M5+M6 truth is therefore `326 total / 290 owner-accepted / 0
implementation-complete / 0 proof-complete / 36 blocked`.

The canonical repository audit validates every configured matrix, each matrix's
own declared status/schema/count truth, delivery/gate policy, and permanent ID
uniqueness across the configured matrix set.

Matrix rows are not GitHub issues. One slice issue may own many matrix rows.

### GitHub umbrella and execution issues

The closed [M4 umbrella issue](https://github.com/dornglut/runen-ui/issues/3)
and its linked execution issues preserve M4's volatile delivery history. The
[M5 umbrella issue #45](https://github.com/dornglut/runen-ui/issues/45) and
M5E [#51](https://github.com/dornglut/runen-ui/issues/51) preserve M5's completed
coordination history after final reconciliation.

M6 uses [umbrella issue #71](https://github.com/dornglut/runen-ui/issues/71).
[M6A0 issue #72](https://github.com/dornglut/runen-ui/issues/72) established the
renderer-neutral scene/publication architecture and conformance gate through
accepted ADR 0007 and the 36-row M6 matrix. The required bounded post-A0
current-contract reconciliation completed in PR #75. All 36 M6 behavior rows
remain `blocked`. [#59](https://github.com/dornglut/runen-ui/issues/59) is the
first M6A implementation pickup and owns only `SCENE-PUB-01..05`; later scene,
renderer, and backend behavior remains outside that slice.

Each active milestone issue owns volatile state such as:

- exact accepted base SHA;
- active branch and draft pull request;
- current reviewed checkpoint head;
- dependencies and blockers;
- conformance/proof checklists for the active slice;
- latest validation and exact-head CI or an explicit infrastructure-only waiver;
- acceptance state;
- next action and next unblocked issue.

The live branch head is read from pull-request metadata. Do not manually mirror
every transient commit into issue or pull-request prose. Record exact heads at
reviewed green checkpoints, readiness transitions, review corrections that
change the accepted diff, and merge.

Architecture, governance, and tooling issues own real work outside a slice's
accepted behavior. Each issue must state whether it blocks the next
implementation slice.

### Pull requests

A pull request owns its exact implementation or governance diff, public API
accounting, structure changes, tests, validation results, CI run, review
findings, deferred scope, and any explicit infrastructure-only CI waiver. It
must not redefine accepted behavior that belongs to an ADR, charter, or matrix
row.

The pull-request body should retain stable scope, accepted base, authorities,
and merge gates. Use one updated checkpoint record for the reviewed head,
validation run, findings, and remaining blockers rather than a stale manually
maintained live-head field.

### Status map and support matrix

[`status-map.md`](status-map.md) and
[`feature-support-matrix.md`](feature-support-matrix.md) own current accepted
repository truth. They report what exists after accepted merges and must not
describe unmerged branch work as current support.

### Historical archive

[`history/public-repository-migration.md`](history/public-repository-migration.md)
owns the mapping from the former private repository to this public repository.
Private issue and pull-request numbers are historical only and must be qualified
as archive references. They are never active execution authority.

## Operational milestones

M4's public operational milestone was
[issue #3](https://github.com/dornglut/runen-ui/issues/3). Its execution graph is
complete through M4D3 and the final M4 authority reconciliation. It is historical
coordination.

M5 used [issue #45](https://github.com/dornglut/runen-ui/issues/45) as its
coordination and pickup authority. M5A0 #46, M5A #47, readiness amendment #55,
M5B #48, M5C #49, M5D #50, and M5E #51 are complete through the final M5
authority reconciliation.

M6 is now the active roadmap milestone at the M6A implementation boundary.
M6A0 #72 is accepted through PR #73 and its bounded current-contract
reconciliation completed in PR #75. #59 is the first M6A implementation slice;
it must preserve all inherited publication/input/semantic contracts and is
limited to `SCENE-PUB-01..05`.

Accepted M5A evidence remains:

```text
reviewed feature head:      8377ced53c08d7b5be3020368ceddd3ee81294a5
feature exact-head CI:      #889
feature squash:             e3c304600ec1777cd17a1973946a43c765df1c31
reconciliation head:        66c2e2a5e2adf3709f93e8d45821a5844986dc0c
reconciliation squash/main: d7189d9d145b20edc6ad931ead1589f6277373d2
accepted-main CI:           #898
```

Accepted #55 readiness evidence remains:

```text
reviewed feature head:      15c90424a0fbae4312b0cb0c5fb76932b3ce1ee1
feature exact-head CI:      #902
feature squash:             d2f8fabd33860ec1510f82d5792b5bd8f2db8f43
reconciliation head:        48328df608a12425c3f03dd06cddecaabc50069f
reconciliation squash/main: 807bd7feb1e796eccd49c0ff024da0f79d1868b1
reconciliation tree:        22efd2561be6fb25e9e5f411d0ac1fa53d3595ee
accepted-main CI:           #905
```

Accepted #55 freezes the successor authority for semantic PRIMARY focus,
support versus current availability, virtual semantic-node target preservation,
private semantic-to-mounted resolution, relationship default targeting,
hidden/invalid-owner composition, surface-scoped semantic requests and revisions,
exact stale/post-callback revalidation, staged atomic publication and failure
taxonomy, and clean renderer/semantic cutover. It removes route-bound
LogicalScroll from `SemanticAction` while preserving accepted M4 routed scrolling.

### M5B through M5E closed

M5B #48 was implemented in PR #58 from exact accepted base
`807bd7feb1e796eccd49c0ff024da0f79d1868b1`. Critical review found and corrected
two material issues before acceptance: routed focus changes now dirty the
semantic product without invalidating cached contribution, and the semantic
compositor now uses deterministic publication-local lookup indexes instead of
repeated fallback scans.

The repository owner explicitly accepted exact final reviewed head
`3b9db8b37098786cc0d53d38ae5d597c3460c38b`. Exact-head CI #1082 /
`31847771313` passed. PR #58 was guarded-squash-merged pinned to the accepted head as
`43d23aefb81757a516ae569b3e86b9e0f2c71e23`. Reviewed head and squash share
exact tree `1708d2536c6f1d202ac58dd7cb5f3cc97a438517`, proving exact repository
content identity.

The connector-origin feature merge did not emit the normal `push` workflow
event. That event-delivery fact was preserved honestly. The exact accepted
feature squash was independently revalidated without source mutation through
temporary PR #60; CI #1084 / `31850376490`, attempt 2, checked out exact SHA
`43d23aefb81757a516ae569b3e86b9e0f2c71e23` and passed canonical stable/MSRV
repository validation. PR #60 was closed unmerged after evidence capture.

The mandatory M5B authority/current-contract reconciliation was then explicitly
owner-accepted at exact reviewed head
`c154e91b5ba693a27eb61a4745d4184193088d5b`. Exact-head CI #1089 /
`31851743216` passed. PR #61 was guarded-squash-merged as
`afb7f8f363a8df3eb51be1a9bc5f0f180f84190b`; reviewed head and squash share
exact tree `e6797fb439d8b181d1532c57090915f2589e57de`. This merge emitted the
normal default-branch `push` event, and accepted-main CI #1090 /
`31872934604` passed at exact squash/main. M5B is therefore fully accepted,
reconciled, accepted-main verified, and #48 is closed.

M5C #49 was activated from exact accepted M5B reconciliation base
`afb7f8f363a8df3eb51be1a9bc5f0f180f84190b` and implemented in PR #62. Its
complete implementation/proof package passed exact-head CI #1166 /
`31882567707` at proof-evidence head
`7565a7a3744c50a93cb542549b8c82e6ae548084`. Final reviewed head
`504899b79059eb94ad4474d67bba1e27eb30b374` then passed exact-head CI #1170 /
`31889342640` and final critical review. The repository owner explicitly accepted
that exact head.

PR #62 was guarded-squash-merged pinned to the accepted head as
`846c4e6adfdcd9236586f1b9978f63e71ff4fb86`. Reviewed head and squash share
exact tree `dfa7cb71166a3f333b560508a7e82fbeb45df000`, proving exact repository
content identity. `main` pointed to that exact squash, and accepted-main push CI
#1171 / `31903354382` passed on the exact squash.

The mandatory M5C authority/current-contract reconciliation was then explicitly
owner-accepted at exact reviewed head
`fbd0bdf44bddd660e06b4642a56f7a1d64bccab2`. Exact-head CI #1179 /
`31914448654` passed. PR #63 was guarded-squash-merged as
`b2064f24e778bd69e2876ec09a7431d612682304`; reviewed head and squash share
exact tree `82625aedbdc03a5754949cffee51025e99cd6949`. Accepted-main push CI
#1180 / `31938332306` passed at that exact squash. M5C is therefore fully
accepted, reconciled, accepted-main verified, and #49 is closed.

M5D #50 was activated from exact accepted M5C reconciliation base
`b2064f24e778bd69e2876ec09a7431d612682304` and implemented in PR #64. After
iterative exact-head formatting/compile/Clippy corrections and a final cold
review, exact reviewed head `471d2acf402a0f7d3f89a1de2a1b908fe23ff619`
passed canonical exact-head CI #1230 / `31962536977`. The final COMMENT review
found no feature blocker, and the repository owner explicitly accepted that
exact head.

PR #64 was guarded-squash-merged pinned to that accepted head as
`72d2405211a3fd6d11e0d17680b7769df90b5ffe`. Reviewed head and squash share
exact complete repository tree `bdbf19f5c2197490d6b922fb792791b205f40370`.
Accepted-main push CI #1231 / `31967898198` passed on the exact squash.

The M5D feature adds genuine downstream public crate `runenui_testing` with
ordinary public core/runtime dependencies only, deterministic `TestHarness<App>`,
public `ManualClock` use, configurable nonzero fixed-surface publication,
explicit bounded pumping and finite settling, exact snapshot-scoped semantic
queries/targets, ordinary public interaction ingress, and read-only state/focus/
reconciliation/frame/layout/hit/paint/semantic/trace/replay inspection. It does
not enable `internal-test-seams`, use hidden mutation bridges, recover semantic
owners as `MountedNodeId`, guess surface scope from bare semantic IDs, create an
unbounded settle/wall-clock wait, add a parallel runtime model, or introduce a
semantic LogicalScroll compatibility path.

M5D's mandatory authority/current-contract reconciliation was then explicitly
owner-accepted at exact reviewed head
`522b2770a2e6763e54e9eb6237fefc83e88d8cf9`. Exact-head CI #1242 /
`31969642341` passed. PR #65 was guarded-squash-merged as
`3c50f2fe0732871a3e2fdf7dba45983a23b813a1`; reviewed head and squash share
exact tree `7e72b2738d539042ed28a032b305fc27cb45042a`. Accepted-main CI #1244 /
`32108782685` passed at that exact squash. M5D is therefore fully accepted,
reconciled, accepted-main verified, and #50 is closed.

M5E #51 was implemented/proved in PR #67 from exact accepted M5D
reconciliation base `3c50f2fe0732871a3e2fdf7dba45983a23b813a1`. Its final corrected
reviewed feature head `7f3e0c9e881ff384516459db66436e662c5fb790` passed canonical
exact-head CI #1294 / `32130312467`, received explicit repository-owner merge
authorization, and was guarded-squash-merged as
`b07ae423d6a3573a4dd8a96a7ce5d6b5b1f0be1e`. Reviewed feature head and squash
share exact complete repository tree `c5dc7fa000496d76c35e98f3a481fc1de5762f4c`.
Accepted-main CI #1296 / `32135074552` independently validated that exact squash
through unchanged read-only PR #68, which was closed unmerged.

M5E proves the accepted M5A–M5D machinery as one public architecture: genuine
downstream semantic contribution/action/state/trace/replay integration, Counter
activation convergence across semantic/pointer/keyboard/automation/programmatic
origins, source-grounded adapter-neutral accessibility mapping, and a clean
cutover from retired semantic/testing authority. It adds no native accessibility
bridge, renderer scene, M6 implementation, or compatibility API.

Final M5 truth after the bounded authority reconciliation is:

```text
M5:       53 total / 53 owner-accepted / 0 implementation-complete / 0 proof-complete / 0 blocked
M4:       237 total / 237 owner-accepted / 0 proof-complete / 0 blocked
```

Final authority reconciliation PR #69 is the closure record that promotes only
`M5-INTEG-01..05`, aligns retained current-contract authority, and establishes
the final accepted-M5 base for successor pickup. It is accepted and
accepted-main validated.

### M6A0 architecture/conformance gate accepted

M6A0 #72 was executed in PR #73 from exact final-M5 reconciliation base
`8e09a61832e2077db0e1366472b628c9b2478880`. After repeated source-first cold
review, exact reviewed head `c0169ebea044a0009a334f3d5ecc13ff8d495885`
passed canonical exact-head CI #1349 / `32181344340` and received explicit
repository-owner merge authorization. PR #73 was guarded-squash-merged as
`966778dd31e0f6b6df76ee4f6283a984fc724b36`; reviewed head and squash share
exact complete tree `fe057a3fef9ea6de053ce86ce336212f0aa3a413`.

The connector-origin merge exposed no exact-squash push run through the available
workflow lookup, so exact accepted squash `966778dd...` was independently
validated through unchanged read-only PR #74. CI #1351 / `32186597198` checked
out that exact squash and passed canonical validation; PR #74 was then closed
unmerged.

The required bounded current-contract reconciliation then completed in PR #75.
It changed current status/discoverability/evidence only and did not promote any
M6 behavior row or implement #59. ADR 0007 and the M6 matrix remain accepted
target authority. M6 behavior remains:

```text
M6:        36 total / 0 owner-accepted / 0 implementation-complete / 0 proof-complete / 36 blocked
configured: 326 total / 290 owner-accepted / 0 implementation-complete / 0 proof-complete / 36 blocked
```

The completed public M4 execution graph is:

- [#2 — public RunenUI authority cutover](https://github.com/dornglut/runen-ui/issues/2);
- [#11 — deterministic repository structure and authority audit](https://github.com/dornglut/runen-ui/issues/11), after #2 and its readiness freeze;
- [#4 — M4C3 pointer lifecycle](https://github.com/dornglut/runen-ui/issues/4), after #11 and its readiness freeze;
- [#5 — M4C4 focus scopes and modality](https://github.com/dornglut/runen-ui/issues/5), after #4;
- [#6 — M4C5 keyboard, text, IME, automation, and M4C closure](https://github.com/dornglut/runen-ui/issues/6), after #5;
- [#7 — M4D1 complete trace schema and causality](https://github.com/dornglut/runen-ui/issues/7), after #6;
- [#8 — M4D2 export, redaction, and bounded sink](https://github.com/dornglut/runen-ui/issues/8), after #7;
- [#9 — M4D3 replay and M4 closure](https://github.com/dornglut/runen-ui/issues/9), after #8.

The accepted M5 implementation sequence is complete:

- [#46 — M5A0 semantic/testing architecture and conformance authority](https://github.com/dornglut/runen-ui/issues/46), accepted and closed;
- [#47 — M5A semantic contribution and independent identity](https://github.com/dornglut/runen-ui/issues/47), accepted, reconciled, and closed;
- [#55 — M5 readiness semantic publication/focus/virtual-action authority amendment](https://github.com/dornglut/runen-ui/issues/55), accepted, reconciled through PR #57, and closed;
- [#48 — M5B semantic tree publication and incremental updates](https://github.com/dornglut/runen-ui/issues/48), accepted, reconciled through PR #61, accepted-main verified, and closed;
- [#49 — M5C semantic action ingress and accessibility resolution](https://github.com/dornglut/runen-ui/issues/49), accepted, reconciled through PR #63, accepted-main verified, and closed;
- [#50 — M5D public deterministic headless testing harness](https://github.com/dornglut/runen-ui/issues/50), accepted, reconciled through PR #65, accepted-main verified, and closed;
- [#51 — M5E integrated conformance, migration, and M5 closure](https://github.com/dornglut/runen-ui/issues/51), accepted feature/proof through PR #67 and completed by final authority reconciliation PR #69.

The M6 execution graph begins:

- [#71 — M6 renderer-neutral paint/hit scene protocol](https://github.com/dornglut/runen-ui/issues/71), active umbrella;
- [#72 — M6A0 architecture/conformance authority](https://github.com/dornglut/runen-ui/issues/72), accepted through PR #73 and reconciled through PR #75;
- [#59 — retained-publication persistence/cost](https://github.com/dornglut/runen-ui/issues/59), first M6A implementation slice, owning `SCENE-PUB-01..05` only.

Architecture and tooling follow-up:

- [#10 — review core Element and Widget protocol concentration](https://github.com/dornglut/runen-ui/issues/10) remains open and non-blocking;
- [#12 — evaluate widget-declared event output capacity after M4](https://github.com/dornglut/runen-ui/issues/12) is closed; current capacity is sufficient for accepted M5 work.

M4A through M4C2 are accepted imported history. They are recorded in the
[public-repository migration history](history/public-repository-migration.md)
rather than recreated as false closed public issues.

## Required pickup sequence

1. Read the roadmap and status map to identify the active milestone.
2. Open the milestone umbrella and active slice/readiness issue. For M6, use #71 and #59 for the first M6A implementation slice.
3. Verify exact accepted `main`, branch, pull request, and live head.
4. Read linked ADRs, accepted charters/matrices where applicable, and stable architecture contracts.
5. Inspect current source, tests, unresolved review findings, and exact-head CI.
6. Execute only the current issue or an explicitly linked prerequisite.
7. Update execution records after each reviewed green checkpoint, material review correction, readiness transition, and merge.
8. Never begin the next slice from an unmerged feature or authority branch.

A new thread at the current implementation boundary should need only:

```text
Repository: dornglut/runen-ui
Completed milestones: M0–M5
Active milestone: M6 renderer-neutral paint/hit scene protocol
Accepted M6A0 authority: ADR 0007 + 36-row matrix, all 36 rows blocked
Post-A0 current-contract reconciliation: complete through PR #75
Current implementation slice: #59 / M6A retained-publication substrate
Owned rows: SCENE-PUB-01..05
Do not introduce M6B scene APIs or renderer/backend behavior in #59.
```

## Execution and branch discipline

One feature branch has one active writer. Parallel analysis and review are
allowed, but repository writes must be serialized through one execution path.
Before every write, re-read the live pull-request head and the target file's blob
SHA. A moved head invalidates earlier file snapshots, validation claims, and
review conclusions until they are refreshed.

Do not run multiple agents that independently commit to the same branch. That
creates stale writes, cancelled CI runs, duplicate implementations, and
unreviewable interleaving. When responsibility changes, record the last reviewed
checkpoint and hand off the branch explicitly.

Use the smallest suitable execution path:

- structured GitHub state changes and bounded text edits may use the connected GitHub interface;
- broad, generated, binary, or validation-heavy changes use a checked-out repository executor;
- native GitHub Projects and settings use the GitHub interface when no reviewed automation surface exists.

All paths publish ordinary task branches and pull requests. CI remains
independently read-only. Do not add temporary branch-mutating formatter, fixer,
source-export, or self-authoring workflows to compensate for an execution-tool
limitation.

## Slice issue requirements

Every slice issue records:

- authority documents;
- exact prerequisite and accepted base SHA;
- dependencies;
- included conformance/proof rows where applicable;
- included work and explicit non-goals;
- positive, negative, and trace proof ownership where applicable;
- validation commands;
- target branch and draft PR title;
- current reviewed checkpoint head and blockers;
- acceptance state;
- next unblocked issue.

A matrix-row checklist is updated only for the owning slice. Do not create one
issue per matrix row.

## Acceptance transitions

Where an accepted conformance matrix uses the repository status vocabulary, use
it exactly:

- `blocked`: the owning implementation slice has not been accepted;
- `implementation-complete`: public behavior exists but the complete proof package has not passed;
- `proof-complete`: the exact-head proof package passes but owner acceptance and merge are pending;
- `owner-accepted`: public behavior, complete proof, validation, owner review, and merge have passed, together with either successful exact-head CI or a documented infrastructure-only owner waiver satisfying the policy below.

Later milestone plans may define a different bounded status vocabulary only
through reviewed authority; do not silently reinterpret an accepted matrix's
terms.

Exact-head CI means the workflow explicitly checks out and verifies the feature
head SHA. GitHub's synthetic pull-request merge ref does not qualify. Any head
movement invalidates earlier exact-head evidence.

After a squash merge, record the accepted feature head and squash merge commit
separately. Do not require feature-head ancestry from the squash commit. Verify
changed-file content identity between the reviewed feature head and squash
before treating the merge as accepted implementation evidence.

When a connector-origin merge suppresses the normal default-branch push event,
do not relabel feature-head CI as accepted-main push evidence. Preserve the
missing event as an explicit infrastructure fact. If independent exact-squash CI
is required before continuing, use an existing read-only validation path without
source mutation; record its event type honestly and close any temporary
validation PR unmerged.

## Infrastructure-only exact-head CI waiver

A failed or missing CI result may be waived only when all of the following are
true:

1. The workflow run targets the exact reviewed feature head.
2. The job fails before checkout or before any repository-controlled validation step executes, and the recorded job contains no source, compiler, test, lint, documentation, or policy failure.
3. The complete exact-head local baseline, slice-specific proofs, repository audits, diff checks, remote-head verification, and clean-worktree verification pass.
4. The final diff and unresolved review threads are audited after that validation.
5. The repository owner records an explicit waiver on both the pull request and owning slice issue, names the exact head and failed run, states that CI did not pass, and authorizes a merge pinned to that head.
6. The merge operation uses the expected head SHA and the post-merge closure records the waiver and squash commit.

A waiver is specific to one pull request and one exact head. It becomes invalid
when the head moves. It must never be reported as successful CI, and it cannot
waive a job that executed repository code and reported a validation failure.
Ordinary runner flakiness after steps begin must be rerun or corrected rather
than waived.

## Validation and reporting

Every repository pull request runs the baseline documented in
[`tooling/validation.md`](tooling/validation.md), plus slice-specific tests and:

- applicable conformance-matrix uniqueness/status/schema/count audits;
- public API and removed-symbol audit;
- unsafe-code audit;
- cross-document truth audit;
- repository structure and authority audit;
- exact base/head/remote verification;
- clean-worktree verification;
- exact-head CI verification or the narrowly documented infrastructure-only waiver above.

PR CI validates the exact reviewed feature head, and head movement invalidates
prior evidence. CI is read-only and does not maintain pull-request comments.
Successful output is compact; failed output is bounded while complete
failed-command output is retained through a short-retention failure-only artifact
outside the checkout. Temporary diagnostics are removed and successful runs create
no diagnostic artifact. Reviewed feature head, synthetic merge result when
separately used, squash merge, exact-squash validation, and accepted-main push
validation are distinct evidence objects and must be named accurately.

Do not reuse validation or CI claims from an earlier head.