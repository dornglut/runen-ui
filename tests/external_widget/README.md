# External Widget Conformance Fixture

> **Category: Test fixture**

This non-publishable package is a genuine downstream consumer of public
`runenui_core` and `runenui_runtime` APIs. It defines external state-aware leaf
and child-layout widgets without framework registration, private imports,
feature flags, global registries, source modification, or unsafe code.

The fixture proves that persistent widget state is visible to activation,
measurement, child layout, paint, semantics, and diagnostics; activation can
produce `Activated` without an application action; mapped non-`Clone` actions
remain supported; and lifecycle mount/update/unmount/shutdown order is runtime
owned.

Mounted conformance covers keyed reorder preserving mounted/semantic identity,
state, focus, and interaction slots; stale and foreign target rejection;
postorder cleanup; clean capability-cache reuse; selective paint/layout
invalidation; immediate focus validation; truthful phase reports and isolated
paint/semantics/diagnostics/layout execution; built-in
and external row/column layout; fixed/text/unsupported intrinsic minimums;
measurement and child-layout query counts; nested gaps; hit testing; structured
identity diagnostics; and exact mounted-ID, semantic-ID, and parent alignment
across the mounted index and all publication products. Measurement provider
revision changes execute layout with current provider output.

Focused downstream identity coverage also proves concrete and generic widget
type identity, state type identity, and recursive mapping of non-`Clone`
actions without changing the mounted widget/state pair.

The fixture remains proof-level. It does not claim routed events, pointer
capture/release behavior, effects or scheduling, a production semantic tree,
paint scene, layout engine, host, or renderer backend.
