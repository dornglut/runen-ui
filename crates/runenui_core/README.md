# `runenui_core`

> **Category: Current contract**

`runenui_core` is the host- and renderer-neutral authored-data crate in the current RunenUI headless proof.

It currently owns `Element<Action>` and the closed text/button/container descriptors, authored IDs and stored keys, row/column layout intent, style values and typed token references, in-memory token resolution, concrete computed style, provenance, diagnostics, and builder/`element!` authoring.

Important limitations:

- `ElementKind` is closed; external widgets do not yet have a supported protocol.
- keys are stored but do not preserve runtime identity.
- IDs/token IDs accept empty values and duplicates are not diagnosed.
- generic element methods can silently no-op for the wrong kind.
- tuple child conversion stops at eight entries.
- numeric invariants and public API stability are not complete.

The crate does not own mounted runtime state, input routing, effects, layout execution, semantics, paint scenes, renderer backends, native hosts, application state, ECS, or legacy dependencies.

See the workspace [status](../../docs/status-map.md), [support matrix](../../docs/feature-support-matrix.md), and [roadmap](../../docs/roadmap.md).
