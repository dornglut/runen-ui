# Counter Native Showcase

> **Category: Guide**

Counter is the repository's small application-level production showcase. It keeps one `CounterApp`, one state/action/update model, and one `ui.rs` authority while exercising both deterministic headless behavior and the native production UI path.

## Run the native application

```text
cargo run --package counter
```

The default binary opens a real winit window, publishes the ordinary Counter surface, renders it through `runenui_render_wgpu`, and exposes the same semantic tree through the accepted AccessKit adapter. The native host explicitly enables `SystemAndBundled` font discovery so ordinary platform fonts can satisfy the application typography without turning test fixtures into production assets.

- visible title, count, button, and win-screen text use the runtime-owned text/layout path and publication-retained shaped resources rendered through the normal SDF/MSDF renderer path;
- the control row keeps decrement/increment together as one intrinsic Button stepper group, uses the mathematical minus glyph for a more balanced pair, centers the row through Flex, and wraps Reset below the pair as width tightens; equal fixed/minimum Button geometry is deliberately deferred to the framework-owned content-alignment decision in #287 rather than approximated with Counter-local padding or label offsets;
- symmetric zero-basis Flex spacers center the ordinary content vertically when room exists and collapse away when height tightens, so overflow remains top-reachable without viewport breakpoints or unsafe centered overflow;
- one Counter-owned style environment supplies ordinary and Reset recipes; canonical runtime hover, focus, and active facts resolve through those recipes rather than application-owned interaction state;
- hover and press retarget a short decorative background transition, while runtime focus is immediately visible through a persistent outline that remains distinct from hover;
- ordinary count changes before the win screen transition the existing count background through the accepted M9 authored-transition/runtime-clock path; the motion is decorative, so reduced-motion policy uses the accepted default snap-to-end behavior rather than preserving it as essential;
- reaching the win count preserves one keyed screen/content shell, transitions the shell background, and fades the incoming win content through the accepted M9 path while ordinary child identity reconciles where keys permit; Reset reuses that same shell on the way back;
- the screen is a runtime scroll container on both axes, and the native Counter now forwards winit mouse-wheel input through the accepted `runenui_winit::MouseInputState::wheel` path so clipped content remains reachable after aggressive resize;
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

It continues to cover mounted identity, routed pointer/keyboard/automation interaction, semantic publication/action, explicit bounded pumping, screen reconciliation, and trace behavior through ordinary public runtime contracts. The focused visual-interaction proof drives canonical pointer hover, runtime focus, primary press, and release through the same Counter-owned style environment, proves stepper grouping plus narrow-surface wrapping, observes the sampled background/focus treatment, and verifies the stable win-screen shell transition with explicit logical time and no application-owned interaction state. The focused M9 dogfood proof separately changes ordinary Counter state and verifies the existing count-background transition through the same publication path.

Terminal atomicity is covered separately by an explicitly test-only generation-exhaustion proof: the test enables `runenui_runtime/internal-test-seams` and uses `__seed_reconciliation_generation_for_test` to exercise terminal generation exhaustion. That seam is unrelated to the ordinary M9 motion proof.

## Boundaries

Counter owns application state, actions, update logic, transient views, application styling, and its application-specific host-loop policy. It does not own framework runtime state, text shaping or line breaking, renderer internals, native translation semantics, or a second semantic/input/layout/motion authority.

`runenui_winit` supplies only reusable native translation and AccessKit projection mechanics proven by both Counter and the specialized `reference_winit` conformance host. Each application still visibly owns its winit event loop, runtime pumping, redraw/publication acknowledgement, displayed-frame mapping, renderer recovery, and presentation policy.

Counter does not become a control gallery, multi-window lifecycle example, generic native RunenUI runner, or conformance authority merely because it dogfoods the accepted Text/Button and M9 style/motion paths. Repository-level control and motion acceptance remains owned by their conformance contracts rather than this example.

Repository-level conformance runs through `cargo validate`. See [current status](../../docs/status.md), the [M9 conformance matrix](../../docs/conformance/m9-conformance-matrix.md), the [M11 conformance matrix](../../docs/conformance/m11-conformance-matrix.md), [testing](../../TESTING.md), and the [roadmap](../../docs/roadmap.md).
