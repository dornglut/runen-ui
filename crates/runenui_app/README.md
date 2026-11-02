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

The equivalent `App::new().ui(Counter::new()).window("Counter").run()?`
ordering is also supported; both paths create the same single native host.

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

**Current limitations:** advanced native IME text composition, native
touch, platform clipboard/cursor/drag-drop services and application-defined
host-request protocols are not yet implemented by this facade. Unsupported
framework service requests are completed with an explicit typed
`FrameworkServiceFailure::Unavailable` response, not silently retained;
this is not equivalent to providing the native operation. The independent
Counter executable demonstrates typed actions, not a complete text-editing
host. These limitations require explicit implementation and platform proof
before general-purpose desktop acceptance.

Two independently authored sample programs exercise different public contracts:
`fluent_counter` uses `.ui(model)` and pointer-triggered actions;
`fluent_status` uses `.ui_with_state::<StatusUi>(state)`, two application
actions and an initial 1500-ms timer. The
[Native X11 Smoke](../../.github/workflows/native-x11-smoke.yml)
workflow builds both and checks that visible native windows start, resize and
exit after OS close under Xvfb and software Vulkan. For `fluent_status`,
it additionally interrupts a held Space key by unmapping the native X11
window, verifies that its release causes no hidden-window activation,
remaps the same window, and requires a fresh OS keyboard-triggered action
following restoration. A separate Counter
window then receives OS focus while Status has a held Space key; the test
asserts no unfocused release activation and confirms new keyboard input after
focus returns, without manipulating runtime internals. An additional debug-only,
explicit `internal-test-seams` X11 run injects each recoverable *host-visible*
surface status (timeout, occlusion, outdated, suboptimal, unconfigured, lost)
before the first successful Present, then requires the very same immutable
publication to survive all simulated errors and Present successfully. The
default-feature native run still has no injection. This is host recovery-policy
fault injection, **not** a real GPU device-loss, driver fault, IME or physical
platform qualification. A separate debug-only X11 negative-path run injects
`SurfaceValidation` and requires the ordinary executable to terminate with
the typed native fatal error and a nonzero exit, without claiming a real
driver validation incident. This is Linux X11 virtual
display evidence, **not** Windows/macOS, physical input, IME, accessibility
or arbitrary device-loss qualification.

The [Native X11 Smoke workflow](../../.github/workflows/native-x11-smoke.yml)
also runs independent Windows/macOS compile jobs for the public examples,
native crate and test-only feature on hosted runners. Successful
cross-platform compilation is not evidence that a physical native OS window
ran or that its platform input/accessibility and GPU-loss behavior is qualified.

The first native implementation is a standalone desktop delivery candidate
owned by #543. The host-specific behavior still needs native platform and
lifecycle qualification; a compiling crate is not acceptance. A windowless
`.run()` returns a typed unsupported-configuration error until a separate
headless runner contract is accepted. Headless and embedded clients continue
to use `runenui_runtime::AppRuntime` without native dependencies.
