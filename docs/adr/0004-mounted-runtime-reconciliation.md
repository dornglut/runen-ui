# ADR 0004: Mounted Runtime and Reconciliation

> **Status:** Accepted
>
> **Date:** 2026-07-13
>
> **Milestone:** M3

## Context

M2 established an open transient `View`/`Element`/`Widget` protocol, safe widget
erasure, widget and state type identity, recursive action mapping, and aligned
proof publication. Its runtime still rebuilds and indexes an authored element
tree by preorder for every application update. That index is useful only for the
current tree: it does not retain widget state, keys do not reconcile, focus is
cleared after dispatch, and an old numeric position can identify unrelated
content after a rebuild.

M3 must establish the lifetime boundary required by later interaction,
semantics, scheduling, layout, and rendering milestones without beginning those
milestones. Authored descriptions remain ordinary transient Rust values.
Persistent runtime identity, state, lifecycle, focus, interaction slots,
invalidation, and capability caches require a separate mounted authority.

## Decision

### One mounted runtime authority

The runtime pipeline is:

```text
application state
  -> UiApp::root
  -> transient View/Element tree
  -> mounted reconciliation
  -> persistent MountedTree
  -> state-aware capability resolution
  -> style/layout/publication
```

The transient element tree is consumed by reconciliation. It is not retained as
a parallel runtime tree, is not an interaction target, owns no persistent
identity or widget state, and is not publication authority. `runenui_core` owns
transient authoring, public state-aware widget contracts, lifecycle contexts,
invalidation values, and a doc-hidden unstable safe erasure bridge.
`runenui_runtime` owns
the mounted arena and tree, reconciliation, lifecycle execution, focus,
interaction slots, invalidation scheduling, capability caches, inspection, and
publication. No crate or external arena dependency is added.

### Private safe generational arena

Mounted nodes live in a private `MountedArena<T>` backed by `Vec<Slot<T>>`.
Each slot records a `u64` generation, an optional value, and a permanent-retired
flag. The first live generation is 1. Removal leaves the current generation in
the vacant slot. Allocation deterministically reuses the lowest-index vacant,
non-retired slot and increments its generation with `checked_add` before
installing the value.

Generation never wraps. A vacant slot at `u64::MAX` is retired permanently and
is never eligible for reuse; allocation continues with another reusable slot or
appends a new slot. A generation mismatch can therefore never become valid
again. Arena order is storage order only and never defines tree traversal.

### Mounted and semantic identity

`MountedNodeId` is a non-`Copy`, non-forgeable public value containing a private
`Arc<RuntimeInstanceMarker>`, arena slot index, and arena generation. Runtime
identity uses `Arc::ptr_eq`; hashing includes the pointer from `Arc::as_ptr`,
the slot, and the generation. There is no global counter, randomness, raw
preorder identity, or serialization.

`SemanticNodeId` is a distinct non-forgeable public type and identity namespace
with the same runtime token, slot, and generation triplet. A mounted node owns
one semantic ID for its lifetime. It survives compatible reconciliation and
keyed reorder and is replaced with the mounted lifetime. It is a foundation for
M5, not a semantic tree or accessibility identity contract.

Both ID types are process-local and runtime-instance-local. They are not
authored IDs, are not preorder positions, are not stable across processes, and
are never serialized. An ID may outlive its runtime because it retains the
opaque token, but it is inert. Runtime token validation occurs before slot and
generation validation: a different token is foreign; an invalid, vacant, or
generation-mismatched same-runtime slot is stale. Old IDs never address a slot
replacement.

`MountedTreeIndex` is a deterministic borrowed view over live nodes in logical
mounted preorder. `MountedNodeRef` exposes mounted and semantic IDs, parent and
ordered children, authored ID/key, widget and state type IDs, activation and
focusability facts, and read-only interaction-slot facts. It never exposes arena
order or mutable mounted internals.

### Exact reconciliation

The root is preserved only when its old and new keys compare equal, widget type
IDs match, state type IDs match, and the old node has no recorded integrity failure.
`None` equals `None`; authored ID does not affect compatibility. An incompatible
root is fully unmounted in postorder with `Replaced`, including state drop,
before the new subtree mounts in preorder.

Keys are sibling-local. Before matching children, reconciliation counts keys in
both the old and new sibling lists. A key duplicated in either list is
ineligible for reuse on both sides. Deterministic diagnostics contain the key,
parent authored path, and occurrence paths in sibling order. No first- or
last-match preservation is permitted.

A uniquely keyed new child can match only an old sibling under the same mounted
parent with the same key, widget implementation type, and state type, provided
the old node has no recorded integrity failure. Position is irrelevant. A keyed
reorder preserves mounted and semantic IDs, widget state, interaction slots,
focus, and compatible caches. A changed key, an incompatible widget or state
type, or a move to another parent remounts. Cross-parent changes are never
treated as moves.

Unkeyed children match by ordinal among unkeyed siblings, not by absolute child
index. Preservation also requires matching widget and state types and no
recorded integrity failure. Keyed insertion or removal therefore does not
shift unkeyed matching. Inserting or removing an unkeyed sibling can shift later
unkeyed lifetimes. Stable reorderable collections require keys.

After reconciliation the mounted child list exactly follows new authored order.
Unmatched old subtrees unmount, unmatched new subtrees mount, and preserved
nodes are reordered in their parent's child list. A node contributes to
`moved_count` only when it is preserved under the same parent and its sibling
position changes. Arena slot order is irrelevant.

### State-aware widget contract and safe erasure

Every `Widget<Action>` has a concrete `State: 'static`. `create_state` creates
it, `mount`, `update`, and `unmount` receive mutable state and lifecycle
contexts, immutable capabilities receive shared state, and `activate` receives
mutable state plus an activation context. Stateless widgets use `State = ()`.
The default update invalidates every capability for correctness.
`ChildLayoutWidget::child_layout` also observes state.

Action mapping delegates every capability and preserves the wrapped widget and
state identities. It changes action plumbing only. Compatible reconciliation
keeps the old mounted description live while the newly authored description
runs its update hook against preserved state. Only a successful checked update
commits the new description and authored fields. No widget
description must implement `Clone`, `Eq`, or `PartialEq`, and there is no global
`Action: Clone`, `Send`, `Sync`, or `'static` bound.

Because core cannot depend on runtime, a doc-hidden `runenui_core::__runtime`
bridge owns non-forgeable element parts, erased mounted widgets, and erased
state. Consuming `Element::into_runtime_parts` transfers authored common fields,
diagnostics, the erased widget, and transient children. All erased operations
use checked `Any` downcasts and deterministic errors. The bridge is public only
because core and runtime are separate Rust crates. It is outside the prelude,
doc-hidden, unstable, unsupported for application use, and semver-exempt before
1.0. It exposes no concrete widget downcast, payload-box construction, or arena
access, and uses no unsafe code.

The provisional M2 `WidgetState`, mismatch, lifecycle request, old lifecycle
context, direct element lifecycle/capability methods, preorder action extraction,
and free transient publication APIs are removed without aliases.

### Payload mismatch handling

A checked erased-state mismatch never panics and never invokes a typed callback
with the wrong payload. Capability caches retain `Unresolved`, `Ready`, or
`StatePayloadMismatch`; an integrity failure is never conflated with an ordinary
disabled, zero, absent, or default capability.

During compatible reconciliation, update failure aborts preservation before any
new description or authored field is committed. The old subtree is replaced in
that same generation; descendants do not update after replacement is selected.
Descendants unmount normally. The corrupted node's typed unmount hook runs only
if the old widget/state payload pair still downcasts; otherwise the hook is
skipped and a structured `StatePayloadMismatch` diagnostic is retained. The
corrupt payload and old node data drop before a fresh subtree mounts.

Outside reconciliation, activation returns
`RuntimeError::WidgetStatePayloadMismatch`. Capability resolution uses
deterministic fallbacks: disabled/non-actionable activation, zero fixed
measurement, the recorded child-layout category or vertical layout, default
paint and semantics proofs, and diagnostics including the runtime mismatch.
The integrity state remains recorded, and the next reconciliation remounts a
node whose mismatch was discovered outside reconciliation.

### Lifecycle and shutdown

Initial mount is preorder. For each node the runtime creates state, allocates
mounted and semantic identity, installs the description, invokes `mount`, then
mounts children.

Compatible update is new authored preorder. It preserves identity, state,
interaction slots, focus, and compatible caches; invokes the new description's
checked `update` once; commits only on success; then reconciles children and
applies requested plus runtime-detected invalidation. Keyed reorder calls update
only.

Removal is postorder. Children unmount before the parent hook receives
`WidgetUnmountReason::Removed`. The node remains live in its arena slot through
the hook and mismatch reporting; only then does arena removal make the ID stale
and drop state, description, caches, and interaction slots. Replacement fully
unmounts and drops the old subtree with `Replaced` before any new subtree mount.

Explicit `AppRuntime::into_state` performs a complete postorder shutdown with
`RuntimeShutdown`, then returns application state. `Drop` performs the same
shutdown if needed. Shutdown is explicit and idempotent, so every live node is
unmounted exactly once and a node removed earlier is not unmounted again.
Application state is stored in an `Option` or equivalent safe representation so
it can be moved out after shutdown without unsafe code. Widget callbacks must
not panic; M3 does not add `catch_unwind`.

Lifecycle-owned cleanup consists of the unmount hook, state drop after unmount,
discarded interaction slots and caches, stale mounted/semantic IDs, and
descendant-before-ancestor shutdown. M3 adds no cleanup-closure registry, task,
effect, timer, subscription, or host-command API. M4 attaches such work to this
ownership boundary later.

### Focus, activation, and interaction slots

The M4C4 refinement stores the exact focused lifetime, committed ancestor route,
scope memories, reason, and modality in one runtime-owned focus authority. Focus
survives compatible updates, authored-ID changes, and keyed reorder while the
same generation remains eligible; removal, replacement, disablement, or lost
eligibility clears it with the corresponding reason. Linear traversal follows
current mounted preorder, while directional traversal uses current retained
publication rectangles. Deliberate targeting enters the canonical exact-target
semantic-command queue; foreign and stale targets are rejected without direct
focus mutation.

Activation validates a mounted target, resolves checked state-aware activation
facts without mutating a clean cache, rejects disabled/non-actionable targets,
and preflights reconciliation-generation capacity before any mutable widget
callback. It then invokes the mounted widget, collects invalidation, and
optionally dispatches an application action followed immediately by
reconciliation. `Dispatched` means an action updated the application and
reconciliation completed. `Activated` means no application action was produced
but local invalidation or interaction behavior changed. The empty outcome means neither
occurred. Stale, foreign, and integrity-error results are distinct. At
`u64::MAX`, all mutable activation—including state-only activation—returns
`ReconciliationGenerationExhausted` without changing widget/application state,
one-shot actions, focus, slots, caches, reports, identity, or trace. Rejected or
disabled activation consumes nothing, and non-`Clone` actions remain supported.
State-only `INTERACTION` invalidation validates focus before activation returns.

Each mounted node privately owns hovered, pressed, capture-placeholder, and
logical two-dimensional scroll-offset slots. They initialize on mount, survive
compatible updates and keyed reorder, reset on replacement, and disappear on
unmount. Read-only inspection supports conformance. The capture placeholder is
only an ownership proof: M3 does not implement pointer IDs, routing, capture, or
release-inside activation.

### Invalidation and capability caches

`WidgetInvalidation` is a small manual bitset with `NONE`, `INTERACTION`,
`LAYOUT`, `PAINT`, `SEMANTICS`, `DIAGNOSTICS`, and `ALL`, plus containment,
union, emptiness, `BitOr`, and `BitOrAssign`. Mount, update, and activation
contexts can request invalidation; unmount context exposes only its reason.

Each mounted node caches activation, measurement, optional child layout, paint,
semantics, and widget diagnostics with integrity-aware cache states. Caches begin
dirty, are resolved lazily, and are reused while clean. The dependency matrix is
exact:

| Invalidation | Cleared widget caches | Dirtied runtime phases |
|---|---|---|
| `INTERACTION` | activation | focus validation, interaction output, semantic scheduling, paint scheduling |
| `LAYOUT` | measurement, child layout | layout, hit testing, semantic bounds, paint placement |
| `PAINT` | paint | paint output |
| `SEMANTICS` | semantics | semantic output |
| `DIAGNOSTICS` | widget diagnostics | diagnostic output |
| `ALL` | every capability | every dependent phase |

`INTERACTION` alone does not clear paint or semantic widget facts. A widget
whose state changes those facts must request the corresponding bits. Runtime
interaction styling may schedule output without clearing unrelated widget
caches.

Tree changes collect a current activation-free mounted preorder topology
snapshot and rebuild style, layout, hit-test, paint, semantic, and diagnostic
facts; no older node-aligned vector is reused. The topology snapshot contains
only mounted and semantic IDs, parent relationships, current authored ID,
widget-type identity, and ordered children. It never retains `StyleIntent` or
`LayoutStyle`.

On compatible non-structural publication, style resolution looks up every
topology ID in the same stably borrowed mounted tree and resolves that node's
current mounted `StyleIntent`. Layout ensures current mounted capabilities and
constructs publication-local resolved nodes using each current mounted
`LayoutStyle` plus the current cached style resolution. Padding and gap changes
therefore affect current geometry without a tree phase. Missing topology IDs are
internal invariant violations; stale authored data is never a fallback.

Layout implies hit testing, semantic bounds, and paint placement; padding and
gap changes dirty layout; focusability changes schedule validation; authoring
diagnostics changes dirty diagnostics; and widget requests are unioned with
common-field changes. Structural change does not automatically requery unrelated
clean widget facts. Publication-context changes are detected by root-constraint
bits, exact `StyleTokens` content, and the measurement provider's explicit cache
identity/revision promise. Authored token-reference changes are separate mounted
common-field changes and schedule style plus their resolved dependencies even
when the token snapshot and revision are unchanged.

The retained cache-field classification is:

| Cache data | Classification and invalidation authority |
|---|---|
| Context constraints, exact token snapshot, measurement identity/revision | Context key |
| Mounted/semantic IDs, parent, authored ID, widget type, ordered children | Topology facts |
| Resolutions and style report | Style-phase facts |
| Size, bounds, layout report, and hit-test projection | Layout-phase facts |
| Widget paint facts | Paint-phase facts |
| Widget semantic facts | Semantic-phase facts |
| Widget diagnostics | Diagnostic-phase facts |
| Composed publication | Derived aligned materialization; never separate authority |

Explicit phase functions consume tree, style, layout, hit-test, paint,
semantics, and diagnostics work and skip clean branches. Private test-only probes
increment at the actual phase entry points and compile away outside tests;
`SurfacePhaseReport` bookkeeping has no counter side effect and appends a phase
only after successful execution. Tests compare reports with the independent
entry counters. Focus validation remains an immediate activation/reconciliation
concern rather than clean publication work. Dirty bits clear only after
successful work. This is a proof-level whole-surface phase cache, not a
production retained layout cache.

### Mounted publication and reconciliation reports

`AppRuntime::publish_surface(&mut self, context)` publishes mounted preorder,
mounted parent relationships, current authored common fields, mounted semantic
identity, and state-aware cached capabilities. `MountedTreeIndex`,
`SurfaceFrame`, `SurfaceStyleReport`, and `SurfaceLayoutReport` have equal node
count, mounted and semantic ID order, parent relationships, and authored-ID
metadata. Alignment is checked in release builds while all vectors are built
from the same topology snapshot. No live
focusable or activatable node is omitted. Free publication from `Element` is
removed.

`ReconciliationGeneration` privately wraps `u64`. Initial mount completes
generation 1. Every successful reconciliation increments exactly once with
`checked_add`; exhaustion aborts before application or mounted mutation and
returns `RuntimeError::ReconciliationGenerationExhausted`.

The latest non-forgeable `ReconciliationReport` records generation, final live
node count, mounted, updated, unmounted, and moved counts, retained-focus status,
and structured `ReconciliationDiagnostic` values. Duplicate-key diagnostics
contain the key, parent path, and every old/new occurrence path in deterministic
sibling order. Counts describe mounted lifetimes: initial mount has all nodes
mounted and none updated/unmounted/moved; a preserved node updated once counts
once; a lifetime ended counts once; and movement is same-parent preserved
sibling-position change only. `retained_focus` is true only when a previously
focused mounted ID remains focused afterward.

Trace remains proof-level and contains only coarse events actually emitted.
Unused `NodeMounted`, `NodeUpdated`, and `NodeUnmounted` variants are removed;
M3 does not implement trace v2.

### Multi-surface limitation and M4 boundary

Mounted identity does not encode a platform window, mounted storage does not
depend on a native surface, and semantic identity is renderer-independent. This
makes the representation ready for later association with logical surfaces.

M3 nevertheless owns exactly one mounted root, one active focus domain, and one
current publication domain. It does not provide multiple roots, independent
per-surface focus, publication generations, surface lifecycle, or cross-surface
movement.

M3 does not implement routed capture/target/bubble events, pointer identity or
true capture, release-inside activation, abstract navigation commands, action
queues, effects, tasks, timers, subscriptions, host commands, deterministic
scheduling, trace v2, a production semantic tree, accessibility APIs, a
renderer-neutral paint/hit scene, production layout/style/text, broader
controls, hosts, or renderer backends. Those remain M4 and later work.

### Scheduler authority before unmount

Mounted children still unmount before ancestors and remain arena-live through
their hook, but scheduler producer authority does not. Before invoking each
unmount hook, the runtime centrally revokes every exact-owner work generation,
producer token, queued completion payload, future, timer, source, mapper, and
host request. A producer racing the hook therefore observes stale authority.

## Consequences

M3 deliberately breaks the provisional M2 runtime and lifecycle surface rather
than preserving a competing compatibility path. Authored IDs remain useful for
lookup and diagnostics but never determine mounted lifetime. Stable
reorderable collections must supply sibling-local unique keys.

The runtime gains persistent state and selective capability work while keeping
application state, host concerns, rendering, and product ownership separate.
Safe checked erasure and generational validation make stale, foreign, and
corrupt-payload behavior explicit. Arena reuse stays deterministic without
exposing storage order as semantics.

This decision increases implementation and conformance scope in M3: runtime
shutdown, cache invalidation, publication alignment, external widgets, and the
Counter example must all migrate together. That cost is accepted because a
partial arena or parallel transient authority would leave the core lifetime
invariants unresolved for every later milestone.

## Rejected alternatives

- Retaining `RuntimeNodeId` or `RuntimeTreeIndex` as compatibility aliases is
  rejected because it preserves preorder identity authority.
- Keeping the transient root beside a mounted tree is rejected because it
  creates two runtime authorities and permits publication or targeting drift.
- Matching authored IDs globally is rejected because authored lookup identity
  is not lifetime identity and keys are sibling-local.
- First-match or last-match duplicate-key reuse is rejected because ambiguous
  authored input must not preserve arbitrary state.
- Cross-parent keyed reuse is rejected because ownership, lifecycle, and future
  routed-event boundaries are parent-scoped.
- Wrapping generations, recycling `u64::MAX`, global counters, randomness, and
  serializable mounted IDs are rejected because they weaken deterministic stale
  target guarantees or confuse process-local identity with durable identity.
- A public arena, public erased payload access, unchecked casts, and unsafe
  vtables are rejected by the safe-Rust and non-forgeability contracts.
- An ECS or external arena dependency is rejected because M3 needs neither and
  runtime identity must not assume Runenwerk.
- Recomputing all capabilities after every rebuild is rejected because M3 must
  establish operational invalidation and clean-cache reuse.
- Extending M3 into event routing, effects, semantics, paint scenes, production
  layout/style/text, hosts, or renderers is rejected by milestone sequencing.
