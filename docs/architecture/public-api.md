# Public API Contract through M2

> **Category: Current contract**

This document records the reviewed public surface after M1 and M2. Source-level
Rust documentation remains authoritative for individual signatures. The design
decision is [ADR 0003](../adr/0003-extensible-view-widget-component-protocol.md).

## Ownership and inventory

`runenui_core` publicly owns:

- validated logical lengths, authored IDs/keys, style values, typed token
  references, non-overwriting token definitions, pure style resolution,
  provenance, and diagnostics;
- the `View<Action>` single-node conversion protocol, `Views<Action>` child
  collection conversion, and owned erased `Element<Action>` transient node;
- typed built-in authored `Text`, `Button<Action>`, and `Container<Action>` views
  plus `text`, `button`, `container`, `row`, and `column` builders;
- downstream `Widget<Action>` and `ChildLayoutWidget<Action>` implementation
  contracts, `ChildLayout`, `WidgetTypeId`,
  `WidgetStateTypeId`, opaque `WidgetState`, checked lifecycle state access, and
  bounded activation/measurement/paint/semantic/diagnostic proof facts;
- typed recursive `Element::map_action(ChildAction -> ParentAction)`;
- thin `element!` and `children!` conveniences plus checked identity/token
  literal macros.

`runenui_runtime` publicly owns:

- the open `UiApp` and `MeasurementProvider` traits;
- `AppRuntime` construction, typed dispatch, read-only state/root/trace/focus/
  index access, proof input policy, activation, and surface publication;
- validated constraints, measurement requests/results, and geometry;
- opaque transient `RuntimeNodeId`, borrowed `RuntimeNodeRef`/
  `RuntimeTreeIndex`, and read-only identity diagnostics;
- read-only trace, frame, style-report, layout-report, and publication products
  with deterministic debug formatters.

The non-publishable `runenui_external_widget_conformance` package is a genuine
downstream consumer. It depends only on the two public crates and owns no
privileged imports or runtime hook.

## View, element, component, and widget

`View<Action>` consumes a typed transient authoring value and returns one
`Element<Action>`. `Element` is the owned erased transient product stored by the
current runtime. It retains common ID/key, layout/style intent, diagnostics,
children, and one safely erased widget implementation.

A component is ordinary Rust composition. It may return a typed view/element and
map a child-local action subtree into a parent action. It is not a widget merely
because it returns views, and it does not create mounted identity or state.

`Widget<Action>` is the M2 transient proof-participant contract. An implementation
declares a typed state, creates initial state, and may contribute narrow current
activation, measurement, paint-proof, semantic-proof, diagnostic, and lifecycle
behavior. `State` and `create_state` are mandatory; a stateless widget explicitly
writes `type State = ();` and `fn create_state(&self) {}`. Defaults cover the
remaining non-interactive, zero-sized proof capabilities without forcing
unrelated future subsystem implementations.

Built-ins use exactly this public protocol. `ElementKind`, `TextElement`,
`ButtonElement`, `ContainerElement`, `IntoElement`, and `IntoElements` are
removed; no compatibility aliases or built-in dispatch path remain.

Public built-in views do not implement `Widget<Action>`. Conversion transfers
their common authored fields once into `Element` and installs private
behavior-only `TextWidget`, `ButtonWidget<Action>`, or
`LinearContainerWidget`. Passing a configured built-in builder to
`Element::new` is therefore a compile error rather than a configuration-loss
path.

Child-bearing widgets implement `ChildLayoutWidget<Action>` and return a
non-exhaustive `ChildLayout`; M2 currently interprets `Linear { axis }`.
`Container<Action>::new(widget, children)` and the `container` helper own the
widget and arbitrary `Views<Action>` children atomically. Row, column, and
downstream containers use this same authored builder, including common fields
and container-only `gap`. Normal widgets remain structurally childless; there
is no post-erasure child setter or generic element gap.

## Safe erasure and identity

`Element::new` accepts a downstream `Widget<Action>` and installs a private
object-safe adapter. Erased implementation/state payload types are not public and
cannot be forged. The adapter uses checked safe `Any` downcasts only for opaque
state access; mismatch returns `WidgetStateMismatch` before a typed hook runs.
Both public crates retain `#![forbid(unsafe_code)]`.

Lifecycle compatibility checks concrete widget identity first, declared state
identity second, then performs the private typed payload downcast. The public
non-exhaustive mismatch enum distinguishes `WidgetType`, `StateType`, and
`ErasedStatePayload`; its category-specific expected/actual accessors never
mislabel widget IDs as state IDs. A mismatch returns before the lifecycle hook.

`WidgetTypeId` wraps the concrete implementation's process-local Rust `TypeId`.
`WidgetStateTypeId` separately identifies its declared state. Debug type names
are inspectable but never determine identity. Generic widget instantiations have
Rust's deliberate concrete generic identity. Action mapping delegates the child
widget/state identities rather than inventing a wrapper widget identity.

These IDs are not authored `ElementId`/`ElementKey`, transient `RuntimeNodeId`,
or future mounted identity. They are not serialized and make no cross-build
stability claim.

## Typed action mapping and bounds

`Element::map_action` consumes a subtree and an owned typed mapper. Mapping is
deferred until activation and recursively preserves children, authored ID/key,
layout/style, diagnostics, type/state identity, and all non-action capabilities.
Nested mappings compose without strings or application-action `Any` downcasts.

The stored closure alone is `'static`; it is neither `Send` nor `Sync`. No global
`Action: Clone`, `Send`, `Sync`, or `'static` bound exists on `UiApp`, `Element`,
or `AppRuntime`. M2 removes M1's activation `Clone` limitation: button and
downstream proofs explicitly extract a non-`Clone` action through mutable
`Widget::activate`/`Element::activate`, then successful dispatch immediately
rebuilds the authored tree. Borrowed inspection cannot consume an action.

The transient source is one-shot: the first enabled extraction may return an
action and the second returns `None`. Failed or disabled lookup consumes
nothing. Configured focusability and semantic action intent stay stable after
extraction. Runtime dispatch rebuilds immediately, restoring a newly authored
source; mapped widgets forward mutable extraction through every mapping layer.
The doc-hidden `extract_action_at_preorder_for_runtime` bridge supports current
runtime lookup without exposing mutable children or erased internals. Its raw
index belongs only to one transient tree, has no downstream compatibility
guarantee, and is replaced by M3 generational mounted targeting.

Widget implementation values and state types must be `'static` at the erasure
and state-type-identity operations that require Rust `TypeId`. This bound is
local to those operations.

## State and lifecycle seam

Every widget declares `State: 'static` and creates it; statelessness is explicit,
not an associated-type default. `Element::create_widget_state` returns a
non-forgeable opaque value.
`Element::run_lifecycle` checks both widget and state type identities before
running a typed `Mount`, `Update`, or `Unmount` hook with a bounded request
collector.

This is an isolated conformance seam. `AppRuntime` does not store that state,
does not reconcile widgets, and does not run lifecycle across application
rebuilds. M3 owns the persistent mounted arena, keyed/type/position matching,
generational IDs, state retention/drop, lifecycle scheduling, focus retention,
and granular invalidation.

Only lifecycle receives typed state in M2. Activation, measurement, paint,
semantics, and diagnostics are deliberately state-independent proof methods, so
the current trait is not the complete mounted participant interface. M3 must
introduce a breaking state-aware mounted behavior contract after its storage,
reconciliation, borrowing, and phase-order design is accepted; M2 retains no
persistent state and makes no state-dependent behavior claim.

## Current capability proofs

- Activation supplies enabled/actionable facts and may move one typed action.
- `WidgetMeasure` supplies only fixed, text-intrinsic, or explicit unsupported
  intrinsic minimums. `ChildLayoutWidget` separately supplies child arrangement.
  M2 combines intrinsic and child content component-wise by maximum before
  constraints and padding.
- Intrinsic measurement and child layout are each snapshotted once per
  node/publication and reused by measurement and arrangement. Unsupported or
  unknown intrinsic capabilities publish deterministic diagnostics without
  hiding children. Unknown child layout publishes
  `runenui.child-layout.unrecognized`, falls back vertically, and preserves all
  descendants. Generic control text is `ControlLabel` in both public crates.
- Paint proof supplies deterministic category/description facts. It is not the
  M6 primitive/resource scene.
- Semantic proof supplies deterministic role/name/enabled/action-intent facts.
  It is not the M5 semantic/accessibility tree.
- Widget diagnostics are published in deterministic traversal order beside the
  ordered `SurfaceLayoutNode::diagnostics()` collection.
- Public element/index/frame APIs let downstream tests inspect and interact
  without concrete runtime downcasts.
- Index, frame, style report, and layout report have identical preorder node IDs
  and parent relationships for every valid authored tree.

M4 replaces the bounded activation policy with canonical routed events. M5 owns
semantics, accessibility, and the public testing harness. M6 owns paint and hit
scenes. M7 owns production layout/style extension. M8 owns production text.

## Builders and macros

Typed builders expose only kind-valid configuration. Common element identity and
style remain common. `Views` accepts arbitrary iterators/collections of one view
type; `children!` collects any number of heterogeneous views. `element!` erases
exactly one ordinary Rust expression. Neither macro defines a second property,
binding, component, or action language.

## Generated products and evolution

All generated-product fields remain private. `RuntimeNodeId`, tree indexes,
frames/nodes, reports, traces, focus state, and opaque widget state have no normal
public forgery or mutation path. Constructible public inputs include constraints,
measurement requests, style tokens, widget capability facts, and typed input.

Evolution-prone proof enums are `#[non_exhaustive]`. `UiApp`,
`MeasurementProvider`, `View`, `Views`, `Widget`, and `ChildLayoutWidget` are
open. Identifier builder
input traits remain sealed so downstream code cannot bypass validation. The core
prelude contains ordinary builders, identity, `Element`, `View`, and `Widget`;
specialist capability/state/style inspection uses explicit root imports.

## Breaking migration

| Removed API | M2 contract |
|---|---|
| `ElementKind<Action>` | private safe erasure of public `Widget<Action>` implementations |
| `TextElement`, `ButtonElement<Action>`, `ContainerElement<Action>` | common `Element` and widget capability inspection |
| `IntoElement<Action>` | `View<Action>` |
| `IntoElements<Action>` | `Views<Action>` |
| concrete-kind runtime matches | open activation/measurement/paint/semantic/diagnostic capabilities plus common children |
| `SurfaceNodeKind` | open `SurfaceNode` widget type, paint, semantic, and diagnostic proof facts |
| activation-only `Action: Clone` | explicit mutable one-shot extraction followed by rebuild |
| `ChildBearingWidget`, `Element::with_children` | `ChildLayoutWidget`, `ChildLayout`, and canonical `Container<Action>` authoring |
| `WidgetMeasure::Container` | independent intrinsic `WidgetMeasure` and child `ChildLayout` snapshots |
| singular measurement diagnostic | ordered `SurfaceLayoutNode::diagnostics()` |

M1's validated values, textual identity invariants, typed configuration,
arity-free composition, protected products, and finite saturating geometry remain
in force. M2 changes only the closed extension architecture and related proof
publication; it does not begin M3.
