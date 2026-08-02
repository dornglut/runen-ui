# External Widget Conformance Fixture

> **Category: Test fixture**

This non-publishable package is a genuine downstream consumer of public
`runenui_core` and `runenui_runtime` APIs. It defines external state-aware leaf
and child-layout widgets without framework registration, private imports,
feature flags, global registries, source modification, or unsafe code.

Its mounted-work and subscription proofs cover M4B. Its routed semantic-event
and pointer proofs cover the public M4C protocol without private test seams.

The fixture proves that persistent widget state is visible to activation,
measurement, child layout, paint, semantics, and diagnostics; activation can
produce `Activated` without an application action; repeated activations can
queue fresh mapped non-`Clone` actions before pumping; and lifecycle
mount/update/unmount/shutdown order is runtime owned.

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

The downstream widget protocol also exposes complete-set mounted subscription
declarations and owner-local invalidation without registry access. Runtime tests
prove declaration after mount, coalesced compatible-update invalidation, stable
identity retention/replacement, duplicate diagnostics, and owner-lifetime
cancellation through this same public bridge. The fixture also proves activation
invalidation is committed before primary/auxiliary actions, a queued declaration
observes newest live mounted state, and a removed dirty owner suppresses its
declaration callback with structured trace evidence.

M4C3 additionally proves that a downstream mapped widget receives public
pointer, target-only boundary, and target-only capture notifications; observes
physical hit facts separately from captured routing; requests capture through
`EventContext`; and receives exactly one wheel-derived logical-scroll command.

M4C5 additionally proves that the same downstream public widget can opt into
committed text and composition, receive keyboard/committed-text/composition
Capture/Target/Bubble events, observe opaque exact composition generations, and
map resulting actions without private input helpers. Its integration test also
uses only public runtime ingress and verifies that text/preedit payloads do not
appear in the canonical trace. This is a proof-complete branch package pending
independent review and owner acceptance, not editable text or native IME support.

The fixture remains proof-level. It does not claim native host translation, a
production semantic tree, paint scene, layout engine, host, or renderer backend.
