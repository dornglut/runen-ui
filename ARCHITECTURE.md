# RunenUI Architecture

RunenUI is a host-neutral, renderer-neutral Rust UI framework. The durable ownership direction is:

```text
application state and actions
    -> transient typed View/Element descriptions
    -> keyed reconciliation
    -> persistent mounted runtime tree
    -> interaction / style / layout / semantics
         ├── renderer-neutral logical text measurement/resources
         └── runtime-owned deterministic motion sampling
    -> staged surface publication
         ├── renderer-facing products
         ├── hit/input products
         ├── semantic publication
         └── diagnostics
    -> host integration and renderer backend
```

The authored tree is transient reconciliation input. The mounted runtime tree is the persistent authority for runtime identity, widget-local state, lifecycle, invalidation, focus, interaction state, work ownership, layout orchestration, and publication coordination.

Semantic identity is independently runtime-issued and published through a renderer-independent semantic product. It is not a mounted-arena alias and must not be folded into renderer scene authority. Surface publication is staged and atomic: rejected or terminally failed publication must not expose a partial new RunenUI-owned product.

Production logical text follows the same authority model: `runenui_core` owns exact application document/revision-scoped coordinate values, and `runenui_text` owns renderer-neutral font/shaping/line-breaking/artifact/resource computation plus immutable caret/preedit mapping over the exact retained private layout. Runtime owns when text artifacts participate in mounted measurement and publication; any later mounted use or publication of caret maps must remain under that same runtime authority. The exact logical artifacts/resources measured by runtime are the ones retained for paint; the renderer owns only disposable SDF/MSDF/device realization.

Accepted M9A visual composition follows that same split. `runenui_core` owns neutral structural geometry, brush/stroke/image/group/style values and logical semantics; `runenui_runtime` owns common decoration, static presentation correlation, pre-group ordering, snapshot-local composition/effect groups, image mapping and conservative effect publication; `runenui_render_wgpu` owns only disposable tessellation, GPU composition, clip, gradient, text, image and ordinary-shadow realization. Renderer alpha, caches, target state, and mask algorithms do not become logical shape, ordering, resource, style, or effect-support authority.

Accepted M9B motion extends the same path rather than creating a parallel animation engine. `runenui_core` owns the host-neutral authored motion vocabulary and pure deterministic sampling/interpolation rules; `runenui_runtime` owns all live transition/timeline/declaration state, exact mounted-generation reconciliation, one monotonic candidate instant, reduced-motion and mandatory-preference decisions, differential invalidation/group lifetime, redraw/wake demand, canonical motion trace, and atomic commit with the rest of surface publication. Renderers consume only the already-sampled immutable publication and never own clock, timeline, easing, interpolation, lifecycle, preference, or completion authority.

Accepted M9C closes the integrated path without adding another authority. One runtime-sampled presentation transform is correlated through paint, physical hit testing, directional-focus geometry, semantic bounds, and presentation-relative clips while retained layout remains the structural authority; singular transforms preserve empty physical coverage rather than stale-layout fallback. Public deterministic tests advance the ordinary runtime clock explicitly, canonical hover/focus/active facts drive the existing style/transition path, and real-wgpu consumes retained already-sampled publications through retry plus disposable cache and fresh renderer/device reconstruction without resampling, resource rebinding, or renderer-owned motion state.

Accepted M10C transactional editing extends the retained text path without creating a framework-owned document. Application state remains authoritative for document contents, revisions, validation, persistence, sensitivity, and durable undo history. `runenui_core` owns host-neutral editable contributions, opaque edit intents, immediate update resolutions, inverse/grouping hints, and editable semantic values. `runenui_runtime` alone owns bounded mounted editing-session lifetimes, provisional predecessor chains, selection/preedit state, routed editing defaults, exact post-update reconciliation, redacted trace, and retirement. Platform adapters project accepted semantics and translate neutral actions; they do not own an editor buffer, edit queue, document mutation, or editing-session authority.

## Workspace ownership

- `runenui_core` — host-neutral public application, authoring, geometry, style, layout, event, effect, identity, semantic, application-owned transactional-editing, renderer-neutral visual/composition, and deterministic motion description/sampling protocol values.
- `runenui_text` — renderer-neutral production font-source policy, shaping/line breaking, text-specific constraints, immutable logical text artifacts, shaped-resource bindings, and retained-layout caret/preedit maps behind RunenUI-owned contracts.
- `runenui_runtime` — live mounted/semantic/editing-session storage, reconciliation, routing, focus/input/edit-chain state, scheduling, style/text/layout orchestration, deterministic motion lifecycle/sampling, correlated visual/hit/focus/semantic geometry, visual/composition publication, tracing, publication, and shutdown.
- `runenui_testing` — downstream deterministic testing ergonomics over ordinary public core/runtime contracts, including explicit manual-time integrated proof without private runtime authority.
- concrete hosts, platform adapters, renderer backends, and product state remain outside those ownership boundaries; accepted edge implementations do not move their authority into core/runtime/text.

The workspace dependency and extraction rules are defined in [workspace structure](docs/architecture/workspace-structure.md).

## Current behavior and required contracts

Code and executable tests are the evidence for what the current implementation does. Accepted ADRs, architecture/design contracts, and conformance observations define what the implementation is required to do. A mismatch is a defect or requires an explicit reviewed contract revision; implementation never silently overrides accepted architecture.

Detailed current architecture is indexed under [docs/architecture](docs/architecture/README.md). Durable decisions live in [ADRs](docs/adr/). Permanent observable/proof contracts live under [conformance](docs/conformance/README.md). High-level dependency sequence lives in the [roadmap](docs/roadmap.md). Current accepted maturity is summarized in [status](docs/status.md).

Accepted future architecture becomes current API only after implementation and acceptance. M6 is accepted current behavior through retained publication, canonical renderer-neutral `PaintPublication`/`PaintScene`/`HitTestScene` ownership, composition/resource-reference/renderer-metadata/capability semantics, two independent deterministic consumers, public testing convergence, and proof-era paint/hit migration closure. M7 is accepted current edge behavior at proof maturity through the reusable wgpu renderer/resource implementation, real offscreen/readback and golden proof, standalone winit/native application paths with native input and presentation/recovery, the AccessKit semantic-publication/action adapter, and a separate winit-free downstream host-owned frame-loop proof over the same public runtime/publication/renderer/resource contracts. M8 is accepted complete at its production-foundation scope: M8A provides deterministic production style resolution and invalidation, M8B provides renderer-neutral international logical text and retained shaped-resource identity with renderer-owned SDF/MSDF realization, M8C provides runtime-owned private-Taffy Block/Flex/Grid layout and exact final-layout text feedback, and M8D proves the integrated path from available-space text measurement through retained artifact/resource identity into paint, semantic content/bounds, retained retry/raster-scale re-realization, and real-wgpu multiscript output without introducing a second layout, text, semantic, or renderer authority. M9 is accepted complete at its production visual/motion scope: M9A establishes RunenUI-owned structural shape/path/stroke/brush/image/group vocabulary, common visual style facts, runtime ordering/composition/effect publication, and subordinate real-wgpu realization; M9B establishes target-keyed transitions and owner-local timelines with exact runtime sampling/lifecycle, reduced-motion policy, differential invalidation/group lifetime, redraw/wake integration, atomic publication/retry correlation, and canonical motion trace; M9C proves the final cross-authority correlation across transformed paint/hit/focus/semantics/clips, ordinary public manual-time integration, canonical interaction-driven transitions, real-wgpu retained retry/cache/device re-realization, and clean single-path authority closure. All twenty-five permanent M9 conformance rows are owner-accepted. M10B and M10C are accepted current behavior through RunenUI-owned document/revision coordinates, one correlated immutable caret/preedit map over the retained production text layout, application-owned edit transactions, bounded runtime editing sessions/chains, and editable semantic/native projection; later M10 framework services, interactions, and integrated closure remain future architecture.

Exact public Rust signatures remain authoritative in source and Rustdoc; conceptual public ownership is summarized in the [public API contract](docs/architecture/public-api.md).

Live issue, branch, pull-request, head, CI-run, blocker, and pickup state belongs in GitHub and is deliberately absent from this document.
