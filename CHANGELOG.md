# Changelog

## Unreleased

### Changed

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
