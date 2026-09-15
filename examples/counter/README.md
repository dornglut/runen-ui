# Counter Native Showcase

> **Category: Guide**

Counter is the repository's small application-level production showcase. It keeps one `CounterApp`, one state/action/update model, and one `ui.rs` authority while exercising both deterministic headless behavior and the native production UI path.

## Run the native application

```text
cargo run --package counter
```

The default binary opens a real winit window, publishes the ordinary Counter surface, renders it through `runenui_render_wgpu`, and exposes the same semantic tree through the accepted AccessKit adapter. The native host explicitly enables `SystemAndBundled` font discovery so ordinary platform fonts can satisfy the application typography without turning test fixtures into production assets.

- visible title, count, button, and win-screen text use the runtime-owned text/layout path and publication-retained shaped resources rendered through the normal SDF/MSDF renderer path;
- the horizontal control row is decrement, increment, and reset, with the authored labels rendered normally;
- ordinary count changes before the win screen transition the existing count background through the accepted M9 authored-transition/runtime-clock path; the motion is decorative, so reduced-motion policy uses the accepted default snap-to-end behavior rather than preserving it as essential;
- reaching the win count still switches structurally to the win screen rather than introducing showcase-only lifecycle state;
- use Tab / Shift-Tab to move runtime focus;
- use Enter or Space to activate the focused control;
- resize the window or move it across scale-factor boundaries; layout remains logical while the renderer re-realizes scale-dependent output and native point input remains tied to the exact successfully presented surface mapping;
- accessibility actions return through ordinary semantic action ingress.

Shaped text resources are retained by the paint publication and do not flow through Counter's external `ResourceProvider`; that provider remains only the host edge for external resources such as images.

## Font-policy boundary

The native application intentionally permits ambient system-font discovery because it is a production host. This does not change the framework default: `RuntimeConfig::default()` remains `BundledOnly`, and deterministic headless/conformance proofs continue to register controlled bundled fonts when their text output is part of the proof.

## Deterministic headless proof

The deterministic proof remains available as the `counter` binary and through tests:

```text
cargo run --package counter --bin counter
cargo test --package counter
```

It continues to cover mounted identity, routed pointer/keyboard/automation interaction, semantic publication/action, explicit bounded pumping, screen replacement, and trace behavior through ordinary public runtime contracts. The focused M9 dogfood proof additionally changes the ordinary Counter state, observes the transition start, advances public logical time explicitly to the midpoint and terminal sample, and verifies the count background through the normal surface publication path without sleeps, wall time, private motion state, or renderer-driven animation.

Terminal atomicity is covered separately by an explicitly test-only generation-exhaustion proof: the test enables `runenui_runtime/internal-test-seams` and uses `__seed_reconciliation_generation_for_test` to exercise terminal generation exhaustion. That seam is unrelated to the ordinary M9 motion proof.

## Boundaries

Counter owns application state, actions, update logic, transient views, application styling, and its application-specific host-loop policy. It does not own framework runtime state, text shaping or line breaking, renderer internals, native translation semantics, or a second semantic/input/layout/motion authority.

`runenui_winit` supplies only reusable native translation and AccessKit projection mechanics proven by both Counter and the specialized `reference_winit` conformance host. Each application still visibly owns its winit event loop, runtime pumping, redraw/publication acknowledgement, displayed-frame mapping, renderer recovery, and presentation policy.

Counter does not claim a standard control library, multi-window lifecycle, a generic native RunenUI runner, or conformance authority merely because it dogfoods accepted M9 motion. Repository-level M9 acceptance remains owned by the accepted M9 conformance/reconciliation process.

Repository-level conformance runs through `cargo validate`. See [current status](../../docs/status.md), the [M9 conformance matrix](../../docs/conformance/m9-conformance-matrix.md), [testing](../../TESTING.md), and the [roadmap](../../docs/roadmap.md).
