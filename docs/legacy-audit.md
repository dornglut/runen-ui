# Legacy Audit

This document records the current stance on the `legacy/` archive.

The legacy code is useful historical material, but it is not the active foundation for the new `RunenUI` core.

## Verdict

Keep legacy as a reference archive.

Do not build the new framework by wiring the old crate graph back into the workspace.

## Why Legacy Is Not The Foundation

The legacy archive contains a broad proof-oriented system: definitions, programs, compiler lowering, artifacts, runtime views, app-integration proofs, composition systems, headless rendering, and test fixtures.

That work solved useful problems, but it carries assumptions from the old `Runenwerk` UI track:

- compiler/program/artifact path as a central pipeline
- route-string and schema-payload action dispatch
- proof-local app bridges
- host and ECS assumptions in places that should be host-neutral
- many crates before the public authoring model is stable

The clean `RunenUI` foundation should instead start from:

```text
Element<Action>
  -> Runtime
  -> SurfaceFrame
```

## Worth Keeping As Concepts

Legacy material may be mined for:

- layout edge cases
- input and focus behavior
- hit-testing semantics
- trace and report structure
- accessibility/source-map concepts
- renderer-neutral primitive ideas
- deterministic story/conformance testing
- document/compiler ideas for later editor support

## Not Worth Keeping As Active Core

Legacy material should not be directly reused for:

- the first public `Element<Action>` API
- the first headless runtime loop
- the first counter proof
- typed app action dispatch
- core crate structure
- mandatory compiler/program/artifact runtime path

## Salvage Rule

A legacy concept can be salvaged only by reimplementing it under a clean crate boundary.

The new implementation must use `runenui.*` vocabulary and must remove old `runenwerk.*` assumptions unless they are explicitly still correct for the standalone project.

## Current Status

`legacy/` remains outside the active workspace.

The active skeleton is limited to:

- `runenui_core`
- `runenui_runtime`
- `examples/counter`
