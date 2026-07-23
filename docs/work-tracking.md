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

### Stable work-package identifiers

Roadmap and delivery identifiers such as `M4`, `M4C3`, and `M4D1` are stable
repository-local identities. Cross-repository references prefix them with the
repository name, for example `runen-ui:M4C3`.

Forge issue numbers, pull-request numbers, native relationships, boards, and
custom fields are current operational projections. They are not durable
architecture identifiers and may change if the repository moves to another
forge.

### M4 conformance matrix

[`architecture/m4-conformance-matrix.md`](architecture/m4-conformance-matrix.md)
owns permanent behavioral acceptance IDs, observable requirements, positive proof
ownership, negative proof ownership, trace proof ownership, delivery slice,
status, and M4-gate classification.

Matrix rows are not tracker issues. One slice issue may own many matrix rows.

### Current forge projection

The [public M4 umbrella issue](https://github.com/dornglut/runen-ui/issues/3)
and its native sub-issues and dependencies own volatile execution state:

- exact accepted base SHA;
- active branch and draft pull request;
- current reviewed and remote heads;
- current dependencies and blockers;
- matrix-row checklists for the active slice;
- latest validation and exact-head CI or an explicit infrastructure-only waiver;
- acceptance state;
- next action and next unblocked issue.

Native tracker relationships are the live hierarchy and dependency projection.
Issue bodies retain semantic prerequisites and acceptance policy, but must not
maintain a second child inventory or duplicate native dependency checklists.

Architecture, governance, and tooling issues own real work outside a slice's
accepted behavior. Each issue must state whether it blocks the next
implementation slice.

### Organization project

The [Dornglut organization project](https://github.com/orgs/dornglut/projects/1)
owns volatile portfolio state such as status, horizon, priority, effort, risk,
start date, and target date. Repository documents do not mirror routine field
changes.

The umbrella issue is the high-level roadmap item. Delivery views may also show
its sub-issues. Project views are replaceable planning projections, not durable
architecture authority.

### Pull requests

A pull request owns its exact implementation or governance diff, public API
accounting, structure changes, tests, validation results, CI run, review
findings, deferred scope, and any explicit infrastructure-only CI waiver. It
must not redefine accepted behavior that belongs to an ADR, charter, or matrix
row.

Pull-request titles and descriptions should include the stable work-package
identifier and the current issue-closing reference where practical.

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

### Forge milestones and releases

Technical roadmap phases are represented operationally by umbrella issues and
native sub-issues. Forge milestones are reserved for release or shipping targets
such as a version, developer preview, public alpha, or first native-host release.
This prevents a roadmap phase and a forge milestone from maintaining identical
membership and competing progress values.

Accepted software versions remain identified by Git tags and release records.

## Forge portability

The repository remains authoritative when the current hosting platform changes.
A forge migration preserves:

- roadmap and work-package identifiers;
- ADRs, architecture contracts, and conformance IDs;
- required proofs and validation commands;
- accepted Git history, tags, and release records.

The destination forge recreates, where supported:

- umbrella and execution issues;
- parent-child and dependency relationships;
- project or board fields and views;
- pull-request or merge-request links.

Loss of a forge-specific planning feature must not erase the durable definition
of an outcome or its acceptance requirements.

## M4 operational work graph

The durable phase identity is `runen-ui:M4`. Its current GitHub projection is
[issue #3](https://github.com/dornglut/runen-ui/issues/3), which is the pickup
surface, native parent, and operational coordination record.

Accepted public prerequisites:

- [#2 — public RunenUI authority cutover](https://github.com/dornglut/runen-ui/issues/2);
- [#11 — deterministic repository structure and authority audit](https://github.com/dornglut/runen-ui/issues/11), after #2.

Remaining ordered M4 delivery work:

- `runen-ui:M4C3` — [#4 — pointer lifecycle](https://github.com/dornglut/runen-ui/issues/4), after #11 and its readiness freeze;
- `runen-ui:M4C4` — [#5 — focus scopes and modality](https://github.com/dornglut/runen-ui/issues/5), after #4;
- `runen-ui:M4C5` — [#6 — keyboard, text, IME, automation, and M4C closure](https://github.com/dornglut/runen-ui/issues/6), after #5;
- `runen-ui:M4D1` — [#7 — complete trace schema and causality](https://github.com/dornglut/runen-ui/issues/7), after #6;
- `runen-ui:M4D2` — [#8 — export, redaction, and bounded sink](https://github.com/dornglut/runen-ui/issues/8), after #7;
- `runen-ui:M4D3` — [#9 — replay and M4 closure](https://github.com/dornglut/runen-ui/issues/9), after #8.

Architecture and tooling follow-up:

- [#10 — review core Element and Widget protocol concentration](https://github.com/dornglut/runen-ui/issues/10), non-blocking unless the M4C3 readiness audit proves otherwise;
- [#12 — evaluate widget-declared event output capacity after M4](https://github.com/dornglut/runen-ui/issues/12), explicitly deferred until after M4.

M4A through M4C2 are accepted imported history. They are recorded in the
[public-repository migration history](history/public-repository-migration.md)
rather than recreated as false closed public issues.

## Required pickup sequence

1. Read the public M4 umbrella issue.
2. Open the execution issue it identifies as current.
3. Verify exact accepted `main`, branch, pull request, and head.
4. Read linked ADRs, the accepted charter, matrix rows, and stable architecture contracts.
5. Inspect current source, tests, and unresolved review findings.
6. Execute only the current issue or an explicitly linked prerequisite.
7. Update the issue after every accepted head, review correction, and merge.
8. Never begin the next slice from an unmerged feature branch.

A new thread should need only:

```text
Repository: dornglut/runen-ui
Umbrella issue: #3
Current execution issue: read from #3
Stable work package: runen-ui:M4C3
```

## Slice issue requirements

Every slice issue records:

- stable work-package identifier;
- authority documents;
- exact prerequisite and accepted base SHA;
- semantic prerequisites;
- included matrix rows;
- included work and explicit non-goals;
- positive, negative, and trace proof ownership;
- validation commands;
- target branch and draft PR title;
- current head and blockers;
- acceptance state;
- next unblocked issue.

The forge owns current parent-child and dependency relationships where supported.
The issue body must not duplicate those relationships as a manually maintained
inventory or checkbox list. It may state the technical prerequisite in prose when
that prerequisite is part of the execution policy.

The matrix-row checklist is updated only for the owning slice. Do not create one
issue per matrix row.

## Acceptance transitions

Use matrix statuses exactly:

- `blocked`: the owning implementation slice has not been accepted;
- `implementation-complete`: public behavior exists but the complete proof package has not passed;
- `proof-complete`: the exact-head proof package passes but owner acceptance and merge are pending;
- `owner-accepted`: public behavior, complete proof, validation, owner review, and merge have passed, together with either successful exact-head CI or a documented infrastructure-only owner waiver satisfying the policy below.

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
- repository structure and authority audit after #11 is accepted;
- exact base/head/remote verification;
- clean-worktree verification;
- exact-head CI verification or the narrowly documented infrastructure-only waiver above.

Do not reuse validation or CI claims from an earlier head.
