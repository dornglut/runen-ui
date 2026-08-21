# Workspace Structure

> **Category: Current architecture**

RunenUI remains one Rust workspace. Crates are extracted only when Cargo should enforce a real ownership, dependency, optionality, independent-consumer, or conformance boundary.

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
| `runenui_core` | host-neutral application/effect protocols; validated authored values/identity/style/geometry; transient views/elements; state-aware widget/lifecycle/event/semantic vocabulary; opaque runtime-issued protocol value types | persistent mounted/semantic storage, live scheduling, native hosts, concrete renderers, application product state, ECS/legacy dependencies |
| `runenui_runtime` | generational mounted/semantic arenas; reconciliation/lifecycle; canonical queue/scheduler; focus/input state; clocks/tasks/timers/subscriptions/host requests; wake/redraw; trace/replay; layout and staged surface/semantic publication | application domain policy, testing convenience authority, native platform implementations, concrete renderers, ECS/legacy dependencies |
| `runenui_testing` | public deterministic headless testing over ordinary `runenui_core` + `runenui_runtime` contracts | runtime behavior, private mutation seams, identity/sequence fabrication, parallel expected state, native host behavior |
| `counter` | application-owned state/action/update and ordinary public-API proof | framework internals or platform/backend ownership |
| `runenui_external_widget_conformance` | non-publishable genuine downstream custom-widget/public conformance proof | production framework ownership or privileged internal access |
| `xtask` | deterministic repository validation/audit orchestration | framework runtime behavior |

Current production dependency direction is acyclic:

```text
runenui_core <- runenui_runtime
       ^             ^
       └──────┬──────┘
              ├── runenui_testing
              ├── counter
              └── external widget conformance

xtask  (repository tooling; no framework dependency)
```

The external-widget conformance package may consume `runenui_testing` as a test/dev dependency without making testing convenience a production dependency. Repository validation distinguishes those dependency classes.

## Ownership rules

`runenui_core` owns public host-neutral protocol/value definitions. `runenui_runtime` remains the sole live authority for namespace creation, mounted and semantic arenas, topology, reconciliation, routes, interaction mutation, queue/work/trace sequencing, scheduling, clocks, publication, semantic resolution, and shutdown.

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

Future render, layout/style, text, controls, semantics/accessibility-adapter, host, platform, backend, facade, or devtool crates require those same demonstrated boundaries when their owning milestones become real. The [roadmap](../roadmap.md) owns sequencing; exact current public ownership is summarized in the [public API contract](public-api.md).
