# Current Status

> **Category: Current status**

This file is the single durable owner of RunenUI capability maturity. It describes accepted default-branch capability, not active implementation progress, branch state, pull-request state, or blocker state.

## Maturity vocabulary

- **absent** — no accepted implementation exists;
- **planned** — roadmap/accepted target contract exists, but implementation is absent;
- **proof** — bounded implementation demonstrates the architecture/conformance contract but is not production breadth;
- **partial** — meaningful implementation exists but required production behavior is incomplete;
- **usable** — intended profile works for real downstream use with documented limitations;
- **stable** — compatibility/support policy is deliberately committed and release-grade;
- **deferred** — intentionally outside the active production path;
- **archived** — historical only; not active authority.

No current subsystem is `stable`.

## Capability map

| Area | Maturity | Current accepted capability | Decisive limitation / durable next owner |
|---|---|---|---|
| Application model | proof | typed `UiApp` state/action/update and transient typed view authoring | broader ergonomic facade remains later production work |
| Mounted runtime | proof | persistent keyed generational mounted tree, state/lifecycle, invalidation, exact targeting | production breadth still depends on later subsystems |
| Effects/scheduling | proof | bounded FIFO/pump, tasks/timers/subscriptions/host requests, deterministic time, wake/redraw, terminal/shutdown | broader host/platform operational breadth remains later |
| Routed interaction | proof | pointer, focus scopes/navigation, keyboard, committed text, composition, automation, semantic commands, plus accepted loss-preserving native winit translation reused by the reference host and native Counter | production controls/editing remain later; supported platform breadth belongs to M13 |
| Trace/replay | proof | bounded canonical trace, deterministic export, optional sink, inert offline replay | production devtools/inspection UI remains later |
| Semantics/accessibility core | proof | independent semantic identity/tree/update/action ingress with deterministic public testing and an accepted native AccessKit projection/action round-trip | broader platform/accessibility profiles belong to M13 |
| Testing | usable | public deterministic headless harness over ordinary public runtime contracts, including latest public paint/hit publication inspection, exact input-context derivation, runtime convergence, semantics, trace, and replay | concrete backend/native-host assertions remain separate platform evidence |
| Styling | partial | accepted production host-neutral style environment with typed themes/recipes/ordered variants, deterministic property-local precedence and exact provenance/diagnostics, canonical hover/focus/active/disabled projection, explicit high-contrast/reduced-motion preferences, bounded foreground/typography inheritance, metric typography, brush-valued backgrounds, outline, ordered ordinary shadows, opacity, static presentation, and effect-driven retained invalidation | deterministic transitions/timelines and reduced-motion motion policy belong to M9B; external theme loading and broader material breadth remain later |
| Layout/measurement | partial | accepted runtime-owned production Block/Flex/Grid layout over exact mounted topology through private low-level Taffy algorithms, with normalized finite/unbounded sizing/min/max/auto/fill/grow/shrink, positioning/Overlay, box/gap behavior, bounded intrinsic/custom measurement, baselines, logical overflow/content/scroll extents, exact text-feedback convergence, and one final geometry authority shared by paint/hit/focus/semantic bounds | virtualization and native scrolling mechanics remain later work |
| Renderer-neutral paint/hit scenes | proof | complete accepted M6 publication protocol plus accepted M9A static visual/composition breadth: retained immutable paint/hit products; structural rect/rounded-rect/ellipse/path geometry; generic fills/strokes/brushes; image mapping/nine-slice; exact pre-group ordering; snapshot-local composition groups; conservative effect bounds; neutral resource identity; and alpha-independent ordinary-shadow support | deterministic motion publication belongs to M9B; integrated visual-motion closure belongs to M9C |
| Visual composition | partial | all ten `M9VIS-*` obligations are owner-accepted: RunenUI-owned shape/path/stroke/brush/image/group semantics, common node decoration and static presentation, runtime-owned composition/effect publication, ordered ordinary shadows with Euclidean signed spread and finite blur support, generic clips, explicit extension boundaries, and subordinate private Lyon/wgpu realization | `M9MOTION-*` and `M9INTEG-*` remain blocked; M9B/M9C are required before M9 is complete |
| Concrete renderer backend | proof | accepted reusable wgpu renderer/resource edge with real offscreen/native pixels, external-image provider/cache realization, retained shaped-text SDF/MSDF atlases, generic shape/stroke/gradient/image/clip/group realization, alpha-independent ordinary-shadow realization with bounded disposable masks, exact real-wgpu evidence, renderer observations, reconstruction after target/cache loss, and native presentation | deterministic motion semantics remain runtime/M9B authority; broader platform/device-loss breadth belongs to M13; intrinsic color-font rendering remains unsupported with explicit diagnostics |
| Native window/event-loop host | proof | accepted reusable winit translation edge plus standalone reference host and native Counter application, each with host-owned wake/pump/redraw, displayed-frame mapping, resize/raster-scale handling, native input, real wgpu presentation, and bounded target recovery | supported platform breadth belongs to M13 |
| External host embedding | proof | accepted winit-free downstream host proof with caller-owned submit/pump/redraw/publish/ack/render/present sequencing, retained-publication renderer retry, semantic-action next-frame proof, and complete `ResourceRef` provider identity over ordinary public core/runtime/renderer contracts | production engine/embedded-host and supported-platform breadth belongs to M13 |
| Native accessibility adapter | proof | accepted reusable AccessKit adapter over ordinary semantic publication with stable adapter-owned identity, exact delta/full-resync behavior, exact action translation, install-before-show ordering, proxy callbacks, and host-thread runtime ingress, exercised by both native hosts | broader platform profiles belong to M13 |
| Production text shaping/editing | partial | accepted renderer-neutral production font-source policy, international shaping/bidi/grapheme handling, line breaking/reflow, immutable logical artifacts and shaped-resource lifetime, exact final-layout text-state retention, measurement-to-paint resource continuity, semantic content/bounds correlation, and real wgpu SDF/MSDF realization of supported outlines | production editing/selection/clipboard remains M10; intrinsic color glyph rendering is later breadth |
| Standard control library | partial | proof-level built-in authoring widgets used by conformance/examples | production control behavior/recipes/accessibility breadth belongs to M11 |
| Multi-window/multi-surface host lifecycle | absent | one logical surface proof | M13 |
| Stable facade/release | absent | lower-level pre-1.0 crates only | M15 qualifies the first public `0.1.0`; `1.0.0` is the later compatibility/support stability gate |

## Milestone summary

M0–M5 form the accepted headless foundation: repository/tooling policy, typed core values, open widget/component architecture, mounted runtime/layout/style proofs, deterministic effects and routed interaction, trace/replay, semantic publication/action ingress, and public deterministic testing.

M6 is accepted complete at `proof` maturity: retained publication, canonical renderer-neutral paint/hit products, composition/resource/metadata/damage/capability semantics, independent-consumer proof, public testing convergence, and migration closure are accepted on the default branch. M7 is accepted complete at `proof` maturity: all twenty M7 conformance rows are owner-accepted, covering the reusable wgpu renderer/resource edge, reusable winit translation and AccessKit adapter edge, real offscreen/readback and golden evidence, standalone reference-host native input/presentation, the same Counter application exercised through deterministic headless and real native execution, and a separate winit-free downstream host-owned frame-loop proof over the accepted public runtime/publication/renderer/resource contracts.

M8 is accepted complete at its production-foundation scope: all thirty-three `M8STYLE-*`, `M8TEXT-*`, `M8LAYOUT-*`, and `M8INTEG-*` conformance rows are owner-accepted. M8A establishes production style resolution and invalidation; M8B establishes renderer-neutral international logical text and retained shaped-resource identity with renderer-owned SDF/MSDF realization; M8C establishes runtime-owned private-Taffy Block/Flex/Grid layout and exact final-layout text feedback; M8D closes the integrated production path by correlating Taffy available-space measurement, text reflow/artifact retention, measurement-to-paint shaped-resource identity, final semantic text/bounds, deterministic bundled-font public tests, retained retry/raster-scale re-realization, and real-wgpu multiscript evidence while removing replaced proof-era authorities.

M9 is **partial**. M9A is accepted at static visual/composition scope: all ten `M9VIS-01..10` rows are owner-accepted, covering normalized RunenUI-owned geometry/brush/stroke/image/group vocabulary, common node decoration/static presentation, runtime ordering/composition/effect publication, conservative effect bounds, alpha-independent ordinary-shadow support and painter order, generic clips, explicit extension boundaries, private subordinate Lyon adoption, real-wgpu realization/reconstruction, and bounded renderer-private mask allocation. The remaining fifteen `M9MOTION-01..10` and `M9INTEG-01..05` rows remain blocked; deterministic transitions/timelines/reduced-motion behavior belong to M9B and integrated visual-motion closure belongs to M9C. No M9 completion is implied by M9A acceptance.

See the [roadmap](roadmap.md) for durable sequencing and [conformance](conformance/README.md) for permanent observable/proof contracts.
