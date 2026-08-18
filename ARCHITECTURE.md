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
- [Architecture decision records](docs/adr/)

M0–M4 are complete. M5 is active: M5A–M5D are accepted and reconciled, while M5E #51 is the sole active integration/migration/closure slice from accepted main `3c50f2fe0732871a3e2fdf7dba45983a23b813a1`. M6 implementation remains blocked until accepted M5 closure.

Current implementation maturity belongs in the [status map](docs/status-map.md). Delivery sequence belongs in the [roadmap](docs/roadmap.md). Live branch and issue state belongs in GitHub according to the [work-tracking contract](docs/work-tracking.md).
