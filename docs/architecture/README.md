# Architecture

This directory owns RunenUI's **current durable architecture**: system boundaries, ownership, dependency direction, and conceptual public contracts. Code and executable tests show what behavior currently exists; accepted ADRs and conformance contracts define what that implementation must satisfy. A mismatch is a defect or requires an explicit reviewed contract revision.

## Runtime pipeline

```text
Application state
  -> transient typed View/Element tree
  -> keyed reconciliation
  -> persistent mounted runtime tree
  -> interaction / computed style / layout / semantics
       └── renderer-neutral logical text measurement/resources
  -> surface publication
       ├── renderer-facing paint publication/scene products
       ├── hit/input products
       ├── semantic publication
       └── diagnostics
  -> host/platform adapters and renderer backends
```

The mounted tree is the live runtime authority. Authored views/elements are transient inputs and are not retained as a second runtime tree.

Mounted and semantic identity are deliberately distinct runtime-issued lifetimes. Semantic publication is a renderer-independent sibling product, not a renderer field or mounted-ID alias. Input targeting is generation-safe and must not silently retarget through newer publication state.

Application work, routed interaction, scheduling, trace, layout/text orchestration, and publication converge through runtime-owned sequenced authorities. New subsystems must integrate with those ownership boundaries rather than creating parallel queues, stores, focus models, semantic trees, text authorities, or publication caches.

## Current architecture documents

- [Workspace structure](workspace-structure.md) — crate/package ownership, dependency direction, and extraction criteria.
- [Public API contract](public-api.md) — conceptual public ownership/invariants; exact signatures remain in source/Rustdoc.
- [Events, effects, and scheduling](events-effects-and-scheduling.md) — current interaction/work/runtime ownership and invariants.
- [Layout and measurement](layout.md) — runtime-owned production layout orchestration, private Taffy algorithms, bounded widget measurement, and exact text feedback.
- [`runenui_text` package contract](../../crates/runenui_text/README.md) — accepted renderer-neutral production text ownership, artifacts, reuse, and runtime/renderer integration boundary.
- [Styling](styling.md) — current accepted production style mechanism, runtime authority integration, and bounded property limitations.

Durable decisions are recorded separately in [ADRs](../adr/). Permanent observable/proof contracts and directly supporting accepted milestone contract material are under [conformance](../conformance/README.md).

## Current boundary versus target architecture

RunenUI provides a deterministic headless framework foundation with the complete accepted M6 renderer-neutral paint/hit protocol, the complete accepted M7 reference production spine at proof maturity, the accepted M8A production style mechanism at partial styling maturity, and the accepted M8B production logical-text/SDF-MSDF mechanism at partial text maturity. M8B adds explicit deterministic/production font-source policy, international shaping and line breaking behind RunenUI-owned contracts, immutable logical artifacts shared by measurement and paint, retained scale-independent shaped-resource bindings, and renderer-owned per-glyph SDF/MSDF realization without shaped-text provider or alpha fallback. Runtime layout orchestration now uses private low-level Taffy Block/Flex/Grid algorithms over the mounted topology, with bounded custom measurement and exact available-space text feedback; M8D owns integrated responsive/text-heavy closure. See [status](../status.md) for accepted maturity and the [roadmap](../roadmap.md) for durable sequencing.

Concrete native hosts, accessibility adapters, product state, and engine/ECS integration remain edge or consumer responsibilities. The accepted renderer and platform adapters remain downstream of ordinary public framework contracts; RunenUI core/runtime/text must not acquire native or backend authority merely because later production profiles broaden those edges.
