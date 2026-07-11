# RunenUI Production Roadmap

> **Category: Current contract**

This roadmap is the gated execution authority from the current headless proof to a reviewed production release. A milestone is complete only when its behavioral exit criteria pass; types, documents, or isolated proofs are not completion.

## Status legend

| State | Meaning |
|---|---|
| `active` | Approved work currently in progress. |
| `queued` | Defined and next when dependencies pass. |
| `blocked` | Defined but cannot start until listed dependencies pass. |
| `deferred` | Deliberately later than the first production foundation or release. |
| `complete` | All exit criteria and required proofs pass. |

Historical foundations—typed application flow, immutable element descriptions, deterministic headless proofs, typed style resolution, explicit constraints, measurement-provider contracts, and aligned publication diagnostics—are retained inputs. They do not complete any production milestone by themselves.

## Non-negotiable sequencing

- Do not add broad controls before M2–M5 establish extensibility, mounted identity, events, semantics, and public testing.
- Do not implement renderer backends before M6 accepts the neutral paint and hit-test protocols.
- Do not implement interaction-state styling before mounted hover, pressed, focus, and disabled state exist.
- Do not implement editable text before the M4 event model, M5 semantics, and M8 text contracts are coherent.
- Do not manufacture target crates without independent ownership or dependency pressure.
- Do not make signals or observables a competing primary application-state model.

## M0 — Repository authority and governance reset

**Status:** `active`

**Goal:** Make repository documentation, active content, metadata, governance, and validation truthful for a pre-1.0 production-readiness program.

**Why now:** Stale skeleton documents, active historical material, `1.0.0` metadata, missing licensing/governance, and divergent validation undermine every later design and release claim.

**Included work:**

- **M0A — production authority documentation:** README, architecture framing, status map, feature/support matrix, M0–M12 roadmap, and documentation disposition.
- **M0B — archival/removal:** annotated legacy tag; remove `legacy/`, obsolete maps/plans, fake target API, completed incremental documents, and migrated audit backlog; add concise history/ADR records; keep normal context profiles free of legacy material.
- **M0C — release/governance baseline:** reset packages to `0.1.0`, disable publishing, add accurate metadata and license files, pin contributor toolchain/MSRV policy, add contribution/security/conduct/changelog/agent/release/API-stability guidance, and align local validation with CI.

**Explicit non-goals:** Any M1 API repair; mounted runtime; controls; renderer protocol/backend; layout expansion; production text; host implementation.

**Dependencies:** Clean current `master`, recoverable Git history, and the archival tag before legacy deletion.

**Required proofs/tests:** Full workspace format/test/Clippy/MSRV validation, `cargo validate`, Markdown relative-link check, context-profile checks, manifest metadata inspection, and critical stale-reference/diff review.

**Exit criteria:** README and canonical architecture are truthful; status and support coverage are complete; every document has a disposition; obsolete/duplicate docs and active `legacy/` are gone after archival; normal audits exclude legacy; packages are pre-1.0 and non-publishable; real licensing and governance exist; toolchain/MSRV/release/API policies are explicit; local validation and CI share one baseline; links and checks pass; no false implementation or production claim remains.

**Unblocks:** M1.

## M1 — Public API and core vocabulary repair

**Status:** `queued` after M0.

**Goal:** Remove prototype compatibility traps before more framework code depends on them.

**Why now:** Invalid floats/IDs, ambiguous naming, silent no-op methods, closed/exhaustive public shapes, tuple limits, and public generated-product constructors would multiply migration and correctness cost.

**Included work:** Numeric invariants and logical units; layout/style naming; validated IDs/keys and duplicate diagnostics; remove or implement dead token vocabulary; typed control-specific configuration; unlimited children; prelude reduction; protected generated products; public enum/trait semver strategy.

**Explicit non-goals:** Mounted reconciliation, custom widget implementation, new controls, renderer backends, or layout algorithm expansion.

**Dependencies:** M0 complete.

**Required proofs/tests:** Invalid-value tables/property tests; duplicate ID/key diagnostics; compile-time or behavioral tests for invalid control configuration; unlimited-child macro/builder proof; public visibility/API tests; migration of all examples and docs.

**Exit criteria:** Invalid values cannot silently enter normal public paths; ambiguous identity is diagnosed; invalid element configuration cannot silently no-op; child authoring has no fixed tuple ceiling; public API is deliberately pre-1.0 and generated products cannot be freely forged.

**Unblocks:** M2 and safer M3 implementation.

## M2 — Extensible view/widget/component architecture

**Status:** `blocked` by M1.

**Goal:** Let external crates and reusable components participate without modifying closed core enums.

**Why now:** Mounted identity, controls, semantics, layout, and rendering must be built on an open participant contract rather than hardcoded matches.

**Included work:** Public transient View/Element protocol; widget type identity and type erasure; external widget/control boundary; component action mapping; component expressions; lifecycle-capable interface; runtime-local state contract; custom layout, paint, semantics, diagnostics, and testing participation; macro direction.

**Explicit non-goals:** Broad built-in control library, production reconciliation implementation beyond the contract needed for proof, renderer backend, or facade crate.

**Dependencies:** M1; accepted ADR for View/Widget/type erasure coordinated with the M3 identity/storage design.

**Required proofs/tests:** An external test crate defines a custom control through public APIs; a child maps local actions to parent actions; the custom control participates in deterministic event, layout, paint-proof, semantic-proof, and diagnostic paths.

**Exit criteria:** Core enums are no longer the extension gate; component and widget concepts are distinct; public custom-widget and action-mapping contracts are coherent and tested.

**Unblocks:** M3 and future controls.

## M3 — Mounted runtime and reconciliation

**Status:** `blocked` by M1–M2.

**Goal:** Establish persistent runtime identity, lifecycle, state, and granular invalidation.

**Why now:** Focus, capture, editing, scrolling, animations, semantics, tasks, overlays, and safe targeting all require persistent generational identity.

**Included work:** Mounted node arena; generational IDs; keyed/type/position reconciliation; mount/update/unmount; duplicate key diagnostics; widget-local state; focus retention; hover/pressed/capture/scroll slots; semantic identity; dirty/invalidation phases; lifecycle resource ownership; stale-target rejection; multi-surface-ready identity.

**Explicit non-goals:** Full event routing/effects (M4), accessibility adapter (M5), production paint protocol (M6), production layout/text/control breadth.

**Dependencies:** M2 protocol and accepted reconciliation/storage ADR.

**Required proofs/tests:** Keyed reorder preserves local state; compatible rebuild retains identity and focus; removal runs lifecycle and cancels owned resources; stale IDs cannot address replacement nodes; duplicate keys are deterministic; invalidation affects only required phases.

**Exit criteria:** The authored tree is demonstrably transient and mounted state persistent; generational safety and lifecycle tests pass; unconditional focus clearing and preorder identity authority are removed.

**Unblocks:** M4–M7 and mounted control behavior.

## M4 — Events, effects, scheduling, and trace v2

**Status:** `blocked` by M3.

**Goal:** Provide one correct interaction pipeline and deterministic application-work runtime.

**Why now:** Controls, accessibility actions, text input, scrolling, hosts, and testing require consistent routing, commands, queues, effects, time, and observation.

**Included work:** Host-event normalization; capture/target/bubble; handled/default-action policy; pointer IDs/device kind/capture/cancellation; release-inside activation; focus scopes; keyboard commands; separate text/IME streams; action queue; effects/tasks/timers/subscriptions/host commands; cancellation; wake/redraw scheduling; deterministic executor; bounded structured trace and replay foundation.

**Explicit non-goals:** Platform-specific host implementation, full semantics adapter, production text editing, or renderer backend.

**Dependencies:** Mounted identity/lifecycle; accepted event and effects ADRs.

**Required proofs/tests:** Pointer, keyboard, accessibility-stub, automation, and programmatic activation converge on the same semantic command; capture/cancel/release cases; deterministic task/timer/subscription ordering and cancellation; bounded trace reconstructs event/action/effect/reconcile/publication order.

**Exit criteria:** One canonical event path remains; overlapping input-intent paths are removed; correct button activation passes; effects and scheduling are deterministic and lifecycle-bound; trace has sequence/generation/target facts and bounded retention.

**Unblocks:** M5, M8, M9, and M10 host integration.

## M5 — Semantics and deterministic public testing

**Status:** `blocked` by M3–M4.

**Goal:** Make renderer-independent accessibility semantics and framework-level testing first-class.

**Why now:** Every production control must ship with semantics, keyboard/accessibility behavior, and stable public tests rather than retrofit them later.

**Included work:** Semantic tree with stable IDs, roles, names, descriptions, values, states, relationships, actions, bounds, and text-range extensions; incremental semantic updates; AccessKit-neutral adapter foundation; public headless harness; synthetic input/actions; deterministic clock/tasks; semantic/layout/hit/paint assertions; replay foundation.

**Explicit non-goals:** Native platform accessibility bridge, production text ranges, full control library, renderer backend.

**Dependencies:** Mounted identity and canonical commands/effects.

**Required proofs/tests:** Counter and custom-widget proofs operate via semantic queries/actions; keyboard-only and accessibility-action tests; stable IDs across compatible updates; disabled/hidden/inert behavior; tests use public harness rather than private runtime internals.

**Exit criteria:** Semantic output is independent of rendering; public deterministic tests can drive and inspect the framework; AccessKit mapping seams are coherent; accessibility requirements are mandatory in later control gates.

**Unblocks:** M6, M9, M10, and accessible text integration.

## M6 — Renderer-neutral paint and hit-test scene protocol

**Status:** `blocked` by M3 and coordinated with M5.

**Goal:** Publish backend-neutral paint and hit-test products without widget semantics.

**Why now:** Backends and advanced interaction need stable scenes with explicit order, clips, transforms, resources, and generation identity.

**Included work:** Paint primitives; text/image/resource references; fills, borders/strokes, clips, transforms, opacity, stacking/layers, frame metadata, scale and damage facts; separate hit-test scene with shapes, visibility, inertness, pointer policy, clips/transforms/order; scene generations; deterministic snapshots; backend capabilities.

**Explicit non-goals:** Concrete desktop/SDF backend, production text shaping, full layout/styling expansion, or semantic widget kinds in renderer input.

**Dependencies:** Mounted generation identity; accepted render-protocol ADR; semantic separation coordinated with M5.

**Required proofs/tests:** Two independent deterministic consumers; custom backend proof renders without knowing `Button`; hit tests respect clips/transforms/visibility/order; scene snapshots are stable and generational targets reject stale input.

**Exit criteria:** `SurfaceNodeKind` is no longer the renderer protocol; paint, hit, semantics, layout, and diagnostics are distinct authoritative products; no backend-specific vocabulary leaks into public scenes.

**Unblocks:** M7 rendering integration and M10 backends.

## M7 — Production layout and styling

**Status:** `blocked` by M3–M6.

**Goal:** Support normal responsive applications, tools, scrolling, overlays, and stateful visual policy.

**Why now:** Production controls and apps need complete sizing, alignment, box, scroll, and style-state behavior on persistent nodes and neutral scenes.

**Included work:** Adopt-versus-build layout ADR; sizing/min/max/fill/shrink; flex and alignment; baselines and wrap; stack/absolute/overlay; full box model; clipping and scrolling; scroll extents; incremental layout; themes, recipes, variants, interaction state, user preferences, high contrast, and reduced motion.

**Explicit non-goals:** Unicode shaping/editing implementation, full controls, native host/backend, virtualization, or manufactured crate extraction.

**Dependencies:** Mounted interaction state, semantic/testing contracts, and scene protocol.

**Required proofs/tests:** Layout conformance edge cases; responsive settings app; scroll input/focus/semantics behavior; state-layer and resolution-precedence tests; two consumers before extraction; no generic-layout control-size constants.

**Exit criteria:** Control gallery/settings layouts require no ad hoc geometry; scrolling is input-, focus-, and semantics-aware; incremental invalidation passes; style precedence is inspectable; layout dependency choice is reviewed behind RunenUI contracts.

**Unblocks:** M8–M10 control and host breadth.

## M8 — Production text subsystem

**Status:** `blocked` by M4–M7.

**Goal:** Support internationalized display and editable text through a mature text stack.

**Why now:** Text is foundational to controls and accessibility but requires stable events, semantics, layout, resources, scheduling, and renderer scenes.

**Included work:** Reviewed text-stack ADR; font database/discovery/provider and fallback; shaping, script/language, bidi, line breaking, wrapping, alignment, baselines; glyph/resource caches; editing, selection, caret, clipboard, IME; semantic text ranges and mapping; deterministic fixtures.

**Explicit non-goals:** Hand-written Unicode shaping, platform-specific behavior in the core text model, or all advanced rich-text/editor features.

**Dependencies:** Separate text/IME events, semantics, production layout/style, paint resource protocol, and host provider seams.

**Required proofs/tests:** Multilingual scripts, emoji, combining marks, RTL/bidi, fallback, wrapping, baselines, selection/caret, IME flows, accessible ranges, deterministic headless fixtures, invalidation, and cache/resource budgets on desktop platforms.

**Exit criteria:** Deterministic scalar-count metrics cannot be selected accidentally for a production profile; display/edit text contracts pass conformance; ownership between text, host resources, renderer glyphs, and semantics is explicit.

**Unblocks:** Complete M9 controls and M10 desktop IME/clipboard proof.

## M9 — Standard control library

**Status:** `blocked` by M2–M8.

**Goal:** Provide coherent production controls built on public framework contracts.

**Why now:** Only after identity, events, semantics, scenes, layout/style, text, and testing exist can controls be complete rather than hardcoded primitive variants.

**Included work:** Label/text, button, checkbox, radio, toggle, slider, progress, text field, scroll container, list, menu, popover, tooltip, dialog, and tabs. Every control includes lifecycle state, canonical events/commands, semantics, style states, layout, keyboard operation, accessibility actions, and deterministic tests.

**Explicit non-goals:** Advanced tree/data-grid/editor controls, docking, or product-specific navigation frameworks.

**Dependencies:** M2–M8 gates.

**Required proofs/tests:** Complete control gallery; keyboard-only operation; semantic query/action coverage; pointer capture/cancellation; focus scopes and overlays; text-field IME/editing; themes/variants; third-party custom-control parity.

**Exit criteria:** No control-specific behavior remains embedded in generic tree indexing/layout; every required control passes interaction, semantic, accessibility, layout, style, and deterministic conformance.

**Unblocks:** M10 reference applications and M11 release candidates.

## M10 — Host and backend production profiles

**Status:** `blocked` by M4–M9.

**Goal:** Run real standalone desktop applications and embedded-host UI through common contracts.

**Why now:** Native integration and backends should prove stable framework protocols, not dictate them.

**Included work:** Host contract; reference desktop adapter; one conventional renderer backend; platform accessibility bridges; clipboard, IME, cursor, drag/drop; DPI/resize/safe areas; multi-window/surface lifecycle; resource providers; shutdown/device-loss behavior; external embedded-host adapter proof; optional SDF profile only after neutral/conventional proof.

**Explicit non-goals:** Mobile/web, simultaneous competing production backends, or Runenwerk/ECS assumptions in RunenUI.

**Dependencies:** Effects/event/semantic/scene/layout/text/control contracts; reviewed conventional-renderer and unsafe-code ADRs.

**Required proofs/tests:** Reference apps on Windows/macOS/Linux; DPI/resize/input/IME/accessibility/device-loss/shutdown smoke tests; multi-window lifecycle; packaging examples; embedded host owns window/frame loop and consumes the neutral protocol.

**Exit criteria:** Required desktop services and accessibility work across all three platforms; one supported conventional renderer exists; external embedding works without framework ownership leakage.

**Unblocks:** M11 production hardening.

## M11 — Production hardening and first stable release

**Status:** `blocked` by M0–M10.

**Goal:** Make support, compatibility, security, performance, and release claims enforceable, then deliberately release `1.0.0`.

**Why now:** Stability is a verified product property, not a version-number shortcut.

**Included work:** Windows/macOS/Linux CI; stable and MSRV policy enforcement; docs/doctests/examples; feature combinations; publish dry runs; dependency/license/security policy enforcement; API/semver checks; benchmarks/budgets; property/fuzz/stress tests; optional Miri/sanitizers; packaging; resource/memory tests; release automation; facade crate when justified; public API review.

**Explicit non-goals:** M12 editor/game/authoring breadth or lowering release gates to match missing behavior.

**Dependencies:** All required production profiles and M0–M10 exit criteria.

**Required proofs/tests:** Release-candidate runs of control gallery, settings/form, large list, text editor, overlays/dialogs, multi-window, embedded host, and keyboard/accessibility apps; documented budgets; cross-platform matrix; semver and supply-chain checks.

**Exit criteria:** No unresolved P0/P1 correctness defects; production profile/support matrix passes; performance budgets and compatibility policy are enforced; release checklist and artifacts pass; public API review approves `1.0.0`.

**Unblocks:** Stable release and post-v1 maintenance.

## M12 — Advanced editor, game, and authoring systems

**Status:** `deferred` until the production kernel is proven.

**Goal:** Build advanced application systems on the stable kernel.

**Why now:** These features are valuable but depend on almost every foundational contract and must not distort the first production release.

**Included work:** Virtualization; advanced list/tree/data-grid/editor controls; animation/time; overlays and advanced multi-surface systems; docking/workspaces; inspector/devtools; external source formats; hot reload/live preview; advanced replay; production SDF backend if not earlier; optional mobile/web profiles.

**Explicit non-goals:** Weakening M0–M11 requirements or moving product-specific state/host ownership into core.

**Dependencies:** Stable or sufficiently proven kernel capabilities from M1–M11; separate ADRs for animation and source/authoring systems.

**Required proofs/tests:** Large-data stress, persistence/migration, drag/overlay/multi-surface correctness, replay determinism, authoring diagnostics, and host/backend conformance appropriate to each slice.

**Exit criteria:** Defined per reviewed advanced slice; no M12 item is required to declare the first production kernel complete unless explicitly promoted through a reviewed release decision.

**Unblocks:** Additional product profiles and ecosystem growth.

## Primary milestone ownership

Every production capability has one primary owner even when it depends on earlier work:

| Capability family | Primary milestone |
|---|---|
| Repository truth, archival, metadata, governance, validation baseline | M0 |
| Core vocabulary and public API safety | M1 |
| Authoring/component/custom-widget protocol | M2 |
| Persistent identity, lifecycle, local state, invalidation | M3 |
| Events, effects, queues, scheduling, trace | M4 |
| Semantics, accessibility model, public deterministic testing | M5 |
| Paint/hit scenes and renderer-neutral protocol | M6 |
| Production layout, scrolling, themes, recipes, state styling | M7 |
| International and editable text | M8 |
| Standard controls | M9 |
| Native hosts, platform bridges, conventional backend, embedded proof | M10 |
| Cross-platform hardening, budgets, release, `1.0.0` | M11 |
| Advanced editor/game/authoring systems | M12 |

## Definition of roadmap completion

The roadmap reaches its first stable completion only when RunenUI has deterministic mounted headless execution, real applications on Windows/macOS/Linux, an external embedded-host proof, correct pointer/keyboard/text/IME/clipboard/accessibility behavior, production text and responsive layout, standard controls, one conventional backend, a neutral protocol suitable for SDF/engine consumption, public deterministic replay/testing, documented performance budgets, cross-platform validation, no unresolved P0/P1 architecture defects, and a reviewed `1.0.0` release.
