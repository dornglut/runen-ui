# Public API Contract

This document describes the **conceptual ownership and invariants** of RunenUI's current public surface. Exact Rust signatures, trait bounds, visibility, and documentation are authoritative in source/Rustdoc.

## `runenui_core`

`runenui_core` owns host-neutral public values and protocols that must be usable without a live runtime or platform/backend dependency. Its responsibilities include:

- `UiApp` application state/action/update and host-neutral effect/subscription protocol values;
- validated authored identity, style, geometry, transient `View`/`Element` authoring, and typed built-in view vocabulary;
- state-aware open widget/lifecycle/event contracts and typed action mapping;
- runtime-local opaque protocol identity types such as mounted/semantic/surface/work identities, without allocation authority;
- host-neutral pointer/keyboard/text/composition/focus/semantic command and semantic contribution/action vocabulary;
- renderer- and host-neutral paint/hit contribution values, logical scene-composition geometry, opaque neutral resource identity/kind values, and image/shaped-run primitive placement values used by the accepted M6 scene protocol.

Core must not own persistent mounted/semantic storage, live queue/scheduler state, runtime identity allocation, native window/accessibility objects, renderer backend handles, resource-provider/lookup/payload/cache authority, decoding/shaping/realization, application product state, or testing-only mutation seams.

## `runenui_runtime`

`runenui_runtime` owns live framework authority:

- runtime namespace and generational mounted/semantic storage;
- reconciliation, lifecycle execution, focus/interaction state, and capability/invalidation caches;
- one generalized sequenced work queue, bounded pump, tasks/timers/subscriptions/host requests, clocks, wake/redraw, and shutdown;
- exact routed command/input processing and defaults;
- bounded canonical trace, deterministic export, and inert replay projections;
- measurement/layout execution and staged surface publication;
- canonical renderer-neutral transformed/clipped/ordered paint-scene composition plus `RasterScale` and `PaintPublication` revision/base/damage/alignment authority;
- canonical transformed/clipped/ordered displayed `HitTestScene` composition, mounted-target/membership injection, retained displayed-generation lookup, and point/resolved-target authority;
- scene requirements derived from canonical paint content and neutral consumer capability checks without resource lookup or backend-specific rewriting;
- independent semantic publication/update/diagnostics and exact semantic-action admission/resolution.

Runtime must not depend on testing convenience, concrete native platforms, concrete renderer implementations, product state, resource-provider/lookup/payload/cache ownership, decoding/shaping/realization, or a second semantic/paint/hit/testing authority.

## `runenui_render_wgpu`

`runenui_render_wgpu` is the accepted reusable concrete renderer edge over ordinary public paint publication. It owns:

- wgpu instance/adapter/device/queue and surface/offscreen target state;
- exact renderer-local successful-publication lineage and update/full-resync classification;
- caller-facing complete-`ResourceRef` provider requests plus disposable backend realization/cache state;
- the accepted image and bounded shaped-run resource payload realization path;
- native-surface presentation, offscreen readback, and immutable renderer observation records.

It consumes public core/runtime paint contracts and caller-owned resource payloads only. It must not own a native event loop, widget/semantic/mounted/layout authority, runtime mutation, application resource identity/bindings, production shaping/layout policy, or AccessKit/winit behavior.

## `runenui_testing`

`runenui_testing` is a downstream public convenience crate. `TestHarness<App>` composes ordinary public core/runtime APIs with deterministic logical time, bounded pumping/settling, deterministic surface publication, read-only observation of the latest ordinary public paint/hit publication products and exact input context, synthetic public interaction, and semantic queries/targets.

It owns no live runtime queue, mounted/semantic store, identity allocation, publication state, trace authority, resource provider, or private mutation bridge. A test target retains exact public surface/semantic scope; testing must not reconstruct private mounted routing identity, fabricate scene/publication lineage, duplicate hit resolution, or guess a surface from a bare semantic ID.

## Accepted native reference integration

The accepted M7B/M7C reference path is current behavior but is not a generic public host facade. `examples/reference_winit` visibly owns its winit window/event loop, runtime pumping and redraw driving, native coordinate mapping, renderer presentation/recovery policy, and native-to-neutral input translation. Its AccessKit adapter consumes ordinary semantic publication and returns actions through ordinary semantic ingress while retaining only rebuildable adapter-owned projection identity/state.

Native winit/AccessKit types remain outside `runenui_core` and `runenui_runtime`, and the reference host does not become framework-owned loop authority. Reusable host/adapter packaging requires demonstrated second-consumer pressure rather than being inferred from the existence of the reference application.

## Core invariants

### Transient authoring, persistent runtime

`View`/`Element` values are owned transient descriptions derived from application state and consumed by reconciliation. Persistent identity, local widget state, lifecycle, focus/interaction state, work ownership, and publication authority remain in the mounted runtime.

### Distinct identities

Authored IDs/keys, mounted IDs, semantic IDs, work/trace sequences, and surface identities have separate meanings. A semantic ID is not a mounted-arena alias. Runtime-issued identities are runtime-local and must not be serialized or forged into live authority.

### One processing authority

Accepted application actions, routed commands/input, semantic actions after admission, effect/work transitions, timer/subscription events, and derived work converge through the runtime's canonical sequenced processing path. No public direct-dispatch or second event/action queue may silently bypass ordering/default/trace semantics.

### Independent semantics

Widgets contribute platform-neutral owner-local semantic descriptions. Runtime validates/reconciles them into independently allocated semantic lifetimes and a renderer-independent surface-scoped publication. Public semantic consumers receive semantic identity/content, not mounted routing authority.

Exact semantic action requests are admitted against current published semantic authority and then converge on the canonical runtime command/default path. No second accessibility callback engine exists.

### Staged publication

Surface publication follows a staged transaction with admission, read-only/staged planning, candidate-dependent final preflight, and commit. Recoverable refusal or terminal failure must not expose a partial new RunenUI-owned publication state.

Renderer-facing paint products, hit/input products, semantics, layout, and diagnostics remain distinct authorities even when committed together. Accepted M6 uses immutable `PaintScene`/`PaintPublication` and `HitTestScene` products; scene requirements derive from canonical paint content, while raster scale, base revision, and damage remain paint-publication metadata. Paint revision identity remains distinct from displayed input generation. Independent consumers can reconstruct complete deterministic scene snapshots from those public products without widget-kind, mounted/layout storage, private runtime, or hidden prior-scene authority.

## Current limitations

The current public surface is pre-1.0 and may change incompatibly when accepted architecture requires a clean cutover. Important missing production capabilities include:

- a reusable native host/application edge proven by a second real consumer and the M7 external-host closure path;
- broader resource/font/shaping integration beyond the bounded accepted M7 resource realization proof;
- multi-window lifecycle and supported platform-profile breadth;
- production text shaping, international layout, editing, clipboard, and complete editable-text integration;
- production responsive layout/style breadth and a complete standard control library.

M7 has accepted the first real wgpu renderer/resource edge, standalone winit host/native-input/presentation path, and AccessKit semantic adapter over the M6 neutral protocol. M7 remains incomplete until the native application showcase and external-host closure are accepted. Current maturity is summarized in [status](../status.md). Durable future sequencing belongs in the [roadmap](../roadmap.md). Permanent observable/proof requirements live under [conformance](../conformance/README.md).

Do not infer support from a target ADR, design document, type name, or roadmap entry alone. Code/tests establish current behavior; source/Rustdoc establishes the exact public Rust surface.
