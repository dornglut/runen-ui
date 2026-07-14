# Changelog

## Unreleased

### Changed

- Revised proposed M4 routed event and semantic-command architecture with
  exact core/runtime ownership, safe namespace-based opaque identities, the
  normative event-family policy, retained displayed-generation surface input,
  observable target/current-target/phase facts, immutable routes, non-reentrant
  propagation, exact transition/output ordering, pointer identity/capture,
  deterministic multi-pointer geometry revalidation, integrity-only terminal
  pointer cleanup for unavailable snapshots, focus scopes, exact no-action
  defaults for unconsumed route-only commands, separate keyboard/text/IME streams,
  semantic `on_activate`, and release-inside activation.
- Revised proposed M4 deterministic action/effect scheduling with
  `initial_effects`, two-argument no-effects updates, state-derived application
  subscriptions, dedicated complete-set mounted subscription declarations,
  commit-bound owner-local keyed cancellation/replacement, lifecycle-owned
  local/send tasks with one-attempt terminal executor refusal, exact readiness
  checkpoints and four pump budgets, timers, an exact response-kind-validated
  host protocol, configured saturation outcomes, race-free wake/redraw
  acknowledgment, terminal integrity policy, one bounded structured trace v2,
  and bounded/try-based subordinate sink delivery.
- Added a normative M4 conformance matrix covering public downstream routing,
  modality/command convergence, focus/capture/composition ordering, startup and
  application and mounted subscription work, task/timer/host/executor-refusal
  behavior, cancellation, queue/wake races, sink backpressure, saturation, trace
  causality, and shutdown, plus a 20-vector directional-focus corpus that freezes
  public outcomes without exposing the private score.
- Replaced transient preorder runtime authority with a private persistent
  generational mounted tree and separate runtime-local mounted/semantic IDs.
- Added exact sibling-local keyed reconciliation, unkeyed ordinal matching,
  duplicate-key no-reuse diagnostics, cross-parent remounting, deterministic
  reconciliation generations, and lifetime-based reports.
- Replaced the provisional M2 lifecycle seam with state-aware widget
  capabilities, mounted lifecycle/activation contexts, explicit unmount reasons,
  persistent interaction slots, checked erasure fallbacks, and idempotent
  shutdown through `into_state` and `Drop`.
- Moved focus, activation, input targets, measurement requests, trace targets,
  and aligned frame/style/layout publication to `MountedNodeId`; stale and
  foreign targets are now distinct and focus survives compatible rebuilds.
- Added selective `WidgetInvalidation`, integrity-aware capability caches, and
  one topology-aligned whole-surface publication cache. Topology snapshots
  retain structural/alignment facts only; compatible style and layout phases
  read current mounted authored values, including token-reference and gap
  changes. Structural changes rebuild every node-aligned fact, style-token
  context compatibility compares exact token content, measurement compatibility
  uses the provider's explicit identity/revision promise, and private test probes
  count actual phase entry independently from `SurfacePhaseReport` bookkeeping.
- Added generation-capacity preflight before every mutable activation,
  transactional compatible update with immediate mismatch replacement,
  arena-live unmount ordering, immediate state-only focus validation, and
  non-default interaction-slot retention/reset proofs across compatible update,
  dispatch, state-only activation, removal, cross-parent remount, generational
  arena reuse, replacement, and shutdown.
- Replaced reconciliation `Vec<String>` diagnostics with structured duplicate
  sibling-key and state-payload-mismatch values containing deterministic paths.
- Removed un-emitted `NodeMounted`, `NodeUpdated`, and `NodeUnmounted` trace
  variants; trace v2 remains an M4 boundary.
- Split built-in authoring, mounted matching/lifecycle/cache/invalidation/
  interaction/diagnostics, and surface context/cache ownership into focused
  modules.
- Removed `RuntimeNodeId`, `RuntimeNodeRef`, `RuntimeTreeIndex`, `WidgetState`,
  the old lifecycle vocabulary, direct transient capability/action execution,
  and free element publication without compatibility aliases.
- Expanded downstream conformance and Counter proofs for retained state,
  identity, focus, lifecycle/shutdown, invalidation/cache reuse, publication
  alignment, and stale/foreign target safety.
- Completed M2 with an open downstream `Widget<Action>` protocol, safe private
  erasure, process-local widget/state type identity, checked lifecycle/state
  conformance, typed recursive component action mapping, and open proof-level
  event/layout/paint/semantic/diagnostic participation.
- Migrated text, button, container, Counter, traversal, focus, activation,
  layout, surface publication, and debug output off `ElementKind`; removed
  `ElementKind`, built-in element views, `IntoElement`, `IntoElements`, and
  `SurfaceNodeKind` without compatibility aliases.
- Added a non-publishable downstream conformance package that implements and
  interacts with a stateful custom control and child-bearing custom container
  entirely through public APIs on stable and Rust 1.93.0.
- Corrected lifecycle mismatches to distinguish widget type, state type, and
  erased-payload failures with truthful expected/actual accessors.
- Separated public built-in authored views from private behavior-only text,
  button, and linear-container widget implementations; compile-fail proofs
  prevent built-in builders from entering `Element::new` and losing common
  configuration.
- Replaced `ChildBearingWidget`, `Element::with_children`, and
  `WidgetMeasure::Container` with `ChildLayoutWidget`, `ChildLayout`, and the
  canonical `Container<Action>`/`container` authored path shared by row, column,
  and downstream child-layout widgets.
- Replaced hidden `RefCell<Option<Action>>` consumption through `&self` with
  explicit mutable one-shot extraction while retaining non-`Clone` actions and
  immediate successful-dispatch rebuilds.
- Snapshotted every widget measurement capability once per node/publication,
  independently snapshotted child layout once per child-bearing node, combined
  intrinsic/child minimums, added descendant-preserving unsupported/version-skew
  fallbacks, aligned index/frame/style/layout products, and renamed runtime
  `ButtonLabel` to `ControlLabel` without an alias.
- Clarified that stateless widgets explicitly declare/create `()` state and that
  M2 proves state identity/lifecycle compatibility only; M3 owns the breaking
  state-aware mounted behavior contract.

- Completed M1 public API and vocabulary repair with validated logical values,
  IDs/keys/token IDs, deterministic duplicate diagnostics, typed element builders,
  arity-free children, reduced preludes, and read-only generated products.
- Replaced the nested `element!` property grammar with ordinary builder
  expressions and canonical `on_press`; removed prototype compatibility APIs.
- Restricted `Action: Clone` to activation paths and documented public enum/trait
  evolution policy.
- Corrected M1 identity to compare, order, and hash by Unicode-validated text
  independent of static/owned storage; literal and dynamic validation now share
  one grammar, and token families are explicitly non-exhaustive.
- Corrected derived geometry to saturate at finite boundaries and identity
  diagnostics to use true numeric preorder with deterministic same-node ordering.

> **Category: Current contract**

All notable changes to RunenUI are recorded here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses Semantic Versioning as qualified by the [API stability policy](docs/api-stability.md).

## [Unreleased]

### Changed

- Reframed RunenUI truthfully as a pre-1.0 headless architecture proof with required headless, desktop, and embedded production profiles.
- Added canonical status, support, architecture, documentation-retention, and M0–M12 roadmap authorities.
- Archived the historical Runenwerk UI tree at `legacy-runenwerk-ui-archive-2026-07-11` and removed it from active content.
- Consolidated incremental architecture documents into durable styling, layout, event/effect, ADR, and history records.
- Reset workspace packages from `1.0.0` to `0.1.0` and disabled publication.
- Established dual MIT/Apache-2.0 licensing, governance, toolchain, stability, release, and validation policies.

No stable release has been published.
