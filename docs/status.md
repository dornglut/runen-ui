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
| Styling | partial | accepted production host-neutral style environment with typed themes/recipes/ordered variants, deterministic property-local precedence and exact provenance/diagnostics, canonical hover/focus/active/disabled projection, explicit high-contrast/reduced-motion preferences, bounded foreground inheritance, effect-driven retained invalidation, and accepted metric typography integration | current broader property/layout breadth belongs to M8C; animation/composition breadth belongs to M9 |
| Layout/measurement | partial | proof-level linear layout remains runtime-owned while text nodes use accepted production logical text artifacts for exact measurement, reflow, baselines, and the same shaped resources later painted | production responsive sizing/block/flex/grid/intrinsic layout and exact available-space feedback belong to M8C/M8D |
| Renderer-neutral paint/hit scenes | proof | complete accepted M6 protocol: retained publication, canonical immutable paint/hit products, composition/resources/metadata/damage/capabilities, two independent deterministic consumers, downstream public-contract consumption, testing convergence, and proof-era authority migration closure | concrete production breadth remains later |
| Concrete renderer backend | proof | accepted reusable wgpu renderer/resource edge with real offscreen pixels, external-image provider/cache realization, exact golden/readback evidence, renderer observations, native presentation, and accepted renderer-owned per-glyph SDF/MSDF text atlas realization from retained logical resources | broader production/platform/device-loss breadth belongs to M13; intrinsic color-font rendering remains unsupported with explicit diagnostics |
| Native window/event-loop host | proof | accepted reusable winit translation edge plus standalone reference host and native Counter application, each with host-owned wake/pump/redraw, displayed-frame mapping, resize/raster-scale handling, native input, real wgpu presentation, and bounded target recovery | supported platform breadth belongs to M13 |
| External host embedding | proof | accepted winit-free downstream host proof with caller-owned submit/pump/redraw/publish/ack/render/present sequencing, retained-publication renderer retry, semantic-action next-frame proof, and complete `ResourceRef` provider identity over ordinary public core/runtime/renderer contracts | production engine/embedded-host and supported-platform breadth belongs to M13 |
| Native accessibility adapter | proof | accepted reusable AccessKit adapter over ordinary semantic publication with stable adapter-owned identity, exact delta/full-resync behavior, exact action translation, install-before-show ordering, proxy callbacks, and host-thread runtime ingress, exercised by both native hosts | broader platform profiles belong to M13 |
| Production text shaping/editing | partial | accepted renderer-neutral production font-source policy, international shaping/bidi/grapheme handling, line breaking/reflow, immutable logical artifacts and shaped-resource lifetime, with real wgpu SDF/MSDF realization of supported outlines | production editing/selection/clipboard remains M10; integrated production layout/text feedback closes through M8C/M8D; intrinsic color glyph rendering is later breadth |
| Standard control library | partial | proof-level built-in authoring widgets used by conformance/examples | production control behavior/recipes/accessibility breadth belongs to M11 |
| Multi-window/multi-surface host lifecycle | absent | one logical surface proof | M13 |
| Stable facade/release | absent | lower-level pre-1.0 crates only | M15 qualifies the first public `0.1.0`; `1.0.0` is the later compatibility/support stability gate |

## Milestone summary

M0–M5 form the accepted headless foundation: repository/tooling policy, typed core values, open widget/component architecture, mounted runtime/layout/style proofs, deterministic effects and routed interaction, trace/replay, semantic publication/action ingress, and public deterministic testing.

M6 is accepted complete at `proof` maturity: retained publication, canonical renderer-neutral paint/hit products, composition/resource/metadata/damage/capability semantics, independent-consumer proof, public testing convergence, and migration closure are accepted on the default branch. M7 is accepted complete at `proof` maturity: all twenty M7 conformance rows are owner-accepted, covering the reusable wgpu renderer/resource edge, reusable winit translation and AccessKit adapter edge, real offscreen/readback and golden evidence, standalone reference-host native input/presentation, the same Counter application exercised through deterministic headless and real native execution, and a separate winit-free downstream host-owned frame-loop proof over the accepted public runtime/publication/renderer/resource contracts. M8A is accepted at `partial` styling maturity: all seven `M8STYLE-01..07` rows are owner-accepted for the production style environment/resolution/state/preference/inheritance/invalidation mechanism. M8B is accepted at `partial` text maturity: all eleven `M8TEXT-01..11` rows are owner-accepted for renderer-neutral production logical text, exact measurement-to-paint shaped-resource continuity, retained logical resource lifetime, and renderer-owned SDF/MSDF realization with explicit unsupported intrinsic-format diagnostics. M8 remains incomplete; M8C production runtime layout/text feedback is next, followed by M8D integrated closure.

See the [roadmap](roadmap.md) for durable sequencing and [conformance](conformance/README.md) for permanent observable/proof contracts.
