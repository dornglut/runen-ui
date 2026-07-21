# Work Tracking

> **Category: Current contract**

RunenUI separates durable architecture authority from volatile execution state. This document defines where each kind of truth lives and the required pickup workflow for contributors and automated agents.

## Authority boundaries

### Roadmap

[`roadmap.md`](roadmap.md) owns milestone order, dependencies, durable included scope, explicit non-goals, exit criteria, and the long-term production sequence. It must not become a branch or pull-request log.

### M4 conformance matrix

[`architecture/m4-conformance-matrix.md`](architecture/m4-conformance-matrix.md) owns permanent behavioral acceptance IDs, observable requirements, positive proof ownership, negative proof ownership, trace proof ownership, delivery slice, status, and M4-gate classification.

Matrix rows are not GitHub issues. One slice issue may own many matrix rows.

### GitHub umbrella and slice issues

The [M4 umbrella issue](https://github.com/Crystonix/runen-ui/issues/78) and its linked slice issues own volatile execution state:

- exact accepted base SHA;
- active branch and draft pull request;
- current reviewed and remote heads;
- dependencies and blockers;
- matrix-row checklists for the active slice;
- latest validation and exact-head CI or an explicit infrastructure-only waiver;
- acceptance state;
- next action and next unblocked issue.

Architecture and debt issues own work that is real but not part of a slice's accepted behavior. They must state whether they block the active slice.

### Pull requests

A pull request owns its exact implementation or governance diff, public API accounting, structure changes, tests, validation results, CI run, review findings, deferred scope, and any explicit infrastructure-only CI waiver. It must not redefine accepted behavior that belongs to an ADR, charter, or matrix row.

### Status map and support matrix

[`status-map.md`](status-map.md) and [`feature-support-matrix.md`](feature-support-matrix.md) own current accepted repository truth. They report what exists after accepted merges and must not describe unmerged branch work as current support.

## M4 operational milestone

The current operational milestone is [issue #78](https://github.com/Crystonix/runen-ui/issues/78). A formal GitHub milestone should be created as `M4 — Events, Effects, Scheduling, and Trace v2` when milestone creation is available through GitHub UI or CLI. Until then, the umbrella issue is the pickup surface and dependency graph.

Current M4 slice issues:

- [#79 — M4C1 routed semantic-command kernel](https://github.com/Crystonix/runen-ui/issues/79), completed;
- [#80 — M4C2 displayed-generation surface context](https://github.com/Crystonix/runen-ui/issues/80), completed;
- [#81 — M4C3 pointer lifecycle](https://github.com/Crystonix/runen-ui/issues/81), active next slice;
- [#82 — M4C4 focus scopes and modality](https://github.com/Crystonix/runen-ui/issues/82), blocked by M4C3;
- [#83 — M4C5 keyboard, text, IME, automation, and M4C closure](https://github.com/Crystonix/runen-ui/issues/83), blocked by M4C4;
- [#84 — M4D1 complete trace schema and causality](https://github.com/Crystonix/runen-ui/issues/84), blocked by M4C5;
- [#85 — M4D2 export, redaction, and bounded sink](https://github.com/Crystonix/runen-ui/issues/85), blocked by M4D1;
- [#86 — M4D3 replay and M4 closure](https://github.com/Crystonix/runen-ui/issues/86), blocked by M4D2.

Required pre-M4C2 architecture issues [#87](https://github.com/Crystonix/runen-ui/issues/87), [#88](https://github.com/Crystonix/runen-ui/issues/88), and [#89](https://github.com/Crystonix/runen-ui/issues/89) are completed.

## Required pickup sequence

1. Read the umbrella issue.
2. Open the active slice issue.
3. Verify exact accepted `master`, branch, pull request, and head.
4. Read linked ADRs, the accepted charter, matrix rows, and stable architecture contracts.
5. Inspect current source, tests, and unresolved review findings.
6. Execute only the active slice or explicitly linked prerequisite issue.
7. Update the issue after every accepted head, review correction, and merge.
8. Never begin the next slice from an unmerged feature branch.

A new thread should need only:

```text
Repository: Crystonix/runen-ui
Umbrella issue: #78
Active slice issue: #...
```

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
- current head and blockers;
- acceptance state;
- next unblocked issue.

The matrix-row checklist is updated only for the owning slice. Do not create one issue per matrix row.

## Acceptance transitions

Use matrix statuses exactly:

- `blocked`: the owning implementation slice has not been accepted;
- `implementation-complete`: public behavior exists but the complete proof package has not passed;
- `proof-complete`: the exact-head proof package passes but owner acceptance and merge are pending;
- `owner-accepted`: public behavior, complete proof, validation, owner review, and merge have passed, together with either successful exact-head CI or a documented infrastructure-only owner waiver satisfying the policy below.

After a squash merge, record the accepted feature head and squash merge commit separately. Do not require feature-head ancestry from the squash commit.

## Infrastructure-only exact-head CI waiver

A failed or missing CI result may be waived only when all of the following are true:

1. The workflow run targets the exact reviewed feature head.
2. The job fails before checkout or before any repository-controlled validation step executes, and the recorded job contains no source, compiler, test, lint, documentation, or policy failure.
3. The complete exact-head local baseline, slice-specific proofs, repository audits, diff checks, remote-head verification, and clean-worktree verification pass.
4. The final diff and unresolved review threads are audited after that validation.
5. The repository owner records an explicit waiver on both the pull request and owning slice issue, names the exact head and failed run, states that CI did not pass, and authorizes a merge pinned to that head.
6. The merge operation uses the expected head SHA and the post-merge closure records the waiver and squash commit.

A waiver is specific to one pull request and one exact head. It becomes invalid when the head moves. It must never be reported as successful CI, and it cannot waive a job that executed repository code and reported a validation failure. Ordinary runner flakiness after steps begin must be rerun or corrected rather than waived.

## Validation and reporting

Every repository pull request runs the baseline documented in [`tooling/validation.md`](tooling/validation.md), plus slice-specific tests and:

- matrix uniqueness/status/schema/count audit;
- public API and removed-symbol audit;
- unsafe-code audit;
- cross-document truth audit;
- exact base/head/remote verification;
- clean-worktree verification;
- exact-head CI verification or the narrowly documented infrastructure-only waiver above.

Do not reuse validation or CI claims from an earlier head.
