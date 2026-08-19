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

M0–M5 are complete. M6A0 architecture/conformance authority and its required
bounded current-contract reconciliation are also accepted, but no M6 scene
behavior is implemented yet. PR #73 accepted ADR 0007 and the 36-row M6 matrix;
PR #75 completed the post-A0 current-contract reconciliation. All 36 behavior
rows remain `blocked`. The first M6A implementation slice is
[#59](https://github.com/dornglut/runen-ui/issues/59), limited to the persistent
retained-publication substrate required by `SCENE-PUB-01..05`; it does not
introduce M6B scene APIs or renderer/backend behavior.

Current implementation maturity belongs in the [status map](docs/status-map.md). Delivery sequence belongs in the [roadmap](docs/roadmap.md). Live branch and issue state belongs in GitHub according to the [work-tracking contract](docs/work-tracking.md).