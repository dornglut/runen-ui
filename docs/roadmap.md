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
- accepted [Layout Constraints and Measurement Contract](architecture/layout-constraints-measurement-contract.md);
- normalized finite/unbounded `LayoutConstraints` vocabulary;
- renderer-neutral text measurement requests, results, and provider contract;
- deterministic constraints-aware measurement provider for tests and headless examples.

## Current boundary decision

Keep layout, measurement orchestration, and surface publication in `runenui_runtime`.

The neutral contracts now exist, but extraction still is not justified. The implementation still has:

- one placeholder row/column algorithm;
- a fixed-size public surface entry point;
- duplicate character-count measurement inside surface layout;
- no independent layout consumer;
- no independent layout diagnostics or conformance suite.

Reconsider extraction only when the criteria in [Layout Boundary Review](architecture/layout-boundary-review.md) are materially satisfied.

## Next implementation sequence

1. Integrate `LayoutConstraints` and a borrowed `MeasurementProvider` into surface publication.
2. Migrate text and button-label measurement out of `surface.rs` so the provider is authoritative.
3. Retire duplicate `SurfaceLayoutMetrics` measurement fields while retaining explicit button minimum-size policy.
4. Apply content-box constraints to row/column layout and make overflow behavior explicit.
5. Review the layout boundary again using the resulting dependencies and tests.
6. Define the renderer-neutral primitive/frame protocol before implementing WGPU or SDF backends.
7. Add accessibility-tree extraction and a dedicated deterministic testing surface before broad control expansion.
8. Add reusable controls, then a real host contract and first adapter/backend pair.

## Deferred

Not next:

- component recipes and variants;
- interaction-state style layers;
- external theme or source formats;
- layout, style, or render crate extraction without boundary pressure;
- concrete renderer backends;
- docking, hot reload, and the facade crate.
