# RunenUI Roadmap

This document records the current implementation sequence. It is intentionally narrower than the long-term crate map in [Workspace Structure](architecture/workspace-structure.md).

## Current baseline

Implemented:

- host-neutral element, identity, layout-intent, and style-intent vocabulary;
- typed state/action/update application model;
- builder and `element!` authoring;
- runtime tree indexing, input targeting, focus, activation, tracing, and deterministic debug output;
- typed style values, token references, `StyleTokens`, `ComputedStyle`, provenance, and unresolved-token diagnostics;
- unified `SurfacePublication` producing an aligned `SurfaceFrame` and `SurfaceStyleReport` from one style-resolution pass;
- computed padding applied to text, button, container, and root geometry;
- padded outer bounds used for hit testing;
- removal of the hidden `button_horizontal_padding` metric;
- accepted [Layout Constraints and Measurement Contract](architecture/layout-constraints-measurement-contract.md).

## Current boundary decision

Keep layout and surface publication in `runenui_runtime`.

Computed padding is the first style-driven geometry rule, but it does not yet justify `runenui_layout`. The implementation still has:

- one placeholder row/column algorithm;
- only root-size input rather than explicit constraints;
- character-count text measurement;
- no independent layout consumer;
- no independent layout diagnostics or conformance suite.

Reconsider extraction only when the criteria in [Layout Boundary Review](architecture/layout-boundary-review.md) are materially satisfied.

## Next implementation sequence

1. Implement normalized `LayoutConstraints` and finite/unbounded axis bounds.
2. Route the existing tight root-size path through the constraint vocabulary without adding a parallel layout algorithm.
3. Introduce a renderer-neutral text measurement request/response seam and deterministic fallback provider.
4. Migrate text and button label measurement out of layout internals.
5. Apply content-box constraints to row/column layout and make overflow behavior explicit.
6. Review the layout boundary again using the resulting dependencies and tests.
7. Define the renderer-neutral primitive/frame protocol before implementing WGPU or SDF backends.
8. Add accessibility-tree extraction and a dedicated deterministic testing surface before broad control expansion.
9. Add reusable controls, then a real host contract and first adapter/backend pair.

## Deferred

Not next:

- component recipes and variants;
- interaction-state style layers;
- external theme or source formats;
- layout, style, or render crate extraction without boundary pressure;
- concrete renderer backends;
- docking, hot reload, and the facade crate.
