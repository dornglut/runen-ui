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
`.styles(...)` and `.resources(...)`. Applications can opt into an explicit
validated authored initial focus with `.initial_focus(ElementId)`; the native
host never guesses an element to focus or overrides an already-focused UI.
Font policy can be overridden by `.runtime_config(...)`; native convenience
uses system and bundled fonts. No implicit host effect handler or missing-
resource fallback exists.

The native runner installs its monotonic clock and coalesced wake adapter
before initial application effects, using one canonical runtime. It uses the
bounded runtime pump report's next logical deadline for event-driven native
timer/motion scheduling; it does not continuously draw frames when idle.

**Current limitations:** advanced native IME text composition, framework
services (including clipboard), native touch, and application-defined host
request protocols are not yet implemented by this facade. The independent
Counter executable demonstrates typed actions, not a complete text-editing
host. These limitations require explicit implementation and platform proof
before general-purpose desktop acceptance.

The first native implementation is a standalone desktop delivery candidate
owned by #543. The host-specific behavior still needs native platform and
lifecycle qualification; a compiling crate is not acceptance. A windowless
`.run()` returns a typed unsupported-configuration error until a separate
headless runner contract is accepted. Headless and embedded clients continue
to use `runenui_runtime::AppRuntime` without native dependencies.
