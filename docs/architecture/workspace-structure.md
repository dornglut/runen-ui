# Workspace Structure

> **Category: Current architecture**

RunenUI remains one Rust workspace. Crates are extracted only when Cargo should enforce a real ownership, dependency, optionality, independent-consumer, or conformance boundary.

## Current workspace

```text
RunenUI/
├── crates/
│   ├── runenui_core/
│   ├── runenui_runtime/
│   ├── runenui_render_wgpu/
│   └── runenui_testing/
├── examples/
│   ├── counter/
│   └── reference_winit/
├── tests/
│   ├── external_renderer/
│   └── external_widget/
└── xtask/
```

| Package | Current ownership | Must not own |
|---|---|---|
| `runenui_core` | host-neutral application/effect protocols; validated authored values/identity/style/geometry; transient views/elements; state-aware widget/lifecycle/event/semantic vocabulary; opaque runtime-issued protocol value types | persistent mounted/semantic storage, live scheduling, native hosts, concrete renderers, application product state, ECS/legacy dependencies |
| `runenui_runtime` | generational mounted/semantic arenas; reconciliation/lifecycle; canonical queue/scheduler; focus/input state; clocks/tasks/timers/subscriptions/host requests; wake/redraw; trace/replay; layout and staged surface/semantic publication | application domain policy, testing convenience authority, native platform implementations, concrete renderers, ECS/legacy dependencies |
| `runenui_render_wgpu` | reusable renderer edge over ordinary public paint publications; caller-owned resource-provider contract; renderer-owned successful-publication lineage, realization/cache/backend work, readback, and renderer observations | native event loop, accessibility, widget/semantic/mounted/layout authority, runtime mutation, application resource registry, winit/AccessKit ownership |
| `runenui_testing` | public deterministic headless testing over ordinary `runenui_core` + `runenui_runtime` contracts | runtime behavior, private mutation seams, identity/sequence fabrication, parallel expected state, native host behavior |
| `counter` | application-owned state/action/update and ordinary public-API proof | framework internals or platform/backend ownership |
| `reference_winit` | standalone M7 native reference application: winit window/event-loop mechanics, native-to-neutral host translation, displayed native mapping, runtime wake/redraw driving, and consumption of the reusable wgpu renderer | framework/runtime behavior, widget semantics, renderer internals, a generic platform abstraction, or reusable accessibility-adapter authority |
| `runenui_external_renderer_conformance` | non-publishable genuine downstream renderer-neutral scene-consumer conformance proof over public core/runtime contracts | production renderer/backend ownership, testing-convenience dependency, native host/resource-provider authority, concrete widget/semantic interpretation, or privileged internal access |
| `runenui_external_widget_conformance` | non-publishable genuine downstream custom-widget/public conformance proof | production framework ownership or privileged internal access |
| `xtask` | deterministic repository validation/audit orchestration | framework runtime behavior |

Current production dependency direction is acyclic:

```text
runenui_core <- runenui_runtime
       ^             ^
       └──────┬──────┘
              ├── runenui_render_wgpu <- reference_winit
              ├── runenui_testing
              ├── counter
              ├── external renderer conformance
              └── external widget conformance

xtask  (repository tooling; no framework dependency)
```

`runenui_render_wgpu` depends only on ordinary public core/runtime contracts inside the workspace. Its backend/resource state remains downstream and disposable; native host and accessibility dependencies are intentionally outside the package boundary. The external-renderer conformance package remains a separate proof-only consumer and is not the production renderer.

`reference_winit` is the accepted M7 application boundary that consumes ordinary public core/runtime contracts plus `runenui_render_wgpu`, while owning winit and native host mechanics itself. This explicit exception does not make renderer consumption a default rule for examples: ordinary examples remain limited to core/runtime unless a later accepted architecture assigns them another boundary. `runenui_render_wgpu` therefore remains winit-free and does not absorb the event loop.

The external-renderer conformance package intentionally depends only on public `runenui_core` and `runenui_runtime` contracts, including for its own tests, so Cargo preserves the independent-consumer boundary. The external-widget conformance package may consume `runenui_testing` as a test/dev dependency without making testing convenience a production dependency. Repository validation distinguishes those dependency classes.

## Ownership rules

`runenui_core` owns public host-neutral protocol/value definitions. `runenui_runtime` remains the sole live authority for namespace creation, mounted and semantic arenas, topology, reconciliation, routes, interaction mutation, queue/work/trace sequencing, scheduling, clocks, publication, semantic resolution, and shutdown.

`runenui_render_wgpu` may retain only renderer-owned realization state derived from ordinary immutable paint publications plus caller-owned resource payloads. It must not inspect concrete widget/control types, semantic-tree facts, mounted/layout storage, or private runtime mutation seams, and it must not become resource-provider or application-state authority.

`reference_winit` may translate native host facts, drive public runtime APIs from its event-loop thread, retain immutable publications for renderer retry, and retain the exact successfully displayed native mapping needed for point ingress. It must not create a second UI queue, mutate runtime off-thread, reinterpret widget semantics, move winit into the renderer, or become a generic reusable host/platform crate without a second real consumer.

`runenui_testing` may compose public contracts but must not recreate live authority. In particular it may retain immutable public publications for inspection, create scoped semantic test targets only from exact public snapshot membership, delegate semantic actions only through public runtime ingress, advance a public manual clock, perform explicit bounded pumping, and inspect public products. It must not fabricate runtime IDs/sequences, mutate mounted state, use private runtime bridges, or maintain a second expected runtime model.

Built-in authoring uses the same open element/widget protocol as downstream controls. Mounted storage, semantic storage/publication, routing, scheduling, tracing, and surface publication remain focused runtime module families. File size alone is not a crate boundary.

## Extraction rule

A new crate requires at least one demonstrated property:

- independently valuable public API;
- meaningful Cargo feature or dependency optionality;
- dependency direction Cargo must enforce;
- substantial independently owned implementation;
- independent conformance/test surface;
- contract consumed by another repository, host, or backend.

A named concept, target diagram, or large source file is not enough. Do not create empty target crates, genericize identity, or relocate values solely to satisfy an imagined package graph.

Future layout/style, text, controls, semantics/accessibility-adapter, host, platform, facade, or devtool crates require those same demonstrated boundaries when their owning milestones become real. The [roadmap](../roadmap.md) owns sequencing; exact current public ownership is summarized in the [public API contract](public-api.md).
