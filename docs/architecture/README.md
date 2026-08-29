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
- [Styling](styling.md) — current style/token ownership and limitations.

Durable decisions are recorded separately in [ADRs](../adr/). Permanent observable/proof contracts and directly supporting accepted milestone contract material are under [conformance](../conformance/README.md).

## Current boundary versus target architecture

RunenUI currently provides a deterministic headless framework foundation with the complete accepted M6 renderer-neutral paint/hit protocol and the complete accepted M7 reference production spine at proof maturity: retained publication, canonical immutable paint/hit products, composition/resource-reference/renderer-metadata/capability semantics, independent deterministic consumer proof, public deterministic testing, the reusable wgpu renderer/resource edge with real offscreen/readback and golden evidence, reusable winit translation and AccessKit projection/action translation, standalone native host and Counter application presentation paths, and a winit-free downstream host-owned frame-loop proof over the same public runtime/publication/renderer/resource contracts. M8 production style, layout, and international-text foundations are the next target architecture. See [status](../status.md) for current maturity and the [roadmap](../roadmap.md) for durable sequencing.

Concrete native hosts, accessibility adapters, product state, and engine/ECS integration remain edge or consumer responsibilities. The accepted renderer and platform adapters remain downstream of ordinary public core/runtime contracts; RunenUI core/runtime must not acquire native or backend dependencies merely because later production profiles broaden those edges.
