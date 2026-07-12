# `runenui_runtime`

> **Category: Current contract**

`runenui_runtime` owns deterministic headless execution and surface-publication
proofs for RunenUI.

It binds application state/action/update/root through `UiApp` and `AppRuntime`,
assigns opaque preorder IDs, validates authored tree identity, provides basic
open-widget focus and press activation with explicit mutable non-`Clone` action
extraction followed by immediate successful-dispatch rebuild, resolves style, applies validated
constraints and provider-backed intrinsic measurement, arranges the small
row/column layout, and publishes aligned frame/style/layout products with open
paint/semantic/diagnostic proof facts. Duplicate IDs and sibling keys
produce true numeric-preorder diagnostics with deterministic same-node category
ordering; ambiguous ID activation never chooses the first match. Derived layout
and rectangle-edge arithmetic saturates instead of publishing NaN or infinity.
Each intrinsic measurement and child-layout capability is snapshotted exactly
once per node/publication and reused during arrangement. Intrinsic minimums are
combined component-wise with child content. Unsupported intrinsic measurement
and unknown cross-version capabilities publish ordered deterministic layout
diagnostics without hiding descendants; unknown child layout falls back to a
vertical line. Index, frame, style, and layout products remain preorder/parent
aligned. Generic control text uses `ControlLabel`.

Runtime IDs, indexes, focus/trace state, frames, frame nodes, and style/layout
reports are generated read-only products with no normal public forgery
constructors. Legitimate products remain inspectable through public accessors and
deterministic debug formatters. The runtime prelude is intentionally limited to
`AppRuntime`, `UiApp`, `LogicalSize`, and `SurfaceBuildContext`.

There is still no persistent mounted tree, reconciliation, generational identity,
state-aware mounted capability interface,
production lifecycle execution, retained local widget state, granular invalidation, correct release-based
activation, effects, semantics/accessibility, production text, native host, or
renderer backend. `AppRuntime` does not retain the M2 widget state or run
lifecycle across rebuilds. Activation can move a non-`Clone` action from the
transient view before the immediate rebuild.

See the [public API contract](../../docs/architecture/public-api.md), workspace
[status](../../docs/status-map.md), [support matrix](../../docs/feature-support-matrix.md),
and [roadmap](../../docs/roadmap.md).
