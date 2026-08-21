# External Widget Conformance Fixture

> **Category: Guide**

This non-publishable package is a genuine downstream consumer of public `runenui_core` and `runenui_runtime` APIs. Its tests define external state-aware leaf and child-layout widgets without framework registration, private imports, production feature flags, global registries, source modification, or unsafe code. A test-only dev dependency on `runenui_testing` is permitted for public-harness conformance; no production framework dependency points from runtime to testing.

## What it proves

The fixture covers accepted downstream behavior across the current framework foundation:

- external widget/state identity, lifecycle, persistent state, measurement, child layout, paint proof facts, canonical semantic contribution, diagnostics, and recursive non-`Clone` action mapping through the same public protocol as built-ins;
- keyed reorder retention, replacement/removal stale identity, focus/interaction state, capability invalidation, aligned publication products, and deterministic layout/hit-test proof behavior;
- mounted subscription declarations, owner-local invalidation, lifecycle-owned work/cancellation, and scheduler/output ordering;
- public pointer, focus, keyboard, committed-text, composition, authored-ID automation, routed command/default, canonical trace/export/replay, and exact displayed-surface ingress behavior;
- semantic contribution with independent semantic lifetimes, renderer-independent semantic publication, exact surface-scoped semantic action ingress, and private semantic-to-mounted resolution without a public mounted routing shortcut;
- public deterministic `TestHarness` use through ordinary public core/runtime APIs only.

The integrated downstream proof contributes semantics from a mapped external widget, publishes through the public harness, submits an exact semantic action, updates parent application state through recursive action mapping, and leaves canonical trace/replay evidence. It remains a genuine downstream package rather than a framework-internal fixture.

## Boundaries

This package does not own production framework behavior, native host/accessibility integration, renderer backends, production layout/text/controls, semantic-to-mounted routing authority, hidden mutation hooks, fabricated runtime identities/sequences, a parallel semantic queue/default engine, or compatibility aliases for retired APIs.

See [workspace structure](../../docs/architecture/workspace-structure.md), the [M5 charter](../../docs/conformance/m5-semantics-and-testing-charter.md), the [M5 conformance matrix](../../docs/conformance/m5-conformance-matrix.md), and [testing](../../TESTING.md).
