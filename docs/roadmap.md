# RunenUI Roadmap

This document records the current implementation sequence. It is intentionally narrower than the long-term crate map in [Workspace Structure](architecture/workspace-structure.md).

## Current baseline

Implemented:

- host-neutral element, identity, layout-intent, and style-intent vocabulary;
- typed state/action/update application model;
- builder and `element!` authoring;
- runtime tree indexing, input targeting, focus, activation, tracing, and deterministic debug output;
- typed style values, token references, `StyleTokens`, `ComputedStyle`, provenance, and unresolved-token diagnostics;
- unified `SurfacePublication` producing aligned `SurfaceFrame`, `SurfaceStyleReport`, and `SurfaceLayoutReport` products from one preparation pass;
- computed padding applied to text, button, container, and root geometry;
- padded outer bounds used for hit testing;
- removal of the hidden button padding metric;
- accepted [Layout Constraints and Measurement Contract](architecture/layout-constraints-measurement-contract.md);
- normalized finite/unbounded `LayoutConstraints` vocabulary;
- renderer-neutral text measurement requests, results, and provider contract;
- deterministic constraints-aware measurement provider for tests and headless examples;
- authoritative surface publication from explicit root constraints and a borrowed measurement provider;
- provider-backed standalone text and button-label measurement;
- root frame size derived from constrained intrinsic outer size;
- obsolete placeholder surface metrics and duplicate character-count measurement removed;
- one publication-local measured layout result shared by measurement and arrangement;
- finite row/column content-box constraints propagated on the cross axis without stretch;
- intrinsic unbounded main-axis child measurement with deterministic overflow diagnostics;
- exactly one provider call per text or button label during each publication;
- runtime-node-aligned layout diagnostics published with frame and style products.

## Current boundary decision

Keep layout, measurement orchestration, and surface publication in `runenui_runtime`.

The neutral contracts and their first conformance suite now exist, but extraction still is not justified. The implementation still has:

- one intentionally small row/column algorithm;
- no independent layout consumer;
- no Cargo-enforced dependency boundary;
- no completed formal boundary review against the implemented result.

Reconsider extraction only when the criteria in [Layout Boundary Review](architecture/layout-boundary-review.md) are materially satisfied.

## Next implementation sequence

1. Perform the formal layout boundary review using the implemented constraints, measurement, measured result, child propagation, diagnostics, and conformance tests. Do not automatically extract `runenui_layout`.
2. Define the renderer-neutral primitive/frame protocol before implementing WGPU or SDF backends.
3. Add accessibility-tree extraction and a dedicated deterministic testing surface before broad control expansion.
4. Add reusable controls, then a real host contract and first adapter/backend pair.

## Deferred

Not next:

- component recipes and variants;
- interaction-state style layers;
- external theme or source formats;
- layout, style, or render crate extraction without boundary pressure;
- concrete renderer backends;
- docking, hot reload, and the facade crate.
