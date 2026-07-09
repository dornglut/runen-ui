# Cutover Plan

This document defines how `RunenUI` moves from the legacy archive toward a clean standalone framework.

The cutover strategy is a clean rebuild under explicit crate boundaries. Legacy code may inform design, tests, and terminology audits, but it must not become the active implementation by default.

## Cutover Principle

```text
Rebuild the core model cleanly.
Mine legacy for concepts and tests.
Do not port the legacy crate graph.
```

## Active Foundation

The active foundation is:

```text
runenui_core
  -> typed Element<Action> model

runenui_runtime
  -> headless runtime, update dispatch, trace, and surface-frame publication

examples/counter
  -> first public typed UI proof
```

## Legacy Archive Rule

The `legacy/` directory is reference material only.

No legacy crate may become a workspace member unless a focused cutover design explains:

- what concept is being salvaged
- which new crate owns it
- which old assumptions are being removed
- which tests preserve the useful behavior
- why reimplementation is safer than direct reuse

## First Cutover Milestone

The first milestone is a headless counter proof:

```text
Counter state
  -> CounterAction
  -> update(&mut Counter, CounterAction)
  -> root(&Counter) -> Element<CounterAction>
  -> Runtime
  -> trace
  -> SurfaceFrame
```

The proof must include:

- counter screen
- win screen at the configured threshold
- reset back to counter screen
- typed button actions
- no route-string action bridge
- no schema-payload action dispatch
- no compiler/program/artifact dependency
- no renderer backend requirement

## Later Cutovers

After the headless counter proof, legacy concepts may be revisited in this order:

1. element identity and retained runtime tree
2. layout behavior and computed layout boxes
3. renderer-neutral primitive output
4. input, hit testing, focus, and pointer capture
5. accessibility tree extraction
6. testing/story harness
7. optional document/compiler layer
8. optional macro authoring layer
9. renderer and host adapters

## Rejection Rule

A proposed cutover is rejected if it imports legacy complexity before the clean core requires it.
