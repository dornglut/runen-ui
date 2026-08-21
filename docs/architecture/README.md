# Architecture

This directory owns RunenUI's **current durable architecture**: system boundaries, ownership, dependency direction, and conceptual public contracts. Current implementation behavior remains authoritative in code and executable tests; accepted target-only behavior belongs in ADR/design/conformance authority until implemented.

## Runtime pipeline

```text
Application state
  -> transient typed View/Element tree
  -> keyed reconciliation
  -> persistent mounted runtime tree
  -> interaction / computed style / layout / semantics
  -> surface publication
       ├── renderer-facing proof/scene products
       ├── hit/input products
       ├── semantic publication
       └── diagnostics
  -> host/platform adapters and renderer backends
```

The mounted tree is the live runtime authority. Authored views/elements are transient inputs and are not retained as a second runtime tree.

Mounted and semantic identity are deliberately distinct runtime-issued lifetimes. Semantic publication is a renderer-independent sibling product, not a renderer field or mounted-ID alias. Input targeting is generation-safe and must not silently retarget through newer publication state.

Application work, routed interaction, scheduling, trace, and publication converge through one runtime-owned sequenced processing authority. New subsystems must integrate with that ownership rather than creating parallel queues, stores, focus models, semantic trees, or publication caches.

## Current architecture documents

- [Workspace structure](workspace-structure.md) — crate/package ownership, dependency direction, and extraction criteria.
- [Public API contract](public-api.md) — conceptual public ownership/invariants; exact signatures remain in source/Rustdoc.
- [Events, effects, and scheduling](events-effects-and-scheduling.md) — current interaction/work architecture and durable accepted constraints.
- [Layout](layout.md) — current proof-level layout/measurement ownership and target relationship.
- [Styling](styling.md) — current style/token ownership and target relationship.

Durable decisions are recorded separately in [ADRs](../adr/). Permanent observable/proof contracts and accepted milestone contract context are under [conformance](../conformance/README.md).

## Current boundary versus target architecture

RunenUI currently provides a deterministic headless framework foundation with renderer-facing proof products, independent semantic publication, and public deterministic testing. The accepted renderer-neutral paint/hit scene design is successor target authority, not current scene API. See [status](../status.md) for current maturity and the [roadmap](../roadmap.md) for durable sequencing.

Concrete native hosts, accessibility adapters, renderer backends, product state, and engine/ECS integration remain edge or consumer responsibilities. RunenUI core/runtime must not acquire those dependencies merely because future production profiles need adapters for them.
