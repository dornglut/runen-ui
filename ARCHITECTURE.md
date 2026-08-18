# RunenUI Architecture

RunenUI is a host-neutral, renderer-neutral Rust UI framework. Its durable ownership direction is:

```text
application state and actions
    -> transient View/Element authoring
    -> keyed reconciliation
    -> persistent mounted runtime tree
    -> interaction, style, layout, and semantics
    -> hit-test and paint products
    -> host integration and renderer backend
```

The mounted tree owns runtime identity, widget state, lifecycle, invalidation, focus, pointer interaction, task/subscription ownership, and publication authority. The authored tree is transient input and must not become a parallel retained runtime. Semantic identity is independently allocated by runtime and published through a renderer-independent semantic product; it is not a mounted-arena alias or renderer-frame field.

Crate and subsystem boundaries, current proof behavior, target products, and accepted constraints are documented in:

- [Detailed architecture](docs/architecture.md)
- [Workspace structure](docs/architecture/workspace-structure.md)
- [Public API](docs/architecture/public-api.md)
- [M4 delivery charter](docs/architecture/m4c-delivery-and-routed-transaction-charter.md)
- [M4 conformance matrix](docs/architecture/m4-conformance-matrix.md)
- [M5 semantics and testing charter](docs/architecture/m5-semantics-and-testing-charter.md)
- [M5 conformance matrix](docs/architecture/m5-conformance-matrix.md)
- [ADR 0007 renderer-neutral paint/hit scene protocol](docs/adr/0007-renderer-neutral-paint-hit-scene-protocol.md)
- [M6 conformance matrix](docs/architecture/m6-conformance-matrix.md)
- [Architecture decision records](docs/adr/)

M0–M5 are complete. M6A0 architecture/conformance authority is also accepted,
but no M6 scene behavior is implemented yet. Exact reviewed PR #73 head
`c0169ebea044a0009a334f3d5ecc13ff8d495885` passed exact-head CI #1349 /
`32181344340`, was repository-owner-authorized, and was guarded-squash-merged as
`966778dd31e0f6b6df76ee4f6283a984fc724b36`. Reviewed and squash trees are
identical at `fe057a3fef9ea6de053ce86ce336212f0aa3a413`; accepted-main CI #1351 /
`32186597198` then validated the exact squash through read-only PR #74, which was
closed unmerged. ADR 0007 and the 36-row M6 matrix are therefore accepted target
architecture/conformance authority while all 36 behavior rows remain `blocked`.
The bounded M6A0 current-contract reconciliation is the final
pre-implementation gate; #59/M6A may begin only after that reconciliation is
itself accepted, merged, tree-verified, and accepted-main validated.

Current implementation maturity belongs in the [status map](docs/status-map.md). Delivery sequence belongs in the [roadmap](docs/roadmap.md). Live branch and issue state belongs in GitHub according to the [work-tracking contract](docs/work-tracking.md).