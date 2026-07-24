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

### M4 conformance matrix

[`architecture/m4-conformance-matrix.md`](architecture/m4-conformance-matrix.md)
owns permanent behavioral acceptance IDs, observable requirements, positive proof
ownership, negative proof ownership, trace proof ownership, delivery slice,
status, and M4-gate classification.

Matrix rows are not GitHub issues. One slice issue may own many matrix rows.

### GitHub umbrella and execution issues

The [public M4 umbrella issue](https://github.com/dornglut/runen-ui/issues/3)
and its linked execution issues own volatile state:

- exact accepted base SHA;
- active branch and draft pull request;
- current reviewed checkpoint head;
- dependencies and blockers;
- matrix-row checklists for the active slice;
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

## M4 operational milestone

The public operational milestone is
[issue #3](https://github.com/dornglut/runen-ui/issues/3). A formal GitHub
milestone should be named `M4 — Events, Effects, Scheduling, and Trace v2`.
Until that milestone is configured through GitHub UI or another reviewed
administration surface, the umbrella issue remains the pickup surface and
dependency graph.

The public execution graph is:

- [#2 — public RunenUI authority cutover](https://github.com/dornglut/runen-ui/issues/2);
- [#11 — deterministic repository structure and authority audit](https://github.com/dornglut/runen-ui/issues/11), after #2;
- [#4 — M4C3 pointer lifecycle](https://github.com/dornglut/runen-ui/issues/4), after #11 and its readiness freeze;
- [#5 — M4C4 focus scopes and modality](https://github.com/dornglut/runen-ui/issues/5), after #4;
- [#6 — M4C5 keyboard, text, IME, automation, and M4C closure](https://github.com/dornglut/runen-ui/issues/6), after #5;
- [#7 — M4D1 complete trace schema and causality](https://github.com/dornglut/runen-ui/issues/7), after #6;
- [#8 — M4D2 export, redaction, and bounded sink](https://github.com/dornglut/runen-ui/issues/8), after #7;
- [#9 — M4D3 replay and M4 closure](https://github.com/dornglut/runen-ui/issues/9), after #8.

Architecture and tooling follow-up:

- [#10 — review core Element and Widget protocol concentration](https://github.com/dornglut/runen-ui/issues/10), non-blocking unless a later readiness audit proves otherwise;
- [#12 — evaluate widget-declared event output capacity after M4](https://github.com/dornglut/runen-ui/issues/12), explicitly deferred until after M4.

M4A through M4C2 are accepted imported history. They are recorded in the
[public-repository migration history](history/public-repository-migration.md)
rather than recreated as false closed public issues.

## Required pickup sequence

1. Read the public M4 umbrella issue.
2. Open the execution issue it identifies as current.
3. Verify exact accepted `main`, branch, pull request, and live head.
4. Read linked ADRs, the accepted charter, matrix rows, and stable architecture contracts.
5. Inspect current source, tests, unresolved review findings, and exact-head CI.
6. Execute only the current issue or an explicitly linked prerequisite.
7. Update execution records after each reviewed green checkpoint, material review correction, readiness transition, and merge.
8. Never begin the next slice from an unmerged feature branch.

A new thread should need only:

```text
Repository: dornglut/runen-ui
Umbrella issue: #3
Current execution issue: read from #3
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
- included matrix rows;
- included work and explicit non-goals;
- positive, negative, and trace proof ownership;
- validation commands;
- target branch and draft PR title;
- current reviewed checkpoint head and blockers;
- acceptance state;
- next unblocked issue.

The matrix-row checklist is updated only for the owning slice. Do not create one
issue per matrix row.

## Acceptance transitions

Use matrix statuses exactly:

- `blocked`: the owning implementation slice has not been accepted;
- `implementation-complete`: public behavior exists but the complete proof package has not passed;
- `proof-complete`: the exact-head proof package passes but owner acceptance and merge are pending;
- `owner-accepted`: public behavior, complete proof, validation, owner review, and merge have passed, together with either successful exact-head CI or a documented infrastructure-only owner waiver satisfying the policy below.

Exact-head CI means the workflow explicitly checks out and verifies the feature
head SHA. GitHub's synthetic pull-request merge ref does not qualify. Any head
movement invalidates the prior exact-head result.

After a squash merge, record the accepted feature head and squash merge commit
separately. Do not require feature-head ancestry from the squash commit.

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

- matrix uniqueness/status/schema/count audit;
- public API and removed-symbol audit;
- unsafe-code audit;
- cross-document truth audit;
- repository structure and authority audit;
- exact base/head/remote verification;
- clean-worktree verification;
- exact-head CI verification or the narrowly documented infrastructure-only waiver above.

The CI workflow may maintain one marker-owned failure comment containing the
exact head, run URL, and bounded diagnostic excerpt. It is transient diagnostic
state and must be removed automatically after a successful exact-head run. The
complete Actions log remains authoritative.

Do not reuse validation or CI claims from an earlier head.
