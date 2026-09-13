# RunenUI Architecture

RunenUI is a host-neutral, renderer-neutral Rust UI framework. The durable ownership direction is:

```text
application state and actions
    -> transient typed View/Element descriptions
    -> keyed reconciliation
    -> persistent mounted runtime tree
    -> interaction / style / layout / semantics
         └── renderer-neutral logical text measurement/resources
    -> staged surface publication
         ├── renderer-facing products
         ├── hit/input products
         ├── semantic publication
         └── diagnostics
    -> host integration and renderer backend
```

The authored tree is transient reconciliation input. The mounted runtime tree is the persistent authority for runtime identity, widget-local state, lifecycle, invalidation, focus, interaction state, work ownership, layout orchestration, and publication coordination.

Semantic identity is independently runtime-issued and published through a renderer-independent semantic product. It is not a mounted-arena alias and must not be folded into renderer scene authority. Surface publication is staged and atomic: rejected or terminally failed publication must not expose a partial new RunenUI-owned product.

Production logical text follows the same authority model: `runenui_text` owns renderer-neutral font/shaping/line-breaking/artifact/resource computation, while runtime owns when those artifacts participate in mounted measurement and publication. The exact logical artifacts/resources measured by runtime are the ones retained for paint; the renderer owns only disposable SDF/MSDF/device realization.

Accepted M9A visual composition follows that same split. `runenui_core` owns neutral structural geometry, brush/stroke/image/group/style values and logical semantics; `runenui_runtime` owns common decoration, static presentation correlation, pre-group ordering, snapshot-local composition/effect groups, image mapping and conservative effect publication; `runenui_render_wgpu` owns only disposable tessellation, GPU composition, clip, gradient, text, image and ordinary-shadow realization. Renderer alpha, caches, target state, and mask algorithms do not become logical shape, ordering, resource, style, or effect-support authority.

## Workspace ownership

- `runenui_core` — host-neutral public application, authoring, geometry, style, layout, event, effect, identity, semantic, and renderer-neutral visual/composition protocol values.
- `runenui_text` — renderer-neutral production font-source policy, shaping/line breaking, text-specific constraints, immutable logical text artifacts, and shaped-resource bindings behind RunenUI-owned contracts.
- `runenui_runtime` — live mounted/semantic storage, reconciliation, routing, focus/input state, scheduling, style/text/layout orchestration, visual/composition publication, tracing, publication, and shutdown.
- `runenui_testing` — downstream deterministic testing ergonomics over ordinary public core/runtime contracts.
- concrete hosts, platform adapters, renderer backends, and product state remain outside those ownership boundaries; accepted edge implementations do not move their authority into core/runtime/text.

The workspace dependency and extraction rules are defined in [workspace structure](docs/architecture/workspace-structure.md).

## Current behavior and required contracts

Code and executable tests are the evidence for what the current implementation does. Accepted ADRs, architecture/design contracts, and conformance observations define what the implementation is required to do. A mismatch is a defect or requires an explicit reviewed contract revision; implementation never silently overrides accepted architecture.

Detailed current architecture is indexed under [docs/architecture](docs/architecture/README.md). Durable decisions live in [ADRs](docs/adr/). Permanent observable/proof contracts live under [conformance](docs/conformance/README.md). High-level dependency sequence lives in the [roadmap](docs/roadmap.md). Current accepted maturity is summarized in [status](docs/status.md).

Accepted future architecture becomes current API only after implementation and acceptance. M6 is accepted current behavior through retained publication, canonical renderer-neutral `PaintPublication`/`PaintScene`/`HitTestScene` ownership, composition/resource-reference/renderer-metadata/capability semantics, two independent deterministic consumers, public testing convergence, and proof-era paint/hit migration closure. M7 is accepted current edge behavior at proof maturity through the reusable wgpu renderer/resource implementation, real offscreen/readback and golden proof, standalone winit/native application paths with native input and presentation/recovery, the AccessKit semantic-publication/action adapter, and a separate winit-free downstream host-owned frame-loop proof over the same public runtime/publication/renderer/resource contracts. M8 is accepted complete at its production-foundation scope: M8A provides deterministic production style resolution and invalidation, M8B provides renderer-neutral international logical text and retained shaped-resource identity with renderer-owned SDF/MSDF realization, M8C provides runtime-owned private-Taffy Block/Flex/Grid layout and exact final-layout text feedback, and M8D proves the integrated path from available-space text measurement through retained artifact/resource identity into paint, semantic content/bounds, retained retry/raster-scale re-realization, and real-wgpu multiscript output without introducing a second layout, text, semantic, or renderer authority. M9A is accepted current static visual/composition behavior: RunenUI-owned structural shape/path/stroke/brush/image/group vocabulary, typed background/outline/shadow/opacity/static-presentation style facts, runtime-owned ordering/group/effect publication and alpha-independent shadow support, plus subordinate real-wgpu realization with private Lyon tessellation and disposable bounded shadow-mask work. M9B deterministic motion and M9C integrated visual-motion closure remain future accepted-target work, so M9 is not complete.

Exact public Rust signatures remain authoritative in source and Rustdoc; conceptual public ownership is summarized in the [public API contract](docs/architecture/public-api.md).

Live issue, branch, pull-request, head, CI-run, blocker, and pickup state belongs in GitHub and is deliberately absent from this document.
