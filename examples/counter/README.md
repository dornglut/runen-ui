# Counter Headless Proof

> **Category: Guide**

The Counter example demonstrates the currently implemented typed headless loop:

- application-owned `Counter` state and `CounterAction`;
- explicit `update` and conditional root composition;
- typed-builder-authored text/buttons with canonical `on_press` actions and arity-free children;
- direct and validated authored-ID activation;
- transition to a win screen and reset;
- deterministic trace/debug output;
- surface publication with tight constraints, deterministic measurement, and aligned frame/style/layout diagnostics.

Run it with:

```powershell
cargo run --package counter
```

This is not a desktop application, renderer/backend proof, production control example, accessibility proof, or production text demonstration. It uses transient preorder runtime IDs, press activation, deterministic character-count measurement, the small row/column layout, and semantic `SurfaceFrame` kinds. It does not use windowing, GPU rendering, ECS, compiler/program/artifact machinery, or legacy code.

See the [feature/support matrix](../../docs/feature-support-matrix.md) for the exact limits and the [roadmap](../../docs/roadmap.md) for production gates.
