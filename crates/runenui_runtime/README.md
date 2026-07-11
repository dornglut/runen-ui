# runenui_runtime

`runenui_runtime` owns the headless UI runtime for RunenUI.

This crate turns application state and typed UI descriptions into runtime behavior and published surface products. Applications own their state and actions. Elements emit actions. The runtime dispatches input, resolves interaction, calls the application update function, rebuilds the root element, resolves per-node style, computes layout, and publishes an aligned renderer-facing `SurfaceFrame` and diagnostic `SurfaceStyleReport`.

## Responsibilities

`runenui_runtime` may own:

* input event normalization
* hit testing
* focus and pointer capture
* typed action dispatch
* calling `update(&mut State, Action)`
* rebuilding `root(&State) -> Element<Action>`
* layout orchestration
* accessibility tree extraction
* primitive extraction
* runtime tracing
* unified surface publication from explicit style tokens and layout metrics
* deterministic headless execution for tests

## Non-responsibilities

`runenui_runtime` must not own:

* application state definitions
* application action enums
* renderer backend implementation
* windowing or platform event loops
* ECS host ownership
* visual editor document formats
* compiler/program/artifact pipelines as the mandatory runtime path
* legacy crate dependencies

For workspace-wide dependency rules, see [dependency-map](../../docs/dependency-map.md).

For implementation maturity, see [status-map](../../docs/status-map.md).
