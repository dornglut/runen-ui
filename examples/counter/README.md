# Counter Headless Proof

> **Category: Guide**

The Counter example demonstrates the currently implemented typed headless loop:

- application-owned `Counter` state and `CounterAction`;
- explicit `update` and conditional root composition;
- typed `View`-authored text/buttons/containers using the open widget protocol,
  separate private built-in behavior widgets, canonical `on_press` actions,
  `ChildLayout`-backed row/column authoring, and arity-free children;
- direct and validated authored-ID activation;
- explicit owned action extraction followed by immediate root rebuild;
- transition to a win screen and reset;
- deterministic trace/debug output;
- surface publication with tight constraints, deterministic measurement, open
  paint/semantic proof facts, and aligned frame/style/layout diagnostics.

Run it with:

```powershell
cargo run --package counter
```

This is not a desktop application, renderer/backend proof, production control
example, accessibility proof, or production text demonstration. It uses
transient preorder runtime IDs, press activation, deterministic character-count
measurement, separate intrinsic/child-layout snapshots per publication, the
small row/column layout, aligned publication products, and proof-level
`SurfaceFrame` facts.
It does not use persistent widget state, production semantics/paint scenes,
windowing, GPU rendering, ECS, compiler/program/artifact machinery, or legacy
code.

See the [feature/support matrix](../../docs/feature-support-matrix.md) for the exact limits and the [roadmap](../../docs/roadmap.md) for production gates.
