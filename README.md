# RunenUI

RunenUI targets host-neutral, renderer-neutral Rust UI for headless, standalone, and embedded application profiles. The repository is pre-1.0 and currently provides a deterministic headless framework foundation plus a proof-level real host/renderer/accessibility production spine; it is not yet a production-complete desktop UI stack.

## Current accepted foundation

The implemented foundation includes:

- typed application state/action/update and transient `View`/`Element` authoring;
- persistent keyed mounted runtime state with checked generational identity and lifecycle;
- validated logical geometry plus an accepted production style environment/cascade with metric typography integration and an accepted runtime-owned production Block/Flex/Grid layout path over exact mounted topology, alongside hit-test and renderer-facing publication products;
- normalized production sizing, positioning/Overlay, bounded intrinsic/custom measurement, baseline handling, clipping and inspectable logical overflow/content/scroll extents through private low-level Taffy algorithms rather than a second retained layout tree;
- accepted renderer-neutral production text through `runenui_text`: explicit deterministic/production font-source policy, international shaping/bidi/grapheme handling, line breaking/reflow, immutable measurement/artifact/resource facts, exact final-layout feedback, and scale-independent shaped-resource lifetime;
- exact integrated correlation from Taffy available-space measurement through retained text artifacts/resources into paint and semantic bounds/content, with deterministic bundled-font public tests and retained publication/raster-scale re-realization proof;
- accepted wgpu SDF/MSDF realization of those exact already-shaped outline resources through renderer-owned quality classes and atlas pages, with no shaped-text provider or hidden alpha fallback, including the responsive multiscript real-wgpu evidence corpus;
- accepted M9A renderer-neutral visual/composition vocabulary and publication: structural rectangle/rounded-rectangle/ellipse/path geometry, generic fills/strokes, solid/linear/radial brushes, image fit/crop/nine-slice mapping, typed outline/shadow/opacity/static-presentation style facts, snapshot-local composition groups, conservative effect bounds, and alpha-independent ordinary-shadow support;
- accepted M9B deterministic motion through the same style/layout/publication path: target-keyed transition policy, owner-local explicit timelines, runtime-owned live motion/declaration lifecycle, one monotonic candidate instant, deterministic easing/interpolation, explicit reduced-motion policy, differential invalidation/group lifetime, existing redraw/wake scheduling, atomic staged commit/retry, and canonical motion trace;
- accepted M9C integrated visual-motion closure over those same authorities: one sampled presentation transform correlates paint, hit, directional focus, semantics, and presentation-relative clips; public `ManualClock` proof observes deterministic timing/layout/text/semantic results through ordinary runtime contracts; canonical hover/focus/active facts drive the accepted style/transition path; and the real-wgpu path proves retained sampled-publication retry plus cache and fresh renderer/device reconstruction without resampling or resource rebinding;
- real-wgpu realization of the accepted M9 visual vocabulary through renderer-private disposable tessellation, gradient/clip/group/shadow resources and bounded mask work, while motion samples arrive only through immutable renderer-neutral publication and never move clock/timeline/style/interpolation authority into the renderer;
- bounded effects, tasks, timers, subscriptions, host requests, deterministic clocks, wake/redraw, explicit pumping, trace/export/replay;
- canonical routed pointer, keyboard, committed-text, IME, focus, automation, and semantic-command interaction;
- independent semantic identity/publication/action ingress;
- public deterministic headless application testing through `runenui_testing`;
- an accepted reusable wgpu renderer/resource edge with real offscreen/readback and native presentation proof;
- reusable winit input and AccessKit translation exercised by standalone native hosts;
- a separate winit-free downstream host proof that owns pump/publication/render/present sequencing through ordinary public contracts.

The renderer-neutral paint/hit scene protocol is complete through M6 at proof maturity, the M7 reference production spine is complete at proof maturity, M8 is accepted complete at its production style/layout/international-text foundation scope, and M9 is accepted complete at its visual/composition, deterministic-motion, and integrated-closure scope with all twenty-five `M9VIS-*`, `M9MOTION-*`, and `M9INTEG-*` obligations owner-accepted after reconciliation. Text editing, standard controls, virtualization, multi-window lifecycle, and supported platform breadth remain later roadmap outcomes.

See [current status](docs/status.md) for capability maturity and [roadmap](docs/roadmap.md) for durable sequencing.

## Workspace

```text
runenui_text        -> runenui_core
runenui_runtime     -> runenui_core + runenui_text
runenui_render_wgpu -> runenui_core + runenui_runtime + runenui_text
runenui_winit       -> runenui_core + runenui_runtime
runenui_testing     -> runenui_core + runenui_runtime
```

- `runenui_core` owns host-neutral public values and protocols, including normalized authored layout, bounded widget-measurement vocabulary, renderer-neutral visual/composition values, and authored deterministic motion descriptions.
- `runenui_text` owns renderer-neutral production font/shaping/line-breaking/logical-text resources behind RunenUI contracts.
- `runenui_runtime` owns live framework authority and orchestrates style, interaction, text measurement, production layout, live deterministic motion, correlated visual/hit/focus/semantic geometry, visual/composition publication, and aligned surface publication.
- `runenui_render_wgpu` is the reusable concrete renderer/resource edge over ordinary public paint publication; external images use the caller provider, retained logical shaped text is realized directly as renderer-owned SDF/MSDF state, and accepted M9 geometry/brush/clip/group/shadow behavior plus already-sampled motion products are realized only as disposable renderer state, including retained-publication retry and cache/device reconstruction.
- `runenui_winit` owns reusable winit input and AccessKit translation/projection, not a host loop or renderer.
- `runenui_testing` is a downstream public testing convenience layer over ordinary runtime contracts, including deterministic manual-time integration rather than a private expected-motion engine.
- `tests/external_host` is an unpublished winit-free downstream host conformance consumer over public core/runtime/renderer/resource contracts.
- `tests/external_renderer` is an unpublished downstream renderer-neutral conformance consumer over ordinary public core/runtime scene contracts.
- `xtask` owns repository validation tooling and has no framework dependency.

See [workspace structure](docs/architecture/workspace-structure.md) for the enforced ownership/dependency contract.

## Validation

```text
cargo validate
```

For intentional Rust edits, format first with:

```text
cargo +stable fmt --all
```

Focused tests and conformance proofs remain required for the active change. See [TESTING.md](TESTING.md) and [validation details](docs/tooling/validation.md).

## Documentation

- [Architecture](ARCHITECTURE.md)
- [Documentation index](docs/README.md)
- [Current status](docs/status.md)
- [Roadmap](docs/roadmap.md)
- [Public API contract](docs/architecture/public-api.md)
- [ADRs](docs/adr/)
- [Conformance](docs/conformance/README.md)
- [API stability](docs/api-stability.md)
- [Release policy](docs/release-policy.md)

Live work, blockers, pull-request state, exact heads, and CI evidence belong in GitHub rather than durable documentation.

## License

RunenUI is currently publicly licensed under [`GPL-3.0-only`](LICENSE). A separate commercial-license model may be available from copyright holder(s) with sufficient rights; see [`LICENSING.md`](LICENSING.md). Until reviewed inbound terms preserve commercial relicensing authority, external PRs contributing tracked repository content are not accepted. Issue reports, design discussion, reviews, and reproducible cases may still be accepted.
