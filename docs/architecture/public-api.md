# Public API Contract through M3

> **Category: Current contract**

This document records the reviewed public surface after M3. Source-level Rust
documentation is authoritative for signatures. [ADR 0003](../adr/0003-extensible-view-widget-component-protocol.md)
defines the open authoring/widget foundation; [ADR 0004](../adr/0004-mounted-runtime-reconciliation.md)
defines mounted ownership and reconciliation.

## Ownership and inventory

`runenui_core` owns validated authored values and identity, style intent and
resolution, transient `View`/`Element` authoring, typed built-in views, the open
state-aware `Widget`/`ChildLayoutWidget` contracts, proof capability values,
lifecycle contexts, `WidgetInvalidation`, and typed recursive action mapping.

`runenui_runtime` owns `UiApp`, `AppRuntime`, persistent mounted storage,
reconciliation, lifecycle execution, focus and interaction slots, mounted
targeting, invalidation scheduling, capability caches, measurement/layout
execution, trace, and mounted publication. The public mounted inspection and
integrity vocabulary includes:

- `MountedNodeId`, `SemanticNodeId`, `MountedNodeRef`, and `MountedTreeIndex`;
- `ReconciliationGeneration` and `ReconciliationReport`;
- `FocusTargetResult`, `ActivationResult`, and `RuntimeError`;
- read-only frame, style-report, layout-report, and publication products.

The ordinary preludes remain narrow. Specialist mounted/lifecycle inspection is
imported explicitly from crate roots. Generated IDs, mounted state, arena
storage, reconciliation reports, and publication products have no public
constructors.

## Transient authoring and mounted authority

`View<Action>` consumes a typed authored value into one erased
`Element<Action>`. An element contains common authored ID/key, layout/style
intent, authoring diagnostics, transient children, and one safely erased widget
description. Reconciliation consumes it. `AppRuntime` does not retain an element
root, publish from an element, target an element, or treat authored/preorder
position as persistent identity.

Components are ordinary Rust composition and typed action mapping. They do not
automatically create mounted identity or state. Stable reorderable collections
must author sibling-local unique keys.

The private mounted tree is the sole runtime authority. `MountedTreeIndex`
traverses logical mounted preorder; arena slot order is never observable as tree
order. A mounted node owns parent/ordered children, authored metadata, current
widget description, persistent erased state, interaction slots, replacement
status, capability caches, and internal dirty phases.

## State-aware widget contract

Every `Widget<Action>` declares `State: 'static` and creates it. Stateless
widgets use `State = ()`. The runtime passes persistent state to:

- `mount`, `update`, and `unmount`;
- immutable activation facts, measurement, paint, semantics, and diagnostics;
- mutable activation;
- `ChildLayoutWidget::child_layout`.

`WidgetMountContext`, `WidgetUpdateContext`, and `WidgetActivationContext` can
request `WidgetInvalidation`. `WidgetUnmountContext` exposes a
`WidgetUnmountReason` of `Removed`, `Replaced`, or `RuntimeShutdown`. Contexts do
not expose mounted IDs, task/effect APIs, or mutable runtime internals.

The default update invalidates `ALL` for correctness. Built-in text, button, and
linear-container widgets implement narrower comparison-based invalidation.
Button action payload replacement requires no `Clone`, `Eq`, or `PartialEq` and
does not itself invalidate visual capabilities.

`Element::map_action` replaces only action plumbing. It recursively delegates
every state-aware capability and preserves underlying widget/state type IDs.
Compatible reconciliation installs the newly authored description and mapper or
one-shot action source while retaining state and mounted identity. No global
`Action: Clone`, `Send`, `Sync`, or `'static` bound exists.

## Safe core/runtime bridge

Core cannot depend on runtime, so doc-hidden `runenui_core::__runtime` plumbing
consumes an element into common fields, an erased mounted widget, and transient
children. Erased operations use checked `Any` downcasts. The bridge is absent
from the prelude, exposes no concrete widget downcasts, and provides no payload
or arena construction path. It is technically public only because core and
runtime are separate Rust crates; it is doc-hidden, unstable, unsupported for
application use, semver-exempt before 1.0, and safe. Both crates forbid unsafe
code.

A payload mismatch never invokes a typed callback with the wrong state and
never panics by design. It emits
`runenui.runtime.state-payload-mismatch`. Integrity-aware caches preserve the
difference between mismatch and ordinary capability defaults while publication
uses deterministic disabled/zero/vertical/default fallbacks. Activation exposes
`RuntimeError::WidgetStatePayloadMismatch`. A mismatch during compatible update
replaces immediately without partially committing the new description; a
mismatch discovered by another capability replaces on the next reconciliation.

## Identity and targeting

`MountedNodeId` privately stores:

```text
Arc<RuntimeInstanceMarker> + arena slot + u64 generation
```

It is `Clone`, `Debug`, `Eq`, and `Hash`, but not `Copy`. Equality compares
tokens with `Arc::ptr_eq`; hashing includes `Arc::as_ptr`, slot, and generation.
There is no global counter, random ID, serialization, or preorder identity.

`SemanticNodeId` is a distinct type and namespace with the same mounted lifetime
triplet. It survives compatible update and keyed reorder, and changes on
replacement. It is not yet a semantic-tree node or accessibility identity
contract.

Both IDs are process-local and runtime-instance-local. Validation checks the
runtime token first. A different token is `ForeignRuntime`; a same-runtime
invalid/vacant slot or generation mismatch is `StaleTarget`. Old IDs never
target slot replacements, even when the deterministic arena reuses the same
index.

Authored `ElementId` remains a validated lookup/diagnostic handle. It does not
affect reconciliation compatibility and may change while mounted identity
survives. Ambiguous authored-ID activation is rejected.

## Reconciliation

The root is compatible only when its key, widget type ID, and state type ID
match and it has no recorded integrity failure. `None` root keys compare equal.

For children, keys are sibling-local. A unique keyed child matches a unique old
sibling with the same key and compatible widget/state types regardless of
position. Keyed reorder preserves IDs, state, interaction slots, focus, and
clean caches. A key change, type incompatibility, or cross-parent placement
remounts.

Unkeyed children match by ordinal among unkeyed siblings, not absolute child
index. Keyed insertion/removal does not shift unkeyed matching; unkeyed
insertion/removal can shift later semantic meaning onto existing ordinal
lifetimes.

Keys duplicated in either old or new sibling lists are ineligible for reuse on
both sides. Every new occurrence mounts, every unmatched old occurrence
unmounts, and deterministic diagnostics record the ambiguous key and paths.
There is no first/last-match state preservation.

Final mounted child order exactly follows new authored order. `moved_count`
counts only a preserved node whose sibling position changed under the same
parent. Cross-parent changes are remounts.

## Lifecycle and shutdown

Initial mount and compatible update run parent-before-child in new authored
preorder. Each preserved node receives exactly one checked update from the new
description before that description is committed. Removal unmounts
children-before-parent while each node remains arena-live through its hook;
arena removal, stale identity, and state drop follow the hook. Replacement
fully unmounts and drops the old subtree before mounting the new subtree.

State drops after its unmount hook. Interaction slots and caches disappear with
the mounted lifetime. `AppRuntime::into_state` and `Drop` both perform idempotent
postorder `RuntimeShutdown`; every remaining node receives exactly one shutdown
unmount.

## Activation, focus, and interaction slots

Mounted activation validates the ID, reads checked state-aware activation facts,
rejects disabled/non-actionable targets, preflights reconciliation-generation
capacity, invokes the mutable widget/state pair, applies local invalidation, and optionally
dispatches an application action followed by immediate reconciliation.
`Dispatched`, state-only `Activated`, `NoAction`, disabled/non-activatable,
stale, foreign, and runtime-error outcomes are distinct. Exhaustion rejects all
mutable activation before state, one-shot action, focus, cache, report, trace, or
application mutation. State-only interaction changes validate focus immediately.

Focus stores `Option<MountedNodeId>`. It survives compatible update, authored-ID
change, and keyed reorder, and clears on removal, replacement, disablement, or
loss of actionability/focusability. Traversal follows current mounted preorder.

Each node privately owns hovered, pressed, capture-placeholder, and logical
scroll-offset slots. They survive compatible updates and reset on replacement.
The capture placeholder is an ownership proof only; M4 owns pointer IDs, routed
events, true capture, and release-inside activation.

## Invalidation and capability caches

`WidgetInvalidation` is a manual bitset with `NONE`, `INTERACTION`, `LAYOUT`,
`PAINT`, `SEMANTICS`, `DIAGNOSTICS`, and `ALL`, plus containment, union,
emptiness, `BitOr`, and `BitOrAssign`.

| Invalidation | Cleared widget caches | Scheduled dependent output |
|---|---|---|
| `INTERACTION` | activation | focus validation, interaction, semantics, paint |
| `LAYOUT` | measurement, child layout | layout, hit testing, semantic bounds, paint placement |
| `PAINT` | paint | paint output |
| `SEMANTICS` | semantics | semantic output |
| `DIAGNOSTICS` | widget diagnostics | diagnostic output |
| `ALL` | all capabilities | all dependent phases |

Each capability cache retains unresolved, ready, or payload-mismatch state.
`INTERACTION` alone does not clear paint or semantic widget facts. Structural and
common authored changes add runtime-detected phase work; they do not
automatically requery unrelated clean capabilities. Publication-context changes
compare root constraints, exact style-token content, and measurement-provider
identity/revision. Providers must change identity or revision whenever behavior
changes. Runtime-owned proof publication products and an inspectable
`SurfacePhaseReport` allow clean tree/style/layout/hit-test/paint/semantics/
diagnostics branches to be genuinely skipped while preserving clean widget
caches. Topology snapshots retain structural/alignment metadata only. Style and
layout execution use current mounted authored values, and reconciliation—not the
context key—detects authored token-reference or gap changes. The report lists
actual executed phase functions; private test-only counters are incremented at
the phase entry points and are independent from report recording.
M3 does not claim a production retained layout cache.

## Reports and publication

Initial mount completes reconciliation generation 1. Each successful
reconciliation increments once using `checked_add`; direct dispatch and mutable
activation both preflight exhaustion before application, widget, mounted,
one-shot-action, focus, cache, report, or trace mutation.

`ReconciliationReport` defines counts by mounted lifetime: live nodes after
completion, new lifetimes mounted, preserved nodes updated once, lifetimes
ended, same-parent preserved moves, retained-focus truth, and structured
deterministic diagnostics containing complete duplicate-key old/new occurrences.

`AppRuntime::publish_surface(&mut self, context)` is the only public surface
publication authority. `MountedTreeIndex`, `SurfaceFrame`,
`SurfaceStyleReport`, and `SurfaceLayoutReport` expose equal mounted/semantic ID,
parent, authored-ID, and current preorder sequences with no ghost
actionable/focusable node. A tree change rebuilds every aligned product from one
current topology snapshot. Compatible common-field changes retain that topology
but refresh style/layout facts from current mounted nodes. The free transient
publication function is removed.

## Breaking M3 migration

Removed without aliases:

- `RuntimeNodeId`, `RuntimeNodeRef`, and `RuntimeTreeIndex`;
- `WidgetState`, `WidgetStateMismatch`, `WidgetLifecycle`,
  `WidgetLifecycleRequest`, and the old lifecycle context;
- direct element lifecycle/capability execution and preorder action extraction;
- free `publish_surface(Element)`.

Added:

- mounted/semantic identity and inspection;
- reconciliation generation/report vocabulary;
- state-aware widget lifecycle/activation contexts and unmount reasons;
- selective widget invalidation;
- focus/activation stale and foreign results;
- public runtime integrity errors.

M1 validated values, textual identity, typed configuration, arity-free
composition, protected generated products, and finite saturating geometry remain
in force. M4 and later production subsystems are not implied by M3.
