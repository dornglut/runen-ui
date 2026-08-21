# RunenUI Agent Contract

`AGENTS.md` is the executor entrypoint for repository work. It defines how to locate authority, select scope, preserve ownership, validate changes, and deliver reviewed work. It does not duplicate project history or live GitHub state.

## Start from current authority

For nontrivial work:

1. inspect the current repository, default branch, and open matching work before relying on an earlier handoff;
2. if continuing an existing issue or pull request, verify its exact base, branch, head, diff, review state, and current acceptance criteria before editing;
3. if starting a new slice, identify the owning GitHub issue or accepted decision and start from accepted `main`, never an unmerged predecessor branch;
4. inspect the affected implementation and executable tests to determine what behavior currently exists;
5. read the relevant accepted ADR, current architecture/design owner, and conformance contract to determine what behavior is required;
6. consult the roadmap only when durable sequencing or dependencies matter;
7. consult Dornglut Engineering governance only where this repository does not define a more specific rule.

Authority is separated by question:

- code and executable tests are evidence of current implementation behavior;
- accepted ADRs and durable designs define architectural decisions and constraints;
- conformance documents define permanent observable and proof obligations;
- GitHub issues and the Engineering Portfolio own active work, priority, blockers, and execution state;
- pull requests and exact-head CI own delivery and review evidence;
- `docs/roadmap.md` owns durable sequence and dependencies;
- `docs/status.md` owns accepted capability maturity;
- reports and history provide evidence or provenance, not current implementation authority.

Implementation does not silently amend an accepted contract. If current code/tests conflict with an accepted ADR, architecture constraint, or conformance observation, treat the mismatch as a defect or explicitly revise the owning contract through review.

When two retained documents independently define the same rule, treat that as an authority defect. Correct the canonical owner and replace duplicate detail with a relationship statement or link.

## Scope and work selection

- Keep nontrivial changes issue-owned and bounded by accepted outcome, acceptance criteria, dependencies, and non-goals.
- Resume matching in-flight work on its exact branch after verification; do not restart it from `main` merely because the executor changed.
- Start a new successor slice only from accepted `main`; never stack required successor work on an unmerged feature branch.
- Preserve unrelated changes and ownership boundaries.
- Prefer clean pre-1.0 cutovers. Do not preserve obsolete APIs, document paths, queues, stores, or compatibility layers without an explicit current compatibility requirement and removal condition.
- Do not restore historical or legacy source as active authority.
- Do not create generated prompts, work-state ledgers, activation documents, self-authoring workflows, or parallel sources of truth.

## Architecture constraints

- `runenui_core` owns host-neutral public values and protocols, not live runtime state, platforms, or renderers.
- `runenui_runtime` is the sole live authority for mounted/semantic storage, reconciliation, routing, scheduling, publication, trace, and shutdown.
- `runenui_testing` is downstream convenience over public core/runtime contracts; it must not gain private mutation, identity fabrication, or a parallel expected runtime model.
- Application and product state remain outside framework runtime ownership.
- Renderer-facing products must not own semantic widget behavior; platform adapters and backends must not become UI behavior authorities.
- Add crates only when a real ownership, dependency, optionality, independent-consumer, or conformance boundary requires Cargo enforcement.
- File size alone is not an extraction or crate-boundary argument.

See [Architecture](ARCHITECTURE.md), [workspace structure](docs/architecture/workspace-structure.md), and [documentation architecture](docs/documentation-architecture.md).

## Validation

For intentional Rust changes, format with:

```text
cargo +stable fmt --all
```

Run focused tests and proof obligations while implementing, then the canonical repository baseline:

```text
cargo validate
git diff --check
```

`cargo validate` is the repository-owned read-only baseline used by CI. A successful result from an earlier head is stale after the head moves. Report source inspection, local execution, user-reported evidence, and hosted CI distinctly; never claim a command or runtime behavior that was not observed.

For changes governed by conformance rows, review the exact permanent IDs and required positive, negative, diagnostic, or trace proof obligations. In-flight progress belongs in the issue and pull request; durable matrix status changes only when accepted repository state changes.

## Delivery

Use one coherent feature branch and pull request per bounded delivery. Pull requests record the owning authority, accepted base when relevant, frozen reviewed feature head, outcome, included/excluded scope, conformance impact, validation, and public/migration/security impact. Do not copy live head values into durable repository documentation.

Open draft pull requests by default. Review the complete diff and relevant unchanged authority before acceptance. Moving the feature head invalidates earlier exact-head review and CI. Merge only with explicit repository-owner authorization and the repository's accepted merge method.
