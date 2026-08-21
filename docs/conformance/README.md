# Conformance

This directory owns RunenUI's permanent observable requirements and proof obligations, plus accepted milestone contract material that directly defines how those observations are interpreted or delivered.

## Files

- [M4 conformance matrix](m4-conformance-matrix.md) — permanent M4 observable/proof inventory, including inherited semantic-access gates.
- [M4 directional-focus corpus](m4-directional-focus-corpus.md) — fixed public-outcome vectors for directional focus.
- [M4C routed-transaction charter](m4c-delivery-and-routed-transaction-charter.md) — accepted protocol/transaction/slice context for the M4 matrix.
- [M5 semantics and testing charter](m5-semantics-and-testing-charter.md) — accepted durable M5 semantic/testing boundaries.
- [M5 conformance matrix](m5-conformance-matrix.md) — permanent M5 observable/proof inventory.
- [M6 conformance matrix](m6-conformance-matrix.md) — permanent renderer-neutral scene observable/proof inventory.

Accepted architecture decisions remain in [ADRs](../adr/). Current implementation maturity is summarized in [status](../status.md).

## Authority rules

Each conformance ID is permanent and has one observable requirement plus explicit positive, negative, and diagnostic/trace proof ownership. IDs are not recycled when implementation moves.

Matrix status is **accepted default-branch conformance state**. In-flight implementation, proof, review, branch/head, and CI evidence belong in the owning GitHub issue and pull request. A feature branch does not update durable matrix state merely to describe its own progress.

A row reaches an accepted state only through the repository's normal implementation, proof, validation, review, owner-acceptance, and merge process. The matrix records the resulting accepted repository truth; the pull request remains the delivery evidence.

Conformance documents may depend on accepted ADRs/current architectural owners needed to state an observation, but they must not become a second implementation architecture or live work tracker. Target conformance does not imply current public API before implementation.

Repository validation checks matrix schema, permanent-ID uniqueness, declared summary consistency, allowed accepted-state vocabulary, and gate policy. Behavioral correctness remains proven by executable tests and review rather than Markdown parsing alone.
