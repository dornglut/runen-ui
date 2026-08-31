# Architecture

This directory owns RunenUI's **current durable architecture**: system boundaries, ownership, dependency direction, and conceptual public contracts. Code and executable tests show what behavior currently exists; accepted ADRs and conformance contracts define what that implementation must satisfy. A mismatch is a defect or requires an explicit reviewed contract revision.

## Runtime pipeline

```text
Application state
  -> transient typed View/Element tree
  -> keyed reconciliation
  -> persistent mounted runtime tree
  -> interaction / computed style / layout / semantics
  -> surface publication
       ├── renderer-facing paint publication/scene products
       ├── hit/input products
       ├── semantic publication
       └── diagnostics
  -> host/platform adapters and renderer backends
```

The mounted tree is the live runtime authority. Authored views/elements are transient inputs and are not retained as a second runtime tree.

Mounted and semantic identity are deliberately distinct runtime-issued lifetimes. Semantic publication is a renderer-independent sibling product, not a renderer field or mounted-ID alias. Input targeting is generation-safe and must not silently retarget through newer publication state.

Application work, routed interaction, scheduling, trace, and publication converge through runtime-owned sequenced authorities. New subsystems must integrate with those ownership boundaries rather than creating parallel queues, stores, focus models, semantic trees, or publication caches.

## Current architecture documents

- [Workspace structure](workspace-structure.md) — crate/package ownership, dependency direction, and extraction criteria.
- [Public API contract](public-api.md) — conceptual public ownership/invariants; exact signatures remain in source/Rustdoc.
- [Events, effects, and scheduling](events-effects-and-scheduling.md) — current interaction/work/runtime ownership and invariants.
- [Layout](layout.md) — current proof-level layout/measurement ownership and limitations.
- [Styling](styling.md) — current accepted production style mechanism, runtime authority integration, and bounded property limitations.

Durable decisions are recorded separately in [ADRs](../adr/). Permanent observable/proof contracts and directly supporting accepted milestone contract material are under [conformance](../conformance/README.md).

## Current boundary versus target architecture

RunenUI currently provides a deterministic headless framework foundation with the complete accepted M6 renderer-neutral paint/hit protocol, the complete accepted M7 reference production spine at proof maturity, and the accepted M8A production style mechanism at partial styling maturity. M8A adds one host-neutral style environment with typed themes/recipes/ordered variants, deterministic property-local precedence, exact provenance/diagnostics, canonical hover/focus/active/disabled projection from runtime authority, explicit preference policy, bounded foreground inheritance, and effect-driven retained invalidation over the current foreground/background/padding/radius property set. M8B logical text plus SDF/MSDF realization is the next target slice; M8C production runtime layout and M8D integrated closure remain later M8 work. See [status](../status.md) for current maturity and the [roadmap](../roadmap.md) for durable sequencing.

Concrete native hosts, accessibility adapters, product state, and engine/ECS integration remain edge or consumer responsibilities. The accepted renderer and platform adapters remain downstream of ordinary public core/runtime contracts; RunenUI core/runtime must not acquire native or backend dependencies merely because later production profiles broaden those edges.
