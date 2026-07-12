# `runenui_runtime`

> **Category: Current contract**

`runenui_runtime` owns deterministic headless execution and surface-publication
proofs for RunenUI.

It binds application state/action/update/root through `UiApp` and `AppRuntime`,
assigns opaque preorder IDs, validates authored tree identity, provides basic
focus and press activation, resolves style, applies validated constraints and
provider-backed intrinsic measurement, arranges the small row/column layout, and
publishes aligned frame/style/layout products. Duplicate IDs and sibling keys
produce true numeric-preorder diagnostics with deterministic same-node category
ordering; ambiguous ID activation never chooses the first match. Derived layout
and rectangle-edge arithmetic saturates instead of publishing NaN or infinity.

Runtime IDs, indexes, focus/trace state, frames, frame nodes, and style/layout
reports are generated read-only products with no normal public forgery
constructors. Legitimate products remain inspectable through public accessors and
deterministic debug formatters. The runtime prelude is intentionally limited to
`AppRuntime`, `UiApp`, `LogicalSize`, and `SurfaceBuildContext`.

There is still no persistent mounted tree, reconciliation, generational identity,
lifecycle, local widget state, granular invalidation, correct release-based
activation, effects, semantics/accessibility, production text, native host, or
renderer backend. `Action: Clone` is restricted to activation paths that duplicate
an action retained by the current immutable authored tree; mount and direct
dispatch accept non-`Clone` actions.

See the [M1 public API contract](../../docs/architecture/public-api.md), workspace
[status](../../docs/status-map.md), [support matrix](../../docs/feature-support-matrix.md),
and [roadmap](../../docs/roadmap.md).
