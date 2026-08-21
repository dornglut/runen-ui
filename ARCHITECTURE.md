# RunenUI Architecture

RunenUI is a host-neutral, renderer-neutral Rust UI framework. The durable ownership direction is:

```text
application state and actions
    -> transient View/Element authoring
    -> keyed reconciliation
    -> persistent mounted runtime tree
    -> interaction, style, layout, and semantics
    -> hit-test and paint products
    -> host integration and renderer backend
```

The authored tree is transient reconciliation input. The mounted runtime tree is the persistent authority for runtime identity, widget-local state, lifecycle, invalidation, focus, interaction state, work ownership, and publication coordination.

Semantic identity is independently runtime-issued and published through a renderer-independent semantic product. It is not a mounted-arena alias and must not be folded into renderer scene authority. Surface publication is staged and atomic: rejected or terminally failed publication must not expose a partial new RunenUI-owned product.

## Workspace ownership

- `runenui_core` — host-neutral public application, authoring, geometry, style, event, effect, identity, and semantic protocol values.
- `runenui_runtime` — live mounted/semantic storage, reconciliation, routing, focus/input state, scheduling, tracing, publication, and shutdown.
- `runenui_testing` — downstream deterministic testing ergonomics over ordinary public core/runtime contracts.
- concrete hosts, platform adapters, renderer backends, and product state remain outside those ownership boundaries until their roadmap milestones justify real implementations.

The workspace dependency and extraction rules are defined in [workspace structure](docs/architecture/workspace-structure.md).

## Current and target architecture

Current accepted behavior is established by code/tests and summarized in [current status](docs/status.md). Detailed current architecture is indexed under [docs/architecture](docs/architecture/README.md).

Accepted future architecture is introduced only through its owning ADR/design/conformance authority. In particular, the renderer-neutral paint/hit scene contract is accepted target architecture, but target vocabulary is not current Rust API until implemented and accepted.

Durable decisions live in [ADRs](docs/adr/). Permanent observable/proof contracts live under [conformance](docs/conformance/README.md). High-level dependency sequence lives in the [roadmap](docs/roadmap.md). Exact public Rust signatures remain authoritative in source/Rustdoc; conceptual public ownership is summarized in the [public API contract](docs/architecture/public-api.md).

Live issue, branch, pull-request, head, CI-run, blocker, and pickup state belongs in GitHub and is deliberately absent from this document.
