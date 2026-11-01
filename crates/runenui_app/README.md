# Fluent RunenUI Native Application (Draft)

This opt-in package composes one ordinary public `AppRuntime<A>`, the
`runenui_winit` adapter, and `runenui_render_wgpu` into a native window.
It is **not** a second UI runtime and does not require Runenwerk.

```rust
use runenui_app::App;

App::new()
    .window("Counter")
    .ui(Counter::new())
    .run()?;
```

The model implements the existing `runenui_core::UiApp` with `State = Self`.
The native shell supports a single window and `NoHostProtocol` currently.
Additional application styling and resource providers can be specified with
`.styles(...)` and `.resources(...)`. Font policy can be overridden by
`.runtime_config(...)`; native convenience defaults to system and bundled
fonts. No implicit host effect handler or missing-resource fallback exists.

The first native implementation is a standalone desktop delivery candidate
owned by #543. The host-specific behavior still needs native platform and
lifecycle qualification; a compiling crate is not acceptance. A windowless
`.run()` returns a typed unsupported-configuration error until a separate
headless runner contract is accepted. Headless and embedded clients continue
to use `runenui_runtime::AppRuntime` without native dependencies.
