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
- [History](history/) — non-authoritative historical evidence and migration provenance.

## Documentation classes

| Location | Responsibility |
|---|---|
| repository root | concise public/executor/testing entrypoints |
| `architecture/` | current durable system, ownership, dependency, and public-contract structure |
| `adr/` | accepted durable decisions and consequences |
| `design/` | accepted target designs when a real design exists independently of an ADR or conformance bundle |
| `conformance/` | permanent observable/proof contracts and their directly supporting milestone contract material |
| `roadmap.md` | durable high-level sequence, dependencies, gates, and outcomes |
| `status.md` | current accepted maturity/capability summary |
| `tooling/` | maintained repository procedures |
| `reports/` | point-in-time investigations or external compatibility evidence |
| `history/` | superseded or historical provenance |

Directories are created only when they contain real material; the taxonomy is semantic, not a requirement to pre-create empty folders.

## Live work

GitHub issues and the Engineering Portfolio own active work, blockers, priority, and execution state. Pull requests and exact-head CI own delivery/review evidence. Durable Markdown does not mirror current branches, heads, workflow runs, or pickup state.

Current implementation behavior remains owned by code and executable tests. Documentation summarizes or constrains the concern owned by its class; it does not override observed implementation through a stale status statement.
