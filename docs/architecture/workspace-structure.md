# Workspace Structure

> **Category: Target architecture**

RunenUI remains one workspace repository. Crates are extracted only to enforce real ownership, dependency, optionality, independent-consumer, or conformance boundaries.

## Current workspace

```text
RunenUI/
├── crates/
│   ├── runenui_core/
│   ├── runenui_runtime/
│   └── runenui_testing/
├── examples/
│   └── counter/
├── tests/
│   └── external_widget/
└── xtask/
```

| Package | Current ownership | Must not own |
|---|---|---|
| `runenui_core` | `UiApp`, host-neutral effects/work/subscription protocols; validated authored values/identity/style/geometry; transient views/elements; state-aware widgets; lifecycle/event contexts; semantic authoring/action vocabulary; doc-hidden safe runtime bridge | Persistent mounted/semantic storage, live scheduling, hosts, renderer backends, app state, ECS, or legacy dependencies |
| `runenui_runtime` | Generational mounted and semantic arenas; reconciliation/lifecycle; generalized FIFO and deterministic scheduler; clocks/tasks/timers/subscriptions/host requests; wake/redraw; routed input/defaults; bounded trace/replay; renderer and semantic surface publication; exact semantic-action resolution | Application domain policy, testing convenience state, native windows/accessibility adapters, concrete renderers, ECS, or legacy dependencies |
| `runenui_testing` | Public-only deterministic headless testing ergonomics over `runenui_core` + `runenui_runtime`: fixed test publication, bounded pumping/settling, exact snapshot-scoped semantic queries/targets, ordinary public ingress helpers, and read-only observation | Runtime behavior, private/internal test seams, mounted-state mutation, identity/sequence fabrication, parallel expected state, native host behavior, or semantic-to-mounted routing authority |
| `counter` | Application-owned state/action/update and headless public-API proof | Framework internals, native host, renderer backend, or legacy imports |
| `runenui_external_widget_conformance` | Non-publishable test-owned downstream controls and public conformance proofs; may consume `runenui_testing` only as a dev dependency for public-harness conformance | Production framework ownership, a production dependency on testing convenience, or privileged internal access |
| `xtask` | Repository validation orchestration | Framework runtime behavior |

Current production dependency direction is acyclic:

```text
runenui_core <- runenui_runtime
       ^             ^
       └──────┬──────┘
              ├── runenui_testing
              ├── counter
              └── external widget conformance
```

The external-widget conformance package additionally has a test-only edge to `runenui_testing`. Repository validation distinguishes production and dev dependency sections so that edge cannot silently become a normal framework dependency.

`xtask` is repository tooling and has no framework dependency. `runenui_testing` is deliberately downstream of runtime: runtime must never depend on testing convenience APIs.

## Ownership rules

`runenui_core` owns public host-neutral protocol and value definitions. `runenui_runtime` remains the sole live authority for namespace creation, mounted/semantic arenas, topology, validation, routes, interaction mutation, reconciliation, queue/work/trace sequences, scheduling, clocks, publication, semantic resolution, and shutdown.

`runenui_testing` may compose those public contracts, but it must not recreate them. In particular:

- it may retain an immutable `SurfacePublication`, but not replace runtime publication state;
- it may create a scoped semantic test target only from membership in an exact public `SemanticSnapshot`;
- it may delegate semantic actions only through `AppRuntime::submit_semantic_action`;
- it may build pointer input only from public `SurfaceInputContext` values emitted by publication;
- it may advance a public `ManualClock` and call the explicit bounded pump;
- it may inspect public state/focus/reconciliation/frame/layout/semantic/trace/replay products;
- it must not enable or depend on `internal-test-seams`, call private modules, fabricate IDs/sequences, mutate mounted state, or maintain a second expected runtime model.

This direction makes M5D an independently valuable public consumer and a Cargo-enforced proof that the accepted runtime APIs are sufficient for deterministic testing.

Within production crates, built-in authoring remains separate from the open element/widget protocol. Mounted storage, semantic storage/publication, routing, scheduling, tracing, and surface publication remain focused module families. File size alone is not a crate boundary or architecture decision.

## Extraction rule

A new crate requires at least one demonstrated property:

- independently valuable public API;
- meaningful Cargo feature or dependency optionality;
- a dependency direction Cargo must enforce;
- a substantial independently owned implementation;
- an independent conformance/test surface;
- a contract consumed by another repository, host, or backend.

A named concept, a large file, or a target diagram is not enough. Do not create empty target crates, genericize identity, or relocate geometry solely to satisfy an imagined graph.

M5D satisfies this rule because deterministic test ergonomics are independently consumable and must prove that ordinary public runtime contracts are sufficient without privileged access.

## Expected evolution

Later milestones may justify crates for render scenes, layout/style, text, controls, semantics/accessibility adapters, host contracts, desktop platform adapters, renderer backends, facade APIs, and devtools. Exact extraction points still require current ownership and dependency analysis.

Important direction rules remain:

- core never depends on runtime, testing, platform, renderer, controls, or legacy code;
- runtime never depends on testing, concrete platform, or concrete renderer implementations;
- testing depends only on public core/runtime contracts and never becomes live runtime authority;
- render protocols never depend on semantic controls or concrete backends;
- host contracts never depend on a specific windowing implementation;
- controls never depend on platform implementations;
- backends never own UI behavior;
- platform adapters never own widget behavior;
- product state and persistence remain in product repositories;
- Runenwerk/ECS integration remains an external adapter, not a RunenUI assumption.

## Milestone gates

- M5D justifies `runenui_testing` as the first public downstream testing crate after semantic publication/action prerequisites are accepted.
- M6 may justify a render-protocol crate after real independent-consumer pressure.
- M7 layout/style extraction still requires the gates in [ADR 0002](../adr/0002-keep-layout-in-runtime.md) and the styling architecture.
- M8 text may justify a text crate once shaping/editing/resources form an independently testable subsystem.
- M9 controls require all earlier public contracts.
- M10 introduces host/platform/backend crates only with real implementations.
- M11 may add the facade crate after the lower-level public surface is ready to stabilize.

See the [production roadmap](../roadmap.md) for the authoritative sequence and [work tracking](../work-tracking.md) for volatile execution state. The current public-surface ownership restrictions are recorded in the [public API contract](public-api.md).
