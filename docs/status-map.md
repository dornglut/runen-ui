# Status Map

This document tracks the implementation maturity of the clean RunenUI workspace.

Status values:

| Status | Meaning |
|---|---|
| `skeleton` | crate or example exists only to reserve the boundary |
| `draft` | public shape exists but is still unstable |
| `proof` | behavior is covered by tests for a narrow case |
| `usable` | suitable for internal examples |
| `stable` | public API is intentionally supported |

## Current Status

| Area | Status | Current purpose | Next step |
|---|---|---|---|
| `runenui_core` | skeleton | Host-neutral typed UI description crate | Replace marker `Element<Action>` with real element tree, IDs, layout intent, and typed press actions |
| `runenui_runtime` | skeleton | Headless runtime crate boundary | Add typed input/action dispatch, update execution, trace, and first surface-frame model |
| `examples/counter` | skeleton | First public architecture proof | Implement builder-authored counter screen, win screen, reset flow, and headless runtime test |
| `legacy/` | archived reference | Historical Runenwerk UI experiments and proofs | Mine for concepts/tests only; do not import directly |

## First Milestone

The first real milestone is not a renderer.

The first milestone is:

```text
A typed Rust counter screen can be authored as Element<CounterAction>,
executed through a headless runtime, updated through app-owned state,
switched to a win screen at the threshold, reset, traced, and published
as renderer-neutral surface-frame data.
```

## Non-Goals For Current Skeleton

The current skeleton does not implement:

* layout solving
* rendering
* windowing
* accessibility output
* effects
* async tasks
* text input
* visual editor support
* RON or external document compilation
* SDF renderer integration
* legacy compiler/program/artifact integration

## Merge Criteria For Skeleton PR

The skeleton PR is acceptable when:

* generated Cargo template code has been removed
* crate boundaries are documented
* dependency direction is correct
* no legacy crates are active dependencies
* `cargo fmt --all` passes
* `cargo test --workspace` passes
* `cargo clippy --workspace --all-targets -- -D warnings` passes
