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
status, and gate classification. After M4 closure its two remaining blocked rows
are M5-owned semantic/accessibility inputs rather than unfinished M4 work.

[`architecture/m5-conformance-matrix.md`](architecture/m5-conformance-matrix.md)
owns the separate M5-specific behavioral acceptance inventory. It deliberately
does not duplicate inherited M4 `ACCESS-01` and `ACCESS-02`; M5C completes those
rows in the M4 matrix while M5-specific semantic identity, product, action,
testing, migration, and closure observations remain in the M5 matrix.

The canonical repository audit validates every configured matrix, each matrix's
own declared status/schema/count truth, delivery/gate policy, and permanent ID
uniqueness across the configured matrix set.

Matrix rows are not GitHub issues. One slice issue may own many matrix rows.

### GitHub umbrella and execution issues

The closed [M4 umbrella issue](https://github.com/dornglut/runen-ui/issues/3)
and its linked execution issues preserve M4's volatile delivery history. The
successor [M5 umbrella issue #45](https://github.com/dornglut/runen-ui/issues/45)
is the active M5 coordination and pickup surface. Each active milestone issue
owns volatile state such as:

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

M5 uses [issue #45](https://github.com/dornglut/runen-ui/issues/45) as its active
coordination and pickup authority. Its decision-complete readiness audit passed
against exact accepted M4 closure base
`a63a249de9d4d53eeef4104ae3384e7898aacad1`. M5A0 #46 is accepted and closed.
M5A semantic contribution and independent identity #47 was owner-accepted at
exact reviewed feature head `8377ced53c08d7b5be3020368ceddd3ee81294a5`
after exact-head CI run `31497457992` / #889 passed, then guarded-squash-merged
in PR #53 as `e3c304600ec1777cd17a1973946a43c765df1c31`. All 38 feature
changed-file blob identities are byte-identical between reviewed head and
accepted squash, and accepted `main` was verified at that squash.

The current execution gate is the separate M5A post-merge authority/current-
contract reconciliation from exact base `e3c304600ec1777cd17a1973946a43c765df1c31`.
It promotes exactly the twelve M5A-owned matrix rows to `owner-accepted` and
reconciles accepted public/status/pickup truth. #47 remains open and M5B #48
remains blocked until that reconciliation itself is exact-head validated,
critically reviewed, explicitly owner-accepted, merged, and accepted-main
verified.

The completed public M4 execution graph is:

- [#2 — public RunenUI authority cutover](https://github.com/dornglut/runen-ui/issues/2);
- [#11 — deterministic repository structure and authority audit](https://github.com/dornglut/runen-ui/issues/11), after #2;
- [#4 — M4C3 pointer lifecycle](https://github.com/dornglut/runen-ui/issues/4), after #11 and its readiness freeze;
- [#5 — M4C4 focus scopes and modality](https://github.com/dornglut/runen-ui/issues/5), after #4;
- [#6 — M4C5 keyboard, text, IME, automation, and M4C closure](https://github.com/dornglut/runen-ui/issues/6), after #5;
- [#7 — M4D1 complete trace schema and causality](https://github.com/dornglut/runen-ui/issues/7), after #6;
- [#8 — M4D2 export, redaction, and bounded sink](https://github.com/dornglut/runen-ui/issues/8), after #7;
- [#9 — M4D3 replay and M4 closure](https://github.com/dornglut/runen-ui/issues/9), after #8.

The accepted M5 execution graph is sequential:

- [#46 — M5A0 semantic/testing architecture and conformance authority](https://github.com/dornglut/runen-ui/issues/46), accepted and closed;
- [#47 — M5A semantic contribution and independent identity](https://github.com/dornglut/runen-ui/issues/47), feature accepted/merged; post-merge authority reconciliation is the current gate;
- [#48 — M5B semantic tree publication and incremental updates](https://github.com/dornglut/runen-ui/issues/48), after accepted #47 reconciliation;
- [#49 — M5C semantic action ingress and accessibility resolution](https://github.com/dornglut/runen-ui/issues/49), after accepted #48;
- [#50 — M5D public deterministic headless testing harness](https://github.com/dornglut/runen-ui/issues/50), after accepted #49;
- [#51 — M5E integrated conformance, migration, and M5 closure](https://github.com/dornglut/runen-ui/issues/51), after accepted #50.

No later M5 branch begins from an unmerged predecessor or from a feature merge
whose required post-merge authority reconciliation is still pending. M6 is
eligible only from the exact accepted M5 closure base.

Architecture and tooling follow-up:

- [#10 — review core Element and Widget protocol concentration](https://github.com/dornglut/runen-ui/issues/10) remains open and non-blocking; M5 uses the focused semantic ownership seam unless concrete coupling proves broader work necessary;
- [#12 — evaluate widget-declared event output capacity after M4](https://github.com/dornglut/runen-ui/issues/12) is closed completed with the post-M4 decision that current capacity is sufficient for accepted M5 work; semantic actions are ingress to the canonical command authority, not a new widget-output family.

M4A through M4C2 are accepted imported history. They are recorded in the
[public-repository migration history](history/public-repository-migration.md)
rather than recreated as false closed public issues.

## Required pickup sequence

1. Read the roadmap and status map to identify the active milestone.
2. Open that milestone's GitHub umbrella/pickup issue; for M5 this is #45.
3. Verify exact accepted `main`, branch, pull request, and live head.
4. Read linked ADRs, accepted charters/matrices where applicable, and stable architecture contracts.
5. Inspect current source, tests, unresolved review findings, and exact-head CI.
6. Execute only the current issue or an explicitly linked prerequisite.
7. Update execution records after each reviewed green checkpoint, material review correction, readiness transition, and merge.
8. Never begin the next slice from an unmerged feature or authority branch.

A new M5 thread should need only:

```text
Repository: dornglut/runen-ui
Current milestone: M5
Umbrella issue: #45
Current execution issue: read from #45
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
movement invalidates the prior exact-head result.

After a squash merge, record the accepted feature head and squash merge commit
separately. Do not require feature-head ancestry from the squash commit. Verify
changed-file content identity between the reviewed feature head and squash
before treating the merge as accepted implementation evidence.

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
separately used, squash merge, and accepted-main push validation are distinct
evidence objects.

Do not reuse validation or CI claims from an earlier head.
