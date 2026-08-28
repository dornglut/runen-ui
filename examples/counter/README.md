# Counter Native Showcase

> **Category: Guide**

Counter is the repository's small application-level M7 showcase. It keeps one `CounterApp`, one state/action/update model, and one `ui.rs` authority while exercising both deterministic headless proof and the accepted native production edges.

## Run the native application

```text
cargo run --package counter
```

The default binary opens a real winit window, publishes the ordinary Counter surface, renders it through `runenui_render_wgpu`, and exposes the same semantic tree through the accepted AccessKit adapter.

- click **-**, **+**, or **Reset** with the mouse;
- use Tab / Shift-Tab to move runtime focus;
- use Enter or Space to activate the focused control;
- resize the window or move it across scale-factor boundaries; native point input remains tied to the exact successfully presented surface mapping;
- accessibility actions return through ordinary semantic action ingress.

The current M7 control/text vocabulary does not yet render glyphs; production shaping and text rendering remain M8 work. The showcase therefore uses existing proof-level literal background paint so count changes, control activation flow, reset, and the win-screen transition produce real visible wgpu output without inventing an M7 text stack. Control names and count text remain truthful semantic/accessibility facts.

## Deterministic headless proof

The original deterministic proof remains available as the `counter` binary and through tests:

```text
cargo run --package counter --bin counter
cargo test --package counter
```

It continues to cover mounted identity, routed pointer/keyboard/automation interaction, semantic publication/action, explicit bounded pumping, screen replacement, trace behavior, and terminal atomicity over ordinary public runtime contracts.

## Boundaries

Counter owns application state, actions, update logic, transient views, and its application-specific host-loop policy. It does not own framework runtime state, renderer internals, native translation semantics, or a second semantic/input authority.

`runenui_winit` supplies only reusable native translation and AccessKit projection mechanics proven by both Counter and the specialized `reference_winit` conformance host. Each application still visibly owns its winit event loop, runtime pumping, redraw/publication acknowledgement, displayed-frame mapping, renderer recovery, and presentation policy.

Counter does not claim M7D external-host closure, M8 production style/text, a standard control library, multi-window lifecycle, or a generic native RunenUI runner.

Repository-level conformance runs through `cargo validate`. See [current status](../../docs/status.md), the [M7 conformance matrix](../../docs/conformance/m7-conformance-matrix.md), [testing](../../TESTING.md), and the [roadmap](../../docs/roadmap.md).
