# External Widget Conformance Fixture

> **Category: Guide**

This non-publishable package is a genuine downstream consumer of public
`runenui_core` and `runenui_runtime` APIs. Its tests define external state-aware
leaf and child-layout widgets without framework registration, private imports,
production feature flags, global registries, source modification, or unsafe code.
A test-only dev dependency on `runenui_testing` is permitted for public-harness
conformance; no production framework dependency points from runtime to testing.

## What it proves

The fixture covers accepted downstream behavior across M2–M5:

- external widget/state identity, lifecycle, persistent state, measurement, child
  layout, paint facts, canonical semantic contribution, diagnostics, and recursive
  non-`Clone` action mapping through the same public protocol as built-ins;
- keyed reorder retention, replacement/removal stale identity, focus/interaction
  state, capability invalidation, aligned renderer-facing publication products,
  and deterministic layout/hit-test proof behavior;
- mounted subscription declarations, owner-local invalidation, lifecycle-owned
  work/cancellation, and accepted scheduler/output ordering;
- public pointer, boundary/capture, focus, keyboard, committed-text, composition,
  authored-ID automation, routed command/default, canonical trace/export/replay,
  and exact displayed-surface ingress behavior;
- M5 semantic contribution with independent `SemanticNodeId` lifetimes,
  renderer-independent semantic publication, exact surface-scoped semantic action
  ingress, and private semantic-to-mounted resolution without exposing a mounted
  routing shortcut;
- M5D public `TestHarness` use through ordinary public core/runtime APIs only.

The M5E closure proof adds a mapped downstream widget that contributes semantics,
publishes through the public harness, receives an exact semantic action, updates
parent application state through recursive action mapping, and leaves canonical
trace/replay evidence. It is intentionally a genuine downstream package rather
than a framework-internal fixture.

## Boundaries

This package does not own production framework behavior, native host/accessibility
integration, renderer backends, production layout/text/controls, semantic-to-
`MountedNodeId` routing authority, hidden mutation hooks, fabricated runtime
identities/sequences, a parallel semantic queue/default engine, or compatibility
aliases for retired M2/M5 APIs.

See the [workspace structure](../../docs/architecture/workspace-structure.md),
[M5 charter](../../docs/architecture/m5-semantics-and-testing-charter.md),
[M5 conformance matrix](../../docs/architecture/m5-conformance-matrix.md), and
[testing guide](../../TESTING.md) for current ownership and acceptance rules.
