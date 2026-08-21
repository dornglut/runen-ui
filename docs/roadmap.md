# RunenUI Production Roadmap

This roadmap owns RunenUI's durable high-level outcome sequence, dependencies, major gates, and non-goals. It does not track branches, issues, pull requests, CI runs, blockers, or next actions. Current accepted capability maturity belongs in [status](status.md); detailed behavior/proof obligations belong in [conformance](conformance/README.md).

A milestone is complete only when its required behavior and proofs are accepted. Types or target documents alone do not complete a milestone.

## Sequencing principles

- Establish extensibility, persistent identity, canonical interaction, semantics, and public testing before broad controls.
- Establish renderer-neutral paint/hit contracts before concrete renderer backends.
- Establish production layout/style before production text and controls depend on them.
- Establish text shaping/editing before claiming complete production text controls.
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

## M7 — Production layout and styling

**Goal:** Support normal responsive applications, tools, scrolling, overlays, and stateful visual policy.

**Depends on:** persistent mounted interaction state, semantic/testing contracts, and M6 scene ownership.

**Required outcome:** reviewed adopt-versus-build layout decision; production sizing/min/max/fill/shrink, flex/alignment, baseline/wrap, stack/absolute/overlay, box model, clipping/scrolling, scroll extents, incremental layout; themes, recipes, variants, interaction state, user preferences, high contrast, and reduced motion.

**Non-goals:** production Unicode shaping/editing, complete controls, native host/backend, virtualization, or crate extraction without demonstrated need.

**Exit:** representative settings/control-gallery layouts require no ad hoc geometry; scrolling integrates input/focus/semantics; layout/style invalidation and precedence are inspectable and tested.

## M8 — Production text subsystem

**Goal:** Provide internationalized display and editable text on a mature text stack.

**Depends on:** events/IME, semantics, production layout/style, paint resource contracts, and host provider seams.

**Required outcome:** reviewed text-stack decision; font discovery/provider/fallback; shaping, script/language, bidi, line breaking, wrapping, alignment/baselines; glyph/resource caching; editing, selection, caret, clipboard, IME; semantic text ranges/mapping; deterministic fixtures.

**Non-goals:** hand-written general Unicode shaping or platform-specific behavior inside the core text model.

**Exit:** deterministic proof metrics cannot be selected accidentally for production; display/edit contracts and ownership across text/resources/rendering/semantics are explicit and tested.

## M9 — Standard control library

**Goal:** Provide production controls entirely on the public framework contracts.

**Depends on:** M2–M8 foundations.

**Required outcome:** label/text, button, checkbox, radio, toggle, slider, progress, text field, scroll container, list, menu, popover, tooltip, dialog, and tabs with coherent lifecycle state, interaction/defaults, semantics, style states, layout, keyboard/controller behavior where applicable, accessibility actions, and deterministic tests.

**Non-goals:** advanced tree/data-grid/editor controls, docking, or product-specific navigation frameworks.

**Exit:** no standard control relies on hidden generic-runtime special cases; third-party controls can achieve the same contracts through public APIs.

## M10 — Host and backend production profiles

**Goal:** Run real standalone desktop applications and embedded-host UI through common neutral contracts.

**Depends on:** accepted effects/events/semantics/scenes/layout/text/controls plus reviewed platform/backend decisions.

**Required outcome:** host contract; reference desktop integration for Windows/macOS/Linux; one conventional renderer backend; platform accessibility; clipboard/IME/cursor/drag-drop; DPI/resize/safe-area handling; multi-window/surface lifecycle; resource providers; shutdown/device-loss behavior; host-owned raw controller lifecycle/translation/normalization into neutral UI commands; external embedded-host proof. An optional SDF renderer remains subordinate to the neutral/conventional proof.

**Non-goals:** putting native window/controller/accessibility types into core/runtime behavior or assuming Runenwerk/ECS ownership in RunenUI.

**Exit:** required desktop services work across supported platforms, one conventional backend is supported, and external embedding works without ownership leakage.

## M11 — Production hardening and first stable release

**Goal:** Make support, compatibility, security, performance, and release claims enforceable, then deliberately release `1.0.0`.

**Depends on:** all required production profiles through M10.

**Required outcome:** cross-platform CI; stable/MSRV policy enforcement; complete docs/examples; feature-combination and publish-dry-run checks; dependency/license/security enforcement; API/semver checks; benchmarks/budgets; property/fuzz/stress testing; relevant Miri/sanitizer coverage; packaging/release automation; facade API only if justified; final public API review.

**Non-goals:** weakening release gates to accommodate missing M12 features.

**Exit:** no unresolved release-blocking correctness defects, production profiles and budgets pass, compatibility policy is enforced, and release artifacts/public API receive explicit release acceptance.

## M12 — Advanced editor, game, and authoring systems

**Goal:** Build advanced application systems on the proven kernel.

**Depends on:** stable or sufficiently proven M1–M11 capabilities and separate decisions for each major subsystem.

**Candidate scope:** virtualization; advanced list/tree/data-grid/editor controls; animation/time; advanced overlays/multi-surface systems; docking/workspaces; inspector/devtools; external source formats; hot reload/live preview; advanced replay; optional additional renderer profiles; mobile/web profiles when justified.

**Non-goals:** weakening earlier foundation or moving product-specific state/host behavior into the kernel.

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
| Production layout/scrolling/themes/state styling | M7 |
| International/editable text | M8 |
| Standard controls | M9 |
| Native hosts/platform bridges/backend/raw controller translation | M10 |
| Cross-platform hardening/budgets/release | M11 |
| Advanced editor/game/authoring systems | M12 |

The first stable roadmap outcome requires deterministic headless execution, supported desktop applications and external embedding, production input/text/accessibility/layout/style/controls, one conventional renderer backend, public deterministic testing/replay, documented budgets, cross-platform validation, and a reviewed stable release. Live execution order within an accepted milestone belongs to its GitHub work, not this document.
