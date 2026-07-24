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

The mounted tree owns runtime identity, widget state, lifecycle, invalidation, focus, pointer interaction, task/subscription ownership, and publication authority. The authored tree is transient input and must not become a parallel retained runtime.

Crate and subsystem boundaries, current proof behavior, target products, and accepted constraints are documented in:

- [Detailed architecture](docs/architecture.md)
- [Workspace structure](docs/architecture/workspace-structure.md)
- [Public API](docs/architecture/public-api.md)
- [M4 delivery charter](docs/architecture/m4c-delivery-and-routed-transaction-charter.md)
- [M4 conformance matrix](docs/architecture/m4-conformance-matrix.md)
- [Architecture decision records](docs/adr/)

Current implementation maturity belongs in the [status map](docs/status-map.md). Delivery sequence belongs in the [roadmap](docs/roadmap.md). Live branch and issue state belongs in GitHub according to the [work-tracking contract](docs/work-tracking.md).
