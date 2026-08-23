# RunenUI Production Roadmap

This roadmap owns RunenUI's durable high-level outcome sequence, dependencies, major gates, and non-goals. It does not track branches, issues, pull requests, CI runs, blockers, or next actions. Current accepted capability maturity belongs in [status](status.md); detailed behavior/proof obligations belong in [conformance](conformance/README.md).

A milestone is complete only when its required behavior and proofs are accepted. Types or target documents alone do not complete a milestone.

The first public `0.1.0` is reserved for feature-complete supported product profiles. `1.0.0` is a later compatibility and support commitment over an already feature-complete product; it is not the point at which missing foundational UI subsystems are first added.

## Sequencing principles

- Establish extensibility, persistent identity, canonical interaction, semantics, and public testing before broad controls.
- Establish renderer-neutral paint/hit contracts before concrete renderer backends.
- After the renderer-neutral kernel, establish a thin real vertical production spine before broadening abstract subsystems in isolation.
- Treat style, layout, and text measurement as a coupled foundation with deliberate feedback loops rather than a strict waterfall.
- Introduce platform/resource/accessibility seams early enough that production text, controls, and engine embedding are validated against real consumers while they are designed.
- Introduce deterministic animation/time early and integrate animation progressively as concrete style/layout/paint properties gain production semantics.
- Make virtualization a pre-0.1 architectural capability, while keeping specialist products such as advanced data grids, full IDE docking suites, and visual designers outside the core release gate unless they expose a missing general primitive.
- Add inspectability, diagnostics, and performance evidence with each production subsystem; the final tooling/hardening milestones complete those capabilities rather than introducing them for the first time.
- Use explicit adopt-versus-build decisions for layout, text, accessibility, host, and renderer stacks before reimplementing mature external capability.
- Keep native host/platform/backend ownership at the edge of the neutral framework contracts.
- Extract crates only when real ownership/dependency/consumer pressure requires Cargo enforcement.
- Prefer clean pre-1.0 cutovers over compatibility paths for prototype APIs.

## Accepted foundation — M0 through M5

| Milestone | Accepted outcome |
|---|---|
| M0 — repository authority/governance reset | truthful pre-1.0 metadata, archival boundaries, licensing/governance, canonical validation, and removal of active legacy authority |
| M1 — public API/core vocabulary repair | validated values/identity, typed configuration, arity-free composition, constrained generated products, and repaired public-surface semantics |
| M2 — extensible view/widget/component architecture | open transient `View`/`Element` and state-aware widget participation with downstream conformance rather than closed core enums |
| M3 — mounted runtime/reconciliation | persistent generational mounted identity, lifecycle/state, keyed reconciliation, stale-target safety, focus retention, and invalidation ownership |
| M4 — events/effects/scheduling/trace | one queued routed interaction path, pointer/focus/keyboard/text/IME behavior, deterministic application work/scheduling, bounded tracing/export, and inert replay |
| M5 — semantics/public deterministic testing | independent semantic identity/publication/actions and a downstream public deterministic testing harness over ordinary runtime contracts |

These milestones remain durable inputs to successor work. Historical delivery chronology belongs in Git history and repository history records, not this roadmap.

## M6 — Renderer-neutral paint and hit-test scene protocol

**Goal:** Publish backend-neutral immutable paint and hit-test products without semantic widget or backend vocabulary.

**Depends on:** mounted/runtime identity, routed input, semantics, deterministic testing, and the accepted renderer-neutral scene architecture/conformance contract.

**Required outcome:**

- persistent/shared retained publication products avoid whole-surface deep copies for narrow unchanged phases while preserving staged publication atomicity;
- canonical paint and hit-test scene products have explicit ownership distinct from layout, semantics, and diagnostics;
- paint primitives/resources, transforms, clips, opacity, layer/order, scale, revision, and damage semantics are deterministic and backend-neutral;
- hit regions/policies, displayed-generation membership, order, transforms, clips, and stale-input behavior are explicit and tested;
- at least two independent consumers prove the scene contract without widget-type knowledge;
- proof-era renderer/hit authorities are removed cleanly when their production replacements land.

**Non-goals:** concrete production renderer backend, production text shaping, broad production layout/style, or semantic widget kinds in renderer input.

**Exit:** the renderer-neutral scene contract is implemented and accepted through its permanent conformance obligations, with no competing paint/hit authority.

## M7 — Reference production spine

**Goal:** Prove the neutral kernel through one real end-to-end production path before broad UI subsystems accumulate untested platform or renderer assumptions.

**Depends on:** M6 scene ownership and the accepted host-neutral runtime/semantic contracts.

**Required outcome:** reviewed host/renderer/accessibility/resource adopt-versus-build decisions; one real window/event-loop integration; one conventional renderer consumer of ordinary `PaintPublication`; real raster-scale/resize/redraw handling; image and baseline font/resource realization; one real accessibility adapter path over ordinary semantic publication; screenshot/golden rendering proof; instrumentation hooks for scene/revision/damage observation; external host-controlled frame-loop proof suitable for real-time/game embedding.

**Non-goals:** complete cross-platform support matrix, complete controls, production text editing, broad visual effects, or making the reference host/renderer authoritative over neutral framework behavior.

**Exit:** real pixels, resources, native input/accessibility, and an external host-owned loop exercise the public neutral contracts without widget-type knowledge or ownership leakage.

## M8 — Production style, layout, and text foundation

**Goal:** Establish the mutually dependent property, measurement, layout, and international text foundations required by normal responsive applications.

**Depends on:** M7 real production spine, mounted interaction state, semantics/testing, and M6 scene/resource contracts.

**Required outcome:** reviewed layout and text-stack adopt-versus-build decisions; typed style properties/tokens/precedence, themes, recipes/variants, interaction state, user preferences, high contrast, and reduced motion; production sizing/min/max/fill/shrink, flex, grid, block/flow where justified, baseline/wrap, stack/absolute/overlay, box model, clipping, scroll extents, intrinsic/custom measurement, and incremental layout; font discovery/provider/fallback, shaping, script/language, bidi, line breaking, wrapping, alignment/baselines, styled spans, and deterministic text fixtures; explicit feedback loop between available layout constraints and text measurement; inspectable invalidation/precedence/measurement behavior.

**Non-goals:** complete editable-text behavior, complete standard controls, broad animation integration, complete platform matrix, or hand-written general Unicode shaping where a mature stack is adopted.

**Exit:** representative responsive and text-heavy layouts require no ad hoc geometry or proof metrics; production text measurement participates correctly in layout and scrolling under real rendering.

## M9 — Visual composition and animation

**Goal:** Provide the normal visual vocabulary and deterministic motion model expected by production desktop and game UI without coupling widgets to a concrete renderer.

**Depends on:** M6 scene protocol, M7 renderer proof, M8 production style/layout/text properties, and M4 deterministic time/scheduling.

**Required outcome:** production visual vocabulary for common shapes/strokes, images, text, gradients, image fit/crop and scalable/nine-slice imagery, clipping, group opacity/composition, and ordinary shadows; a deliberate renderer-neutral extension boundary for richer vector/effect/offscreen composition needs; deterministic animation clock/timeline/interpolation/easing/cancellation/completion; property-transition integration with explicit invalidation classification across paint/hit/layout/text; reduced-motion policy integrated with animation behavior.

**Non-goals:** implementing every renderer filter/blend/shader feature, a visual animation editor, or making backend-specific effects part of widget semantics.

**Exit:** common application/game visuals and transitions are expressible through public neutral contracts, and animation cannot silently bypass invalidation, semantics, hit testing, or reduced-motion policy.

## M10 — Editing and interaction services

**Goal:** Complete production editable-text and interaction services before the standard control library depends on them.

**Depends on:** M4 canonical interaction, M8 production text/layout, M9 visual/animation properties, and progressively expanded M7 platform seams.

**Required outcome:** caret, selection, grapheme/word/line navigation, clipboard, IME/composition ranges, undo/redo editing substrate, semantic text ranges/mapping, pointer text selection, editing commands, wheel/scroll interaction, pointer capture, drag/drop, cursor policy, controller-command normalization, and the justified touch/multi-pointer/gesture baseline for supported 0.1 profiles; deterministic and real-platform proof for platform-sensitive behavior.

**Non-goals:** product-specific editing models, source-code editor semantics, or native platform behavior leaking into the core text model.

**Exit:** editable text and reusable interaction services are production-capable without hidden control-specific runtime paths.

## M11 — Standard control library

**Goal:** Provide production standard controls entirely on public framework contracts.

**Depends on:** M2–M10 foundations.

**Required outcome:** label/text, button, checkbox, radio, toggle, slider, progress, text field, scroll container, tabs, menus, popovers, tooltips, dialogs, and ordinary collection-facing controls with coherent lifecycle state, semantics/accessibility actions, style states, animation/transitions, layout, keyboard/controller behavior, focus/navigation, and deterministic plus real-renderer tests.

**Non-goals:** advanced virtualized tree/data-grid/editor controls, full docking/workspace products, or product-specific navigation frameworks.

**Exit:** no standard control relies on hidden generic-runtime special cases; downstream controls can achieve the same behavior/semantics/style/layout contracts through public APIs.

## M12 — Virtualization and scalable collections

**Goal:** Make large data-backed UI an architectural capability without requiring product-specific heavyweight controls in core.

**Depends on:** mounted identity/lifecycle, scrolling/layout, focus/semantics, controls, and deterministic testing.

**Required outcome:** stable data-item identity, viewport-driven realization, recycling/reuse rules, variable-size item handling, scroll anchoring, selection/focus retention, semantic publication for virtualized content, offscreen lifecycle rules, deterministic testing/inspection, and representative virtual list plus tree capability through public contracts.

**Non-goals:** full spreadsheet/data-grid suites, source-code editors, asset-browser products, or retaining application state inside recyclable delegates.

**Exit:** large collections do not require mounting the full logical data set, and virtualization does not weaken identity, focus, semantics, or external application-state ownership.

## M13 — Production platform and engine profiles

**Goal:** Complete the supported 0.1 execution profiles across real desktop platforms and external real-time hosts.

**Depends on:** M7–M12 production behavior and reviewed platform/backend decisions.

**Required outcome:** supported Windows/macOS/Linux desktop integrations; Linux/SteamOS-compatible real-time profile where the supported stack permits it; complete DPI/scale, resize, cursor, clipboard, IME, accessibility, drag/drop, activation/focus, multi-window/multi-surface, resource-provider/realization, shutdown, renderer recovery/device-loss, and raw-controller translation behavior; external embedded-host proof; engine-owned main-loop/render/resource proof with RunenUI retaining only UI runtime/behavior authority.

**Non-goals:** assuming Runenwerk/ECS ownership, putting native types into core/runtime behavior, mandatory mobile/web profiles, or requiring one renderer to be the only supported future renderer architecture.

**Exit:** headless, standalone desktop, embedded application, and real-time/game profiles operate through the same neutral contracts without ownership leakage across supported platforms.

## M14 — Tooling and performance completion

**Goal:** Turn the inspection/performance hooks accumulated throughout production work into a coherent developer and performance toolchain.

**Depends on:** inspectable state from M6–M13 and real production consumers.

**Required outcome:** mounted/style/layout/text/animation/focus/semantic/paint/hit/resource/publication inspection; dirty/invalidation visualization; scene capture and replay-oriented diagnostics; repaint/layout/virtualization observation; profiler/timing surfaces; renderer/layout/text/control/virtualization benchmarks and enforced budgets; golden-test workflow and actionable diagnostics suitable for downstream custom widgets and embedded hosts.

**Non-goals:** a full visual UI designer, complete IDE product, or making tooling a second runtime authority.

**Exit:** production behavior and performance can be diagnosed through ordinary truthful framework products without private mutation or duplicated expected-runtime models.

## M15 — Feature-complete 0.1 qualification

**Goal:** Qualify the complete supported product profiles and deliberately release the first public `0.1.0`.

**Depends on:** M0–M14.

**Required outcome:** cross-platform CI; MSRV/platform policy enforcement; complete docs/examples; feature-combination and publish-dry-run checks; dependency/license/security enforcement; API/semver checks appropriate to pre-1.0 evolution; property/fuzz/stress testing; relevant Miri/sanitizer coverage; packaging/release automation; no unresolved release-blocking correctness or budget defects; representative dogfood applications covering desktop/settings UI, real-time/game UI, tool/editor UI, and text-heavy/editing UI using public contracts; final review that no foundational capability required by the declared 0.1 profiles remains intentionally deferred.

**Non-goals:** stabilizing every public API permanently, requiring specialist products such as advanced data grids/docking suites/visual designers, or adding optional mobile/web profiles merely to increase breadth.

**Exit:** `0.1.0` is feature-complete for the declared supported profiles and can be published deliberately; later `0.x` releases refine compatibility, ergonomics, performance, and supported breadth without using 0.1 as an unfinished-foundation preview.

## Stable release — 1.0

**Goal:** Turn the already feature-complete product into a deliberate long-term compatibility and support commitment.

**Depends on:** successful 0.x use, real downstream applications, compatibility evidence, and an explicit stable API/support decision.

**Required outcome:** reviewed stable public API and semver strategy; documented compatibility/deprecation policy; supported platform/MSRV policy; release/support expectations; migration policy; sustained performance/security/license/release enforcement; no unresolved stable-release blockers.

**Exit:** `1.0.0` represents deliberate compatibility and support stability over mature product capability, not first availability of missing foundational UI systems.

## Primary capability ownership

| Capability family | Primary milestone |
|---|---|
| Repository truth/governance/validation baseline | M0 |
| Core vocabulary and public API safety | M1 |
| Authoring/component/custom-widget protocol | M2 |
| Persistent identity/lifecycle/state/invalidation | M3 |
| Events/effects/queues/scheduling/trace/navigation | M4 |
| Semantics/accessibility model/public deterministic testing | M5 |
| Renderer-neutral paint/hit scenes | M6 |
| Real host/renderer/resource/accessibility reference spine | M7 |
| Production style/layout/international text foundation | M8 |
| Visual composition and deterministic animation | M9 |
| Editable text and reusable interaction services | M10 |
| Standard production controls | M11 |
| Virtualization/scalable collections | M12 |
| Production desktop/embedded/real-time platform profiles | M13 |
| Inspection/profiling/performance tooling | M14 |
| Feature-complete public 0.1 qualification/release | M15 |
| Stable API/support commitment | 1.0 |

The first public roadmap outcome requires deterministic headless execution, supported desktop applications and external real-time/embedded hosting, production input/text/accessibility/layout/style/animation/controls/virtualization, one supported conventional renderer, public deterministic testing/replay, inspectability and documented budgets, cross-platform validation, and representative real applications. Live execution order and feedback loops within an accepted milestone belong to its GitHub work, not this document.
