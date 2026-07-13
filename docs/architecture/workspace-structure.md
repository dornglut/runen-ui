# Workspace Structure

> **Category: Target architecture**

RunenUI remains one workspace repository. Crates are extracted only to enforce real ownership, dependency, optionality, independent-consumer, or conformance boundaries.

## Current workspace

```text
RunenUI/
├── crates/
│   ├── runenui_core/
│   └── runenui_runtime/
├── examples/
│   └── counter/
├── tests/
│   └── external_widget/
└── xtask/
```

| Package | Current ownership | Must not own |
|---|---|---|
| `runenui_core` | Validated authored values/identity/style; transient views/elements; state-aware widgets; lifecycle contexts; invalidation; technically public doc-hidden unstable safe runtime bridge | Persistent mounted storage, hosts, renderer backends, app state, ECS, or legacy dependencies |
| `runenui_runtime` | Generational mounted arena/tree, reconciliation, lifecycle/shutdown, focus/interaction slots, caches/invalidation phases, mounted dispatch/targeting, aligned publication, hit testing, and trace | Application domain state, M4 event/effect scheduling, native windows, concrete renderers, ECS, or legacy dependencies |
| `counter` | Application-owned state/action/update and headless public-API proof | Framework internals, native host, renderer backend, or legacy imports |
| `runenui_external_widget_conformance` | Non-publishable test-owned downstream controls, vertical/horizontal/intrinsic/unsupported child-layout widgets, mapping, state/lifecycle, alignment, hit, and snapshot proof | Production framework ownership or privileged internal access |
| `xtask` | Repository validation orchestration | Framework runtime behavior |

Current dependency direction is acyclic:

```text
runenui_core <- runenui_runtime <- counter
       ^              ^
       └──────────────┴── external widget conformance
```

`xtask` is repository tooling and has no framework dependency.

Within the crates, built-in authoring is separate from the public element/widget
protocol. Mounted storage is divided into arena, identity, node, capability
cache, invalidation, interaction, matching, lifecycle, and diagnostic modules.
Surface context/key/cache ownership is separate from phase resolution,
measurement, arrangement, and publication code. These are module boundaries,
not new crate boundaries.

## Extraction rule

A new crate requires at least one demonstrated property:

- independently valuable public API;
- meaningful Cargo feature or dependency optionality;
- a dependency direction Cargo must enforce;
- a substantial independently owned implementation;
- an independent conformance/test surface;
- a contract consumed by another repository, host, or backend.

A named concept, a large file, or a target diagram is not enough. Do not create empty target crates, genericize identity, or relocate geometry solely to satisfy an imagined graph.

## Expected evolution

The roadmap may eventually justify crates for a facade, runtime, layout, style, render scenes, text, controls, semantics/accessibility adapters, host contracts, a desktop platform adapter, a conventional backend, an optional SDF backend, testing, and devtools. Exact extraction points require ADRs and current dependency analysis.

Important direction rules remain:

- core never depends on runtime, platform, renderer, controls, or legacy code;
- runtime never depends on concrete platform or renderer implementations;
- render protocols never depend on semantic controls or concrete backends;
- host contracts never depend on a specific windowing implementation;
- controls never depend on platform implementations;
- backends never own UI behavior;
- platform adapters never own widget behavior;
- product state and persistence remain in product repositories;
- Runenwerk/ECS integration remains an external adapter, not a RunenUI assumption.

## Milestone gates

- M2–M3 establish extensible widget and mounted-runtime ownership before a final crate graph can be trusted.
- M6 may justify a render-protocol crate after two independent consumers.
- M7 layout/style extraction still requires the gates in [ADR 0002](../adr/0002-keep-layout-in-runtime.md) and the styling architecture.
- M8 text may justify a text crate once shaping/editing/resources form an independently testable subsystem.
- M9 controls require all earlier public contracts.
- M10 introduces host/platform/backend crates only with real implementations.
- M11 may add the facade crate after the lower-level public surface is ready to stabilize.

See the [production roadmap](../roadmap.md) for the authoritative sequence.

The current public-surface ownership and M1/M2 construction restrictions are
recorded in the [public API contract](public-api.md).
