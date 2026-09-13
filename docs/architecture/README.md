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

Application work, routed interaction, scheduling, trace, layout/text orchestration, visual/composition publication, and surface publication converge through runtime-owned sequenced authorities. New subsystems must integrate with those ownership boundaries rather than creating parallel queues, stores, focus models, semantic trees, text authorities, layout topologies, visual/composition trees, or publication caches.

## Current architecture documents

- [Workspace structure](workspace-structure.md) — crate/package ownership, dependency direction, and extraction criteria.
- [Public API contract](public-api.md) — conceptual public ownership/invariants; exact signatures remain in source/Rustdoc.
- [Events, effects, and scheduling](events-effects-and-scheduling.md) — current interaction/work/runtime ownership and invariants.
- [Layout and measurement](layout.md) — runtime-owned production layout orchestration, private Taffy algorithms, bounded widget measurement, and exact text feedback.
- [`runenui_text` package contract](../../crates/runenui_text/README.md) — accepted renderer-neutral production text ownership, artifacts, reuse, and runtime/renderer integration boundary.
- [`runenui_render_wgpu` package contract](../../crates/runenui_render_wgpu/README.md) — accepted concrete renderer ownership, resource/text realization, and M9A disposable visual/composition realization boundary.
- [Styling](styling.md) — current accepted production style mechanism, M9A visual property breadth, runtime authority integration, and remaining motion limitations.

Durable decisions are recorded separately in [ADRs](../adr/). Permanent observable/proof contracts and directly supporting accepted milestone contract material are under [conformance](../conformance/README.md).

## Current boundary versus target architecture

RunenUI provides a deterministic headless framework foundation with the complete accepted M6 renderer-neutral paint/hit protocol, the complete accepted M7 reference production spine at proof maturity, the complete accepted M8 production style/layout/international-text foundation, and the accepted M9A static visual/composition foundation. M9A adds RunenUI-owned structural rectangle/rounded-rectangle/ellipse/path geometry, generic fill/stroke and brush semantics, image fit/crop/nine-slice mapping, common outline/shadow/opacity/static-presentation style facts, runtime-owned composition/effect publication, conservative effect bounds and alpha-independent ordinary-shadow support, with private disposable wgpu realization including Lyon tessellation and bounded mask work. Those additions preserve inherited M6 ordering/resource/publication contracts and M8 style/text ownership rather than creating a renderer-owned visual model. M9B still owns deterministic transitions/timelines/reduced-motion behavior and M9C owns integrated visual-motion closure; M9 therefore remains partial. See [status](../status.md) for accepted maturity and the [roadmap](../roadmap.md) for durable sequencing.

Concrete native hosts, accessibility adapters, product state, and engine/ECS integration remain edge or consumer responsibilities. The accepted renderer and platform adapters remain downstream of ordinary public framework contracts; RunenUI core/runtime/text must not acquire native or backend authority merely because later production profiles broaden those edges.
