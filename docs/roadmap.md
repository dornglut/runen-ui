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
- removal of the hidden `button_horizontal_padding` metric.

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

1. Define an explicit layout constraints and intrinsic-measurement contract.
2. Introduce a real text/control measurement seam without choosing a renderer backend.
3. Apply constraints to the existing row/column algorithm and add min/max sizing tests.
4. Review the layout boundary again using the resulting dependencies and tests.
5. Define the renderer-neutral primitive/frame protocol before implementing WGPU or SDF backends.
6. Add accessibility-tree extraction and a dedicated deterministic testing surface before broad control expansion.
7. Add reusable controls, then a real host contract and first adapter/backend pair.

## Deferred

Not next:

- component recipes and variants;
- interaction-state style layers;
- external theme or source formats;
- layout, style, or render crate extraction without boundary pressure;
- concrete renderer backends;
- docking, hot reload, and the facade crate.
