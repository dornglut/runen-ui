# Architecture

This directory owns RunenUI's **current durable architecture**: system boundaries, ownership, dependency direction, and conceptual public contracts. Code and executable tests show what behavior currently exists; accepted ADRs and conformance contracts define what that implementation must satisfy. A mismatch is a defect or requires an explicit reviewed contract revision.

## Runtime pipeline

```text
Application state
  -> transient typed View/Element tree
  -> keyed reconciliation
  -> persistent mounted runtime tree
  -> interaction / computed style / layout / semantics
       ├── renderer-neutral logical text measurement/resources
       └── runtime-owned deterministic motion sampling
  -> surface publication
       ├── renderer-facing paint publication/scene products
       ├── hit/input products
       ├── semantic publication
       └── diagnostics
  -> host/platform adapters and renderer backends
```

The mounted tree is the live runtime authority. Authored views/elements are transient inputs and are not retained as a second runtime tree.

Mounted and semantic identity are deliberately distinct runtime-issued lifetimes. Semantic publication is a renderer-independent sibling product, not a renderer field or mounted-ID alias. Input targeting is generation-safe and must not silently retarget through newer publication state.

Application work, routed interaction, transactional edit reconciliation, scheduling, trace, layout/text orchestration, deterministic motion lifecycle/sampling, visual/composition publication, and surface publication converge through runtime-owned sequenced authorities. New subsystems must integrate with those ownership boundaries rather than creating parallel queues, document stores, editor buffers, focus models, semantic trees, text authorities, layout topologies, motion engines, visual/composition trees, or publication caches.

## Current architecture documents

- [Workspace structure](workspace-structure.md) — crate/package ownership, dependency direction, and extraction criteria.
- [Public API contract](public-api.md) — conceptual public ownership/invariants; exact signatures remain in source/Rustdoc.
- [Events, effects, and scheduling](events-effects-and-scheduling.md) — current interaction/work/runtime ownership and invariants.
- [Layout and measurement](layout.md) — runtime-owned production layout orchestration, private Taffy algorithms, bounded widget measurement, and exact text feedback.
- [`runenui_text` package contract](../../crates/runenui_text/README.md) — accepted renderer-neutral production text ownership, artifacts, retained-layout caret/preedit mapping, reuse, and runtime/renderer integration boundary.
- [`runenui_render_wgpu` package contract](../../crates/runenui_render_wgpu/README.md) — accepted concrete renderer ownership, resource/text realization, M9 visual/composition realization, and the boundary that keeps deterministic motion authority out of the renderer while allowing retained sampled-publication retry and cache/device reconstruction.
- [Styling](styling.md) — current accepted production style mechanism, M9 visual property breadth, transition/reduced-motion integration, canonical interaction-driven motion, runtime authority integration, and current limitations.

The [M13 input-arbitration ADR](../adr/0019-event-correlated-host-input-arbitration.md) defines the event-correlated framework/host boundary. The [M13 conformance matrix](../conformance/m13-conformance-matrix.md) independently distinguishes framework implementation proof from native/Engine proof; the current accepted maturity belongs in [status](../status.md).

Durable decisions are recorded separately in [ADRs](../adr/). Permanent observable/proof contracts and directly supporting accepted milestone contract material are under [conformance](../conformance/README.md).

## Current boundary versus target architecture

The retained runtime owns mounted identity, routed processing, editing,
style/text/layout orchestration, motion, and atomic semantic/visual publication.
Renderer and platform edges consume those public products, retaining their
independent realization/translation lifetimes. An accepted target ADR does not
by itself establish implementation or native-platform qualification.
See [status](../status.md) for the sole accepted-capability maturity record,
[conformance](../conformance/README.md) for proof obligations, and the
[roadmap](../roadmap.md) for future dependency sequencing.

Concrete native hosts, accessibility adapters, product state, and engine/ECS integration remain edge or consumer responsibilities. The accepted renderer and platform adapters remain downstream of ordinary public framework contracts; RunenUI core/runtime/text must not acquire native or backend authority merely because later production profiles broaden those edges.
