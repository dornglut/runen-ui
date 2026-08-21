# RunenUI Documentation

This directory contains the long-form documentation for RunenUI. Each artifact class has one responsibility; cross-link instead of copying detailed rules or live state.

## Start here

- [Documentation architecture](documentation-architecture.md) — placement, authority, and dependency rules for documentation.
- [Current status](status.md) — accepted capability maturity and decisive limitations.
- [Roadmap](roadmap.md) — durable outcome sequence and dependencies.
- [Architecture](architecture/README.md) — current durable system structure and ownership.
- [ADRs](adr/) — accepted durable decisions.
- [Conformance](conformance/README.md) — permanent observable requirements and proof obligations.
- [Tooling](tooling/) — maintained repository validation and audit procedures.
- [Reports](reports/) — point-in-time investigations and external compatibility evidence.
- [History](history/) — non-authoritative historical evidence and migration provenance.

## Documentation classes

| Location | Responsibility |
|---|---|
| repository root | concise public/executor/testing entrypoints |
| `architecture/` | current durable system, ownership, dependency, and conceptual public-contract structure |
| `adr/` | accepted durable decisions and rationale |
| `design/` | accepted target designs when a real design exists independently of an ADR or conformance bundle |
| `conformance/` | permanent observable/proof contracts and directly supporting accepted contract material |
| `roadmap.md` | durable high-level sequence, dependencies, gates, and outcomes |
| `status.md` | current accepted maturity/capability summary |
| `tooling/` | maintained repository procedures |
| `reports/` | point-in-time investigation, compatibility, benchmark, or audit evidence |
| `history/` | superseded or historical provenance |

Directories are created only when they contain real material; the taxonomy is semantic, not a requirement to pre-create empty folders.

## Live work

GitHub issues and the Engineering Portfolio own active work, blockers, priority, and execution state. Pull requests and exact-head CI own delivery/review evidence. Durable current Markdown does not mirror current branches, heads, workflow runs, or pickup state.

Code and executable tests show what the current implementation does. Accepted ADR/architecture/design and conformance contracts define what it is required to do. A mismatch is a defect or requires explicit reviewed contract revision; stale documentation never silently overrides implementation, and implementation never silently overrides accepted contracts.
