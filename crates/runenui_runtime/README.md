# `runenui_runtime`

> **Category: Current contract**

`runenui_runtime` owns deterministic headless execution and surface-publication proofs for RunenUI.

It currently binds application state/action/update/root through `UiApp` and `AppRuntime`, assigns preorder IDs for each transient tree, provides basic focus and press-activation policies, resolves style, applies constraints and provider-backed intrinsic measurement, arranges a small row/column layout, and publishes aligned `SurfaceFrame`, `SurfaceStyleReport`, and `SurfaceLayoutReport` products.

The layout contract has explicit finite/unbounded constraints, computed padding, one text/button-label measurement per node per publication, intrinsic main-axis sizing, finite cross-axis limits, and diagnostic-only overflow. The deterministic provider is for tests/headless examples, not production typography.

Important limitations:

- there is no persistent mounted tree, reconciliation, generational identity, lifecycle, local widget state, or granular invalidation;
- dispatch rebuilds the root and clears focus;
- pointer activation occurs on press and there is no pointer capture or event routing;
- input-intent and direct event paths overlap;
- effects, scheduling, semantics/accessibility, production text, native hosts, and renderer backends are absent;
- `SurfaceFrame` contains semantic control kinds and is not the production paint protocol.

The crate must remain independent of application domain state, native window implementations, concrete renderers, ECS ownership, and legacy dependencies.

See the workspace [status](../../docs/status-map.md), [support matrix](../../docs/feature-support-matrix.md), and [roadmap](../../docs/roadmap.md).
