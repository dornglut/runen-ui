# runenui_testing

> **Category: Current contract**

`runenui_testing` provides deterministic headless testing ergonomics for ordinary RunenUI applications. It is intentionally a downstream consumer of the public `runenui_core` and `runenui_runtime` APIs rather than a privileged runtime test seam.

## What it owns

- `TestHarness<App>` around one ordinary public `AppRuntime<App>`;
- deterministic fixed-surface publication configuration;
- explicit bounded `pump` and finite `run_until_idle` execution;
- manually advanced logical time through the public `ManualClock` contract;
- exact snapshot-scoped semantic queries and unique semantic targets;
- helpers that delegate to ordinary pointer, keyboard, text, composition, automation, command, action, and semantic-action ingress;
- read-only state, focus, reconciliation, surface, semantic, trace, and replay observation.

## Boundary

The crate owns testing ergonomics only. It does not own runtime behavior, mounted or semantic storage, identity/sequence allocation, callback injection, private mutation hooks, native host behavior, or a parallel expected-state model. It must not depend on `internal-test-seams` or recover a `MountedNodeId` from a semantic target.

Semantic actions are deliberately surface-scoped: test targets are produced from an exact committed `SemanticSnapshot` and preserve both its `SurfaceId` and the exact `SemanticNodeId`. Ambiguous semantic queries return every deterministic match instead of selecting a first or last node.

Settling is always explicitly bounded. A settle attempt reports `Idle` only after a complete zero-progress pump iteration; dormant future timers, redraw debt, and externally pending work do not cause hidden waits, while self-requeue remains capped by the caller's finite iteration budget.

M5D is accepted and reconciled. M5E #51 is the active integration/migration/closure slice; this crate acquires no new runtime authority in M5E.

See the repository [workspace structure](../../docs/architecture/workspace-structure.md) and [M5 semantics/testing charter](../../docs/architecture/m5-semantics-and-testing-charter.md) for the ownership and milestone contract.
