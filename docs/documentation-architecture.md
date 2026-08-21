# Documentation Architecture

This document owns RunenUI documentation placement and authority boundaries. It does not define framework behavior, live project state, or implementation priority.

## One concern, one owner

Every retained active document has one primary job. If changing one rule requires editing multiple documents because each independently defines that rule, the decomposition is defective. Keep the detailed rule in one canonical owner and replace other copies with a relationship statement or link.

Current behavior is established by code and executable tests. Accepted ADRs and durable designs constrain architecture. Permanent conformance documents constrain observable acceptance/proof. GitHub issues and the Engineering Portfolio own live work. Pull requests and exact-head CI own delivery evidence.

## Artifact ownership

| Artifact | Owns | Must not become |
|---|---|---|
| `README.md` | public purpose, maturity summary, decisive capabilities/limits, validation, navigation | roadmap, API manual, delivery ledger |
| `AGENTS.md` | executor startup, ownership constraints, prohibited operations, validation/delivery contract | project history, current-work database |
| `ARCHITECTURE.md` | concise system/dependency map and long-form architecture navigation | complete architecture reference, milestone status ledger |
| `TESTING.md` | concise testing/validation/evidence map | duplicated command implementation or CI ledger |
| `docs/architecture/` | current durable system boundaries, ownership, dependency direction, conceptual public contract | live delivery state or target-only feature claims |
| `docs/adr/` | durable accepted decisions, alternatives, consequences | current issue/PR state |
| `docs/design/` | accepted target behavior/migration contracts when independently warranted | speculative taxonomy or implementation ledger |
| `docs/conformance/` | permanent observable requirements, proof obligations, supporting accepted milestone contract material | branch/PR/review-state database |
| `docs/roadmap.md` | durable outcome sequence, dependencies, major gates, non-goals | branch/issue/PR inventory |
| `docs/status.md` | current accepted capability maturity and decisive limitations | transient blocker or pickup map |
| `docs/tooling/` | maintained repository procedures and machine-enforced contracts | product behavior authority |
| `docs/reports/` | point-in-time investigation, compatibility, benchmark, or audit evidence | active architecture/work authority |
| `docs/history/` | superseded decisions, migrations, provenance, recovery context | active implementation authority |

## Dependency direction

Current architecture may cite accepted ADRs and conformance contracts. Conformance material may cite accepted ADRs/current architectural owners needed to define an observation. Roadmap and status may summarize architecture/conformance outcomes, but they do not acquire authority over those detailed contracts.

Reports and history may cite any artifact needed to preserve provenance, but active documents do not derive current truth from historical records.

Root entrypoints summarize and route. They should remain stable when a feature branch, pull request, CI run, or active issue changes.

## Live operational state

Do not store the following as active durable Markdown truth:

- current branch or feature-head SHA;
- pull-request inventory or current PR number as workflow state;
- workflow-run identifiers;
- current assignee, blocker, priority, or next-action state;
- copied Project fields;
- speculative branch names or draft PR titles;
- acceptance checkpoints that belong to a pull request.

Stable historical references may remain in `history/`, `reports/`, ADR rationale, or changelog material when they are genuinely needed as provenance. They must not be used to select current work.

## Conformance state

Permanent conformance IDs and their proof obligations remain repository authority. A matrix status represents accepted default-branch conformance state only. In-flight implementation, proof, review, and exact-head evidence stay in the owning issue and pull request until acceptance changes repository state.

IDs are never recycled. Moving a conformance file or reorganizing documentation must not change the meaning of an accepted observation.

## Navigation and links

Use inline relative Markdown links for repository-internal documentation. Directory README files are indexes and orientation pages, not shadow specifications.

When moving or deleting documentation, update inbound links in the same coherent cutover. Do not retain forwarding Markdown stubs solely to preserve obsolete internal paths unless a real external compatibility contract requires them.

## Growth rule

Split documents when concerns evolve under materially different ownership or review obligations, not merely because a file is long. Create new directories or artifact classes only when real material requires them.

Repository validation enforces structural parts of this model. Semantic duplication still requires review; tooling must not pretend to prove arbitrary prose equivalence.
