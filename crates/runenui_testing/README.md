# `runenui_testing`

> **Category: Current contract**

`runenui_testing` provides deterministic headless testing ergonomics for ordinary RunenUI applications. It is intentionally a downstream consumer of the public `runenui_core` and `runenui_runtime` APIs rather than a privileged runtime test seam.

## What it owns

- `TestHarness<App>` around one ordinary public runtime;
- deterministic fixed-surface publication configuration;
- explicit bounded pumping and finite settling;
- manually advanced logical time through the public manual-clock contract;
- exact snapshot-scoped semantic queries and unique semantic targets;
- helpers that delegate to ordinary public pointer, keyboard, text, composition, automation, command, action, and semantic-action ingress;
- read-only state, focus, reconciliation, surface, semantic, trace, and replay observation.

## Boundary

The crate owns testing ergonomics only. It does not own runtime behavior, mounted or semantic storage, identity/sequence allocation, callback injection, private mutation hooks, native host behavior, or a parallel expected-state model. It must not depend on internal test seams or recover a mounted runtime identity from a semantic target.

Semantic actions remain surface-scoped: test targets are produced from an exact committed semantic snapshot and preserve both surface and semantic identity. Ambiguous semantic queries return every deterministic match instead of selecting a first or last node.

Settling is explicitly bounded. Idle is reported only after a complete zero-progress pump iteration; dormant future timers, redraw/publication debt, and externally pending work do not cause hidden waits, while self-requeue remains capped by caller-supplied budgets.

Harness construction delegates to fallible runtime admission. `TestHarness::mount`
and `mount_with_config` return `HarnessMountError` on refusal. Its
`admission_error()` exposes the runtime reason and owned application state/config;
`into_parts()` also returns the supplied surface configuration for an explicit
corrected retry. Constructor cleanup does not roll back callbacks that already
ran. Refusal creates no partially usable harness and adds no `Clone` or `Debug`
requirement to application state.

Future scene/testing capabilities must continue to arrive through public runtime/publication contracts rather than granting this crate new live authority.

See [workspace structure](../../docs/architecture/workspace-structure.md) and the [M5 semantics/testing charter](../../docs/conformance/m5-semantics-and-testing-charter.md).
