# RunenUI Agent Contract

`AGENTS.md` is the executor entrypoint for repository work. It does not duplicate project history, live GitHub state, or the full architecture.

## Start from current authority

For nontrivial work:

1. inspect the current repository and default branch rather than relying on an earlier handoff;
2. identify the owning GitHub issue or accepted decision;
3. inspect the affected code and executable tests for current behavior;
4. read the relevant local ADR, architecture/design, and conformance contract;
5. consult the roadmap only when durable sequencing or dependencies matter;
6. consult Dornglut Engineering governance when the repository does not define a more specific rule.

Authority is separated by question:

- code and executable tests own current behavior;
- accepted ADRs and durable designs own architecture decisions;
- conformance documents own permanent observable/proof contracts;
- GitHub issues and the Engineering Portfolio own active work and live priority/status;
- pull requests and exact-head CI own delivery/review evidence;
- `docs/roadmap.md` owns durable sequence and dependencies;
- `docs/status.md` owns accepted capability maturity;
- reports/history are evidence and context, not current implementation authority.

When two retained documents appear to define the same rule, treat that as an authority defect. Correct the canonical owner and replace duplicate detail with a relationship or link.

## Scope and work selection

- Keep nontrivial changes issue-owned and bounded by the accepted outcome, acceptance criteria, dependencies, and non-goals.
- Do not begin implementation from a stale plan, report, review conclusion, or unmerged predecessor branch.
- Start from accepted `main`; never stack required successor work on an unmerged feature branch.
- Preserve unrelated changes and ownership boundaries.
- Prefer clean pre-1.0 cutovers. Do not preserve obsolete APIs, document paths, queues, stores, or compatibility layers without an explicit current compatibility requirement and removal condition.
- Do not restore historical/legacy source as active authority.
- Do not create generated prompts, work-state ledgers, activation documents, self-authoring workflows, or parallel sources of truth.

## Architecture constraints

- `runenui_core` owns host-neutral public values and protocols, not live runtime state, platforms, or renderers.
- `runenui_runtime` is the sole live authority for mounted/semantic storage, reconciliation, routing, scheduling, publication, trace, and shutdown.
- `runenui_testing` is downstream convenience over public core/runtime contracts; it must not gain private mutation, identity fabrication, or a parallel expected runtime model.
- Application/product state remains outside framework runtime ownership.
- Renderer-facing products must not own semantic widget behavior; platform adapters and backends must not become UI behavior authorities.
- Add crates only when a real ownership, dependency, optionality, independent-consumer, or conformance boundary requires Cargo enforcement.
- File size alone is not an extraction or crate-boundary argument.

See [Architecture](ARCHITECTURE.md), [workspace structure](docs/architecture/workspace-structure.md), and [documentation architecture](docs/documentation-architecture.md).

## Validation

For intentional Rust changes, format with:

```text
cargo +stable fmt --all
```

Run focused tests/proofs while implementing, then the canonical repository baseline:

```text
cargo validate
git diff --check
```

`cargo validate` is the repository-owned read-only baseline used by CI. A successful result from an earlier head is stale after the head moves. Report local execution, source inspection, user-reported evidence, and hosted CI distinctly; never claim a command or runtime behavior that was not observed.

For changes governed by conformance rows, review the exact affected permanent IDs and required positive, negative, diagnostic/trace proof obligations. In-flight progress belongs in the issue/PR; durable conformance state changes only with accepted repository state.

## Delivery

Use one coherent feature branch and pull request per bounded delivery. Pull requests record the owning authority, accepted base when relevant, reviewed feature head, outcome, included/excluded scope, conformance impact, validation, and public/migration/security impact.

Open draft pull requests by default. Review the complete diff and relevant unchanged authority before acceptance. Moving the feature head invalidates earlier exact-head review/CI. Merge only with explicit repository-owner authorization and the repository's accepted merge method.
