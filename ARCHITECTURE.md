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

M0–M5 are complete. M5E's reviewed feature head `7f3e0c9e881ff384516459db66436e662c5fb790` passed exact-head CI #1294 / `32130312467`, was repository-owner-authorized and guarded-squash-merged as `b07ae423d6a3573a4dd8a96a7ce5d6b5b1f0be1e`, and shares exact tree `c5dc7fa000496d76c35e98f3a481fc1de5762f4c` with that squash. Accepted-main CI #1296 / `32135074552` then validated the exact squash through read-only PR #68, which was closed unmerged. The final M5 authority reconciliation records all five M5E rows as owner-accepted and establishes the exact accepted base from which M6 may begin. M6 owns the renderer-neutral paint/hit scene protocol and must first satisfy its roadmap architecture gate; issue #59 is M6-readiness work, not permission to bypass that gate.

Current implementation maturity belongs in the [status map](docs/status-map.md). Delivery sequence belongs in the [roadmap](docs/roadmap.md). Live branch and issue state belongs in GitHub according to the [work-tracking contract](docs/work-tracking.md).