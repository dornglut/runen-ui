# Workspace Structure

> **Category: Current architecture**

RunenUI remains one Rust workspace. Crates are extracted only when Cargo should enforce a real ownership, dependency, optionality, independent-consumer, or conformance boundary.

## Current workspace

```text
RunenUI/
├── crates/
│   ├── runenui_core/
│   ├── runenui_text/
│   ├── runenui_runtime/
│   ├── runenui_render_wgpu/
│   ├── runenui_winit/
│   └── runenui_testing/
├── examples/
│   ├── counter/
│   └── reference_winit/
├── tests/
│   ├── external_host/
│   ├── external_renderer/
│   └── external_widget/
└── xtask/
```

| Package | Current ownership | Must not own |
|---|---|---|
| `runenui_core` | host-neutral application/effect protocols; validated authored values/identity/style/geometry/layout; transient views/elements; state-aware widget/lifecycle/event/semantic vocabulary; bounded widget-measurement and child-bearing contracts; opaque runtime-issued protocol value types | persistent mounted/semantic storage, live layout topology/algorithm/cache state, live scheduling, native hosts, concrete renderers, application product state, ECS/legacy dependencies |
| `runenui_text` | renderer-neutral font-source policy and cache-visible source revision; production shaping/line breaking, text-specific constraints, immutable logical artifacts and shaped-resource bindings behind RunenUI-owned contracts | mounted/runtime/publication authority, general layout topology/scheduling, renderer/GPU/SDF-MSDF atlas state, native host/accessibility, application state, editable-text behavior |
| `runenui_runtime` | generational mounted/semantic arenas; reconciliation/lifecycle; canonical queue/scheduler; focus/input state; clocks/tasks/timers/subscriptions/host requests; wake/redraw; trace/replay; style/text orchestration; production layout over exact mounted topology through private low-level Taffy algorithms; final logical geometry/extents; staged surface/semantic publication | application domain policy, testing convenience authority, native platform implementations, concrete renderers, text-shaping/font-algorithm authority, retained Taffy topology/identity authority, ECS/legacy dependencies |
| `runenui_render_wgpu` | reusable renderer edge over ordinary public paint publications; external-image resource-provider contract; retained shaped-text consumption; renderer-owned successful-publication lineage, SDF/MSDF/image realization/cache/backend work, readback, and renderer observations | native event loop, accessibility, widget/semantic/mounted/layout authority, runtime mutation, application resource registry, shaping/line breaking/font discovery, winit/AccessKit ownership |
| `runenui_winit` | reusable winit input/device translation and AccessKit semantic projection/action translation proven by the reference host and native Counter | window/event-loop ownership, runtime pumping, redraw/publication policy, displayed-frame authority, renderer/presentation lifecycle, application behavior |
| `runenui_testing` | public deterministic headless testing over ordinary `runenui_core` + `runenui_runtime` contracts | runtime behavior, private mutation seams, identity/sequence fabrication, parallel expected state, native host behavior |
| `counter` | application-owned state/action/update/UI plus the bounded native M7 application host composition and deterministic headless proof | framework internals, reusable native translation authority, renderer internals, generic host/facade ownership |
| `reference_winit` | specialized M7 native conformance application: winit window/event-loop mechanics, displayed native mapping, runtime wake/redraw driving, renderer presentation/recovery, and proof logging | framework/runtime behavior, widget semantics, renderer internals, reusable native translation/accessibility authority, generic platform abstraction |
| `runenui_external_host_conformance` | non-publishable winit-free downstream host proof over public core/runtime/renderer contracts; caller-owned submit/pump/redraw/publish/ack/render/present sequencing and retained-publication resource retry | native host/accessibility dependencies, testing convenience, direct wgpu ownership, private runtime seams, hidden framework loop, generic host/facade authority |
| `runenui_external_renderer_conformance` | non-publishable genuine downstream renderer-neutral scene-consumer conformance proof over public core/runtime contracts | production renderer/backend ownership, testing-convenience dependency, native host/resource-provider authority, concrete widget/semantic interpretation, or privileged internal access |
| `runenui_external_widget_conformance` | non-publishable genuine downstream custom-widget/public conformance proof, including bounded public custom measurement | production framework ownership, private topology access, reentrant runtime mutation, second layout engine, or privileged internal access |
| `xtask` | deterministic repository validation/audit orchestration | framework runtime behavior |

Current production dependency direction is acyclic. The accepted M8B text boundary and M8C production layout integration are current architecture:

```text
runenui_text        -> runenui_core
runenui_runtime     -> runenui_core + runenui_text
runenui_render_wgpu -> runenui_core + runenui_runtime + runenui_text
runenui_winit       -> runenui_core + runenui_runtime
runenui_testing     -> runenui_core + runenui_runtime
```

The application and downstream-conformance packages consume those public layers as required by their owned profiles; `xtask` remains repository tooling with no framework dependency.

`runenui_text` depends only on ordinary public `runenui_core` contracts plus its reviewed text/font dependency stack. Parley/Fontique/HarfRust/Skrifa/ICU remain private implementation dependencies. Its font collection, shaping/line-breaking state, reusable text-layout state, immutable artifacts, and shaped-resource bindings are renderer-neutral derived state; they do not own mounted topology, runtime scheduling/publication, GPU realization, native integration, semantics, or editing behavior.

`runenui_runtime` depends on `runenui_text` to orchestrate the live text system, lower private Taffy layout availability into text-specific constraints, retain topology-aligned reusable text state, measure from immutable logical artifacts, and carry the exact shaped-resource leases into paint publication. That dependency does not transfer mounted, scheduling, layout-topology, or publication authority into `runenui_text`.

Taffy is an exact private dependency of `runenui_runtime`, not a workspace authority layer. Runtime uses its low-level/custom-tree Block/Flex/Grid algorithms directly over the existing mounted topology. Taffy node translation and caches are transaction-local/disposable; `TaffyTree`, public Taffy/CSS types, a second retained topology, or cross-frame dependency-owned layout authority are not part of the architecture.

`runenui_render_wgpu` consumes public core/runtime paint-publication contracts plus the exact retained `runenui_text` shaped-resource facts needed for already-shaped outline realization. External images still use the caller-owned complete-`ResourceRef` provider contract. Runtime-shaped text does not: its immutable binding is retained by publication, while field generation, quality classes, atlas pages, GPU textures, and cache lifetime are disposable renderer-owned state. The renderer has no shaping, line-breaking, fallback, or font-discovery authority.

`runenui_winit` is the targeted adapter boundary justified by the second real winit consumer in M7. It consumes ordinary public core/runtime contracts plus winit/AccessKit types and owns only loss-preserving native translation/projection mechanics. It deliberately has no renderer dependency and no run-loop API. Moving a substantial accepted adapter into one Cargo-owned source prevents Counter from copying the specialized reference application while leaving application-specific host policy visible.

`reference_winit` remains the specialized conformance host; Counter is the bounded application showcase. Both consume `runenui_render_wgpu` and `runenui_winit`, but each owns its own winit event loop, runtime pumping, redraw/publication acknowledgement, displayed-frame mapping, renderer recovery, and presentation policy. This does not establish a generic RunenUI host facade.

The external-host conformance package independently consumes public core/runtime plus `runenui_render_wgpu` without importing native-host or accessibility orchestration. Its frame sequence remains explicit caller code, including acknowledgement after successful publication and renderer retry against the retained immutable publication. This Cargo-enforced consumer demonstrates that the accepted reusable embedding boundary is the public runtime/publication/renderer contract rather than a framework-owned host loop.

The external-renderer conformance package intentionally depends only on public `runenui_core` and `runenui_runtime` contracts, including for its own tests, so Cargo preserves the independent-consumer boundary. The external-widget conformance package consumes ordinary public widget/layout/measurement contracts and may consume `runenui_testing` as a test/dev dependency without making testing convenience a production dependency. Repository validation distinguishes those dependency classes.

## Ownership rules

`runenui_core` owns public host-neutral protocol/value definitions. `runenui_runtime` remains the sole live authority for namespace creation, mounted and semantic arenas, topology, reconciliation, routes, interaction mutation, queue/work/trace sequencing, scheduling, clocks, layout orchestration, publication, semantic resolution, and shutdown.

`runenui_text` owns renderer-neutral font-source configuration, dependency-backed shaping/line breaking, text-specific constraints and reuse decisions, immutable logical text artifacts, and logical shaped-resource bindings. It must not import runtime/mounted/publication authority, general layout topology, renderer/backend state, native/accessibility integration, or editing behavior. Public contracts remain RunenUI-owned and must not expose upstream dependency types as authority.

`runenui_runtime` may use Taffy only as a private algorithm provider. Exact mounted topology/order, layout invalidation, measurement dispatch, final logical geometry, logical overflow/extents, and publication compatibility remain runtime-owned; Taffy state is derived and disposable.

`runenui_render_wgpu` may retain only renderer-owned realization state derived from ordinary immutable paint publications, retained shaped-text resources, and caller-owned external resource payloads. It must not inspect concrete widget/control types, semantic-tree facts, mounted/layout storage, or private runtime mutation seams, and it must not become font/shaping, external-resource-provider, or application-state authority.

`runenui_winit` may retain only native device/key/pointer lifetime translation state and rebuildable AccessKit projection identity/cache derived from public semantic publication. It must not create a second UI queue, pump or mutate the runtime, own a window/event loop, retain renderer state, acknowledge redraw, decide displayed-frame authority, or become application policy.

Native applications may translate native host facts through `runenui_winit`, drive public runtime APIs from their event-loop thread, retain immutable publications for renderer retry, and retain the exact successfully displayed native mapping needed for point ingress. They must not create a second UI queue, mutate runtime off-thread, reinterpret widget semantics, move winit into the renderer, or hide host-loop ownership behind a speculative generic facade.

A non-native external host may drive the same public runtime and renderer contracts directly: it owns when work is submitted and pumped, when redraw work becomes a publication, when that publication is acknowledged, when renderer work is retried, and how a successful renderer result becomes presented host state. Renderer failure must not cause unchanged runtime republish or transfer loop authority into RunenUI.

`runenui_testing` may compose public contracts but must not recreate live authority. In particular it may retain immutable public publications for inspection, create scoped semantic test targets only from exact public snapshot membership, delegate semantic actions only through public runtime ingress, advance a public manual clock, perform explicit bounded pumping, and inspect public products. It must not fabricate runtime IDs/sequences, mutate mounted state, use private runtime bridges, or maintain a second expected runtime model.

Built-in authoring uses the same open element/widget protocol as downstream controls. Mounted storage, semantic storage/publication, routing, scheduling, tracing, layout/text orchestration, and surface publication remain focused runtime module families. File size alone is not a crate boundary.

## Extraction rule

A new crate requires at least one demonstrated property:

- independently valuable public API;
- meaningful Cargo feature or dependency optionality;
- dependency direction Cargo must enforce;
- substantial independently owned implementation;
- independent conformance/test surface;
- contract consumed by another repository, host, or backend.

A named concept, target diagram, or large source file is not enough. Do not create empty target crates, genericize identity, or relocate values solely to satisfy an imagined package graph.

Future controls, host, platform, facade, or devtool crates require those same demonstrated boundaries when their owning milestones become real. Layout remains runtime-owned unless a real future extraction boundary is separately accepted. The [roadmap](../roadmap.md) owns sequencing; exact current public ownership is summarized in the [public API contract](public-api.md).
