# ADR 0005: Canonical Event Routing and Semantic Commands

> **Category: ADR**
>
> **Status:** Accepted
>
> **Accepted by repository owner:** 2026-07-14
>
> **Decision date:** 2026-07-13
>
> **Milestone:** M4
>
> **Reviewed baseline:** `83e3771c34e021ac2960cab2cfd926c1128998ca`

## Implementation authority clarification

This ADR remains the behavioral authority. The accepted
[M4C delivery and routed-transaction charter](../conformance/m4c-delivery-and-routed-transaction-charter.md)
fixes implementation ownership, public protocol boundaries, transaction
admission, mutation policy, trace causality, and the M4C0–M4D3 delivery sequence.
The [M4 conformance matrix](../conformance/m4-conformance-matrix.md) remains the
observable acceptance and proof contract. Charter acceptance does not claim
runtime implementation.

## Context

M3 established one persistent mounted authority with generational
`MountedNodeId`, separate semantic identity, retained focus and interaction
slots, checked state-aware widget erasure, deterministic lifecycle, and
stale/foreign target rejection. The remaining input proof is deliberately
insufficient for production interaction:

- pointer events have no pointer or device identity;
- focus and activation are direct policy methods rather than one routed path;
- pointer activation happens on primary press rather than valid release;
- the capture slot is only a retained placeholder;
- keyboard navigation is hard-coded to Tab/Enter/Space;
- keyboard text and IME composition are not distinct streams;
- there is no capture/target/bubble propagation or default-action control;
- direct activation, `InputIntent`, and combined input handling overlap;
- application update and reconciliation can happen immediately from activation.

M4 must replace those paths without creating a browser DOM clone, hard-coding
built-in controls, introducing a host/platform dependency, weakening M3 mounted
identity, or requiring actions to be `Clone`, `Send`, or `Debug`.

## Decision

### Protocol ownership and safe identity construction

The dependency direction remains `runenui_runtime -> runenui_core`; core never
depends on runtime, and M4 adds no third crate. `runenui_core` owns the public,
host-neutral authored and downstream protocol vocabulary: views/elements/widgets,
recursive action mapping, event families/commands/phases/source/modality,
pointer/device identity, focus/capture/composition/boundary vocabulary,
`WorkKey`, public effect/subscription descriptions, transaction-scoped
`EventContext` APIs, and the opaque mounted/surface identity value types needed
by those protocols.

`runenui_runtime` alone creates the live runtime namespace and owns mounted
arenas/topology, slot/generation allocation, target validation, publication and
input snapshots, route snapshots, live event contexts, focus/capture/composition
mutation, queue sequences, reconciliation, work execution, clocks, wake/redraw,
trace, and shutdown. Core contains definitions and checked construction seams,
not a second live-state authority.

Mounted and surface identity values are conceptually an opaque, reference-counted
namespace plus a `u32` slot and `u64` generation, all with private fields. The
runtime creates its private namespace through a hidden core construction API;
downstream code cannot extract it or forge an ID in it. Any hidden constructor
available to downstream protocol code can create only an unrelated namespace,
so such values always validate as foreign. Runtime validation checks namespace,
slot, and generation. There is no global registry, raw pointer, arena reference,
extractable index, or validation-bypass token. These identities are safe,
process/runtime-local, and non-serialized.

### One canonical non-reentrant event authority

Every accepted external, synthetic, accessibility-stub, automation, or
programmatic interaction enters one canonical sequenced runtime queue. One
routed event transaction is processed at a time:

```text
ingress envelope
  -> normalize event family and source
  -> validate surface-input context
  -> resolve and validate target
  -> snapshot mounted route
  -> capture phase
  -> target phase
  -> bubble phase
  -> semantic default behavior
  -> atomically commit staged interaction changes
  -> append commit-derived notifications
  -> append routed/default actions and commands
  -> append mounted-owned work
```

Widget callbacks never call application `update`, reconcile the mounted tree,
or recursively execute another event. Events, commands, and actions submitted
while a transaction is active are provisional outputs. They are appended to the
sequenced queue only after the transaction commits.

Before the first routed callback, the runtime validates the target, route,
reconciliation generation, and every checked event bridge. A foreign, stale, or
missing initial target invokes no widget callback. A bridge-integrity failure
aborts before the first callback. Callback panics remain unsupported; M4 does
not add partial `catch_unwind` recovery.

### Surface-input and coordinate context

Public host ingress is not an unscoped `(position, MountedNodeId)` pair. Every
publication exposes an opaque `SurfaceInputContext` identifying its runtime
namespace, logical `SurfaceId`, coordinate-space revision, and exact published
hit-test generation. In M4 there is one mounted root and one logical surface,
but `SurfaceId` is present now so M10 can add surface lifetimes without replacing
every event family. Hosts map platform coordinates into RunenUI logical
coordinates for the supplied context; core/runtime ingress exposes no physical
pixel, DPI, monitor, or native-window types.

Pointer targeting is resolved by the runtime from the authoritative hit-test
snapshot associated with the accepted context, or from a checked adapter result
that names the same snapshot. Snapshots live in a bounded generation ring whose
default retains the current and immediately previous publications; configuration
may increase this bound, and the oldest snapshot retires deterministically.

The current context is interpreted against its exact snapshot. Every retained
previous context (only the immediately previous generation at the default
capacity) is also interpreted against its exact snapshot. A retired generation
yields `RetiredSurfaceContext`; another runtime namespace
yields `ForeignSurfaceContext`; another logical surface yields `ForeignSurface`;
and an unknown generation yields `MissingSurfaceGeneration`. None is silently
retargeted through current geometry. Low-level tests may inject resolved targets
only through a seam that performs the same namespace, surface, coordinate-space,
snapshot-generation, and mounted-target validation.

Pointer ingress validates in this order: runtime namespace, logical surface,
active `PointerId` ownership/device consistency, button-transition validity,
supplied snapshot generation, and then the mounted/hit target where the family
requires one. Foreign-runtime and foreign-surface input is rejected without
mutating any local pointer state.

Displayed-generation targeting remains authoritative for pointer routing, but
context rejection cannot strand locally owned release integrity:

- a same-runtime, same-surface `PointerUp` for an active pointer first validates
  the exact button transition; a mismatch is rejected without pointer, pressed,
  or capture mutation even when the supplied generation is retired or missing;
- an accepted partial `PointerUp` whose generation is retired or missing records
  the context diagnosis, does not route or re-hit-test, never activates, commits
  the post-release button set, and keeps the pointer stream alive;
- a partial non-primary unavailable-context release preserves an active primary
  pressed owner and capture; a partial primary release clears primary pressed
  ownership/capture and emits the applicable capture-loss facts/notification;
- an accepted final unavailable-context release performs the existing
  integrity-only terminal cleanup and closes the pointer stream;
- a same-runtime, same-surface `PointerCancel` requires no retained snapshot
  after active pointer ownership validates; an unavailable snapshot remains a
  diagnostic but cannot block terminal cleanup;
- pointer move/down/wheel with an unavailable snapshot remain pure rejection
  with no retargeting or interaction mutation.

An event with a foreign namespace or logical surface cannot clean up or alter a
pointer owned by this runtime/surface, even if its `PointerId` value resembles a
local stream.

M4 does not create multi-window lifecycle, independent surface roots, or
cross-surface focus. Those remain M10 concerns.

### Event families

The public runtime vocabulary uses separate non-exhaustive families rather than
one device-specific catch-all event:

- **Pointer:** required `PointerId`, optional opaque `InputDeviceId`, device kind,
  phase, logical position, movement/scroll delta, complete post-event active
  buttons, changed button, modifiers, and surface-input context.
- **Keyboard:** physical and logical key identity, phase, modifiers, repeat,
  location where available, composition state, and source context.
- **Text commit:** committed Unicode text directed to the focused text-capable
  node. It is not inferred from a keyboard character variant.
- **IME composition:** start, update with preedit text and supplied selection or
  range, end, and cancellation.
- **Semantic command:** device-independent focus, activation, cancellation,
  menu, context, and logical scrolling intent.
- **Pointer-boundary, focus-transition, composition, and capture notifications:**
  deterministic runtime events derived from committed interaction transitions.

Host adapters own platform event types, raw controller/gamepad identity and
axes, dead zones, key translation, native IME objects, and conversion into these
families. Core/runtime types contain only facts required by framework behavior.
Pressure, tilt, twist, click count, and other device facts are added only when a
real control or host consumer requires them.

`PointerId` identifies one active pointer stream and is stable from entry
through final button release or cancellation. `InputDeviceId` identifies an
optional host device within one runtime session. Neither is authored identity,
serialized durable identity, or a substitute for `MountedNodeId`.

### Pointer button transitions and stream lifetime

`PointerEvent::buttons` is the complete active-button snapshot after the event's
transition. `changed_button` names the button changed by `Down` or `Up`.

For an existing pointer stream, accepted transitions are exact:

- `Down` requires `changed_button = Some(button)`, the retained set must not
  already contain `button`, and the supplied set must equal the retained set
  plus `button`;
- `Up` requires `changed_button = Some(button)`, the retained set must contain
  `button`, and the supplied set must equal the retained set minus `button`;
- `Move` and `Wheel` require `changed_button = None` and must preserve the exact
  retained button set;
- `Cancel` is terminal integrity cleanup and is exempt from button-transition
  proof.

A newly established hover-capable stream from `Move` or `Wheel` may retain the
host's initial complete button snapshot because a pointer may enter RunenUI while
buttons are already physically held. A newly established stream from `Down`
requires its changed button to be present in the supplied active set; other
already-held buttons may also be present.

An inconsistent existing-stream transition is rejected as structured
`ButtonTransitionMismatch` processing before displayed-generation resolution,
routing, retargeting, default behavior, pointer-state mutation, or capture
mutation. No compatibility normalization repairs malformed button facts.

`Cancel` always closes the stream. `Up` closes it only when the accepted
post-release button set is empty. An `Up` with remaining active buttons is a
partial release: it routes normally when geometry is available, commits the new
button set, and preserves the same `PointerId` and registration sequence for
subsequent move/down/up/wheel events.

### Normative event policy

Externally routed families use the following exact M4 policy. `C/T/B` means
capture, target, and bubble; target-only means exactly one target callback.

| Family | Route | Bubbles | Cancelable | Default behavior |
|---|---|---:|---:|---|
| Pointer down | C/T/B | yes | yes | A valid primary actionable target may request focus and establish pressed ownership/capture. |
| Pointer move | C/T/B | yes | no | Update the physical path and pressed-inside state. |
| Pointer up | C/T/B | yes | yes | A valid primary release may queue semantic `Activate`. |
| Pointer cancel | C/T/B to the live capture/pressed owner | yes | no | Always clear capture/pressed integrity state. |
| Wheel | C/T/B | yes | yes | Produce logical scrolling command behavior. |
| Keyboard down | C/T/B | yes | yes | Apply focus/navigation/activation policy, including Space press. |
| Keyboard up | C/T/B | yes | yes | Complete a valid matching Space activation. |
| Committed text | C/T/B | yes | yes | No editable-text behavior is introduced in M4. |
| IME start/update/end | C/T/B | yes | no | Maintain composition lifetime bookkeeping only. |
| Semantic command | C/T/B | yes | yes | Run the command's documented semantic default. |

Runtime-derived notifications use this policy:

| Family | Route | Bubbles | Cancelable |
|---|---|---:|---:|
| Pointer enter/leave | target-only | no | no |
| Capture lost/gained | target-only | no | no |
| Focus out, while the old committed route remains live | C/T/B | yes | no |
| Focus in, along the new committed route | C/T/B | yes | no |
| Composition cancellation to the old owner | C/T/B | yes | no |

Input-modality changes are retained interaction state and trace facts, not a
widget event family. Integrity cleanup is mandatory and cannot be suppressed by
`prevent_default`: capture, pressed ownership, composition lifetime, stale work,
and dead focus are always repaired.

### Observable event and routing facts

`UiEvent` contains immutable family payload. A borrowed
`EventContext<'a, Action>` is constructed only by the runtime for the current
transaction and exposes the
current invocation and transaction facts rather than hiding them in private
routing state:

- event phase;
- original mounted target;
- current routed target;
- related target where the family has one;
- source modality and command origin;
- surface-input context;
- logical event time and event/work sequence;
- whether default behavior is cancelable, prevented, or already committed;
- pointer physical hit target/path where relevant.

It also exposes ordered action and semantic-command emission, invalidation,
propagation/default controls, focus and capture requests, and the restricted
exact-owner mounted-work surface from ADR 0006. It may separately request
owner-local mounted-subscription invalidation; that request is provisional and
does not declare subscriptions or execute work. A hidden core sink may connect
the context to runtime-owned provisional ledgers, but the context contains no
mounted storage, application state, host protocol, executor, or mutable runtime
internals. It cannot outlive or be stored beyond the transaction. The checked
erased widget bridge invokes this same context rather than providing a second
event route.

The exact Rust representation may use accessors or a borrowed routing view, but
downstream widgets must be able to distinguish capture, target, and bubble
without inspecting private runtime storage. Target, current target, and related
target are always generational IDs and are validated before exposure.

### Target resolution and route snapshot

Targeting is family-specific:

- an uncaptured pointer uses the current physical hit-test result;
- a captured pointer routes to its live capture owner while retaining the
  physical hit result separately;
- keyboard and ordinary text input target the current focused mounted node;
- IME updates target their exact live composition owner;
- semantic commands carry or resolve one mounted target;
- focus-navigation commands resolve from the active focus scope.

Semantic commands use the same target validation, route phases, propagation
controls, and default-behavior boundary as physical input. They are not direct
activation/focus bypasses.

The runtime distinguishes `Foreign`, `Stale`, `Missing`, and `Live` outcomes.
For a live target, it snapshots the current mounted ancestor path once, root
through target. Capture visits ancestors root-to-parent, target visits the target
exactly once, and bubble visits parent-to-root. The original target does not
change during propagation; current target and phase do. The mounted tree cannot
reconcile underneath the route.

The route contains owned/cloned generational IDs, not borrowed arena positions.
No authored ID or preorder integer participates in routing.

### Widget participation

The open widget protocol gains one state-aware event capability used by
built-ins and downstream widgets alike. Conceptually:

```rust
fn event(
    &mut self,
    state: &mut Self::State,
    event: &UiEvent,
    context: &mut EventContext<Action>,
)
```

This is not permission to implement a second listener registry. Each mounted
widget has at most one framework event capability per phase invocation.
`EventContext` can:

- emit zero or more owned typed actions;
- request invalidation;
- stop later route propagation;
- prevent semantic default behavior;
- request focus movement;
- request, transfer, or release pointer capture;
- request a semantic command;
- request owner-local mounted-subscription invalidation;
- request owner-scoped work through ADR 0006.

Actions are moved into the output ledger and do not require `Clone`. Recursive
action mapping maps every emitted child action into the parent action type.

### Propagation, ordered outputs, and default behavior

RunenUI does not use one ambiguous `handled` bit:

- **Stop propagation** prevents later mounted nodes/phases from receiving the
  event after the current callback returns; it never rolls back callbacks already
  invoked in the current transaction.
- **Prevent default** suppresses runtime semantic default behavior only for a
  cancelable family and does not stop propagation. Calling it for a
  non-cancelable family has no suppressive effect.
- The event outcome reports observation, target status, propagation/default
  state, emitted work, and diagnostics without redefining those controls.

There is no `stop_immediate_propagation` in M4 because RunenUI has no independent
multi-listener registry on one mounted node.

Each transaction records one ordered routed-output ledger. Actions and requested
semantic commands retain the exact order in which callbacks emitted them.
Default behavior runs after propagation and appends its outputs after all routed
callback outputs. Mounted task/timer and same-owner cancellation requests are
kept in a separate ordered work ledger and append after every event-owned
action/command. Subscription invalidation is a distinct declarative trigger,
not an imperative subscription entry in that ledger.

Interaction-state requests are staged separately. For one request class, the
last valid explicit callback request wins. Default behavior runs afterward and
may provide the final request when not prevented. Invalid, stale, foreign, or
capability-incompatible requests are diagnosed and do not erase the latest
valid earlier request.

At successful commit the runtime uses this append order:

1. apply staged focus, capture, composition, pressed, modality, and interaction
   changes atomically;
2. append commit-derived notifications in deterministic order;
3. append one coalesced exact-owner mounted-subscription reconciliation envelope
   for each committed subscription invalidation;
4. append routed callback actions and commands in ledger order;
5. append default-behavior actions and commands in ledger order;
6. append event-owned mounted work in request order.

The reconciliation envelope is later mandatory derived work, not immediate
subscription execution. ADR 0006 fixes its complete-set evaluation and start/
cancellation ordering.

For a single transition, notification order is:

1. capture lost before capture gained;
2. composition cancellation before focus leaves its owner;
3. `FocusOut` before `FocusIn`;
4. other transition notifications in stable mounted logical order.

These notifications are later canonical event transactions. They are queued
before application actions from the initiating event so a committed interaction
transition is observable before an action can remove its new owner. If a target
is nevertheless stale when its notification reaches the queue front, no
post-unmount callback occurs and trace records the suppressed delivery.

Before default behavior reads enabled, actionable, focusable, text-capable, or
other widget capabilities, callback invalidation is applied to the
transaction-local capability view and every dirty fact is re-queried from the
current mounted widget state. Default behavior never acts on a stale pre-route
capability cache.

### Semantic commands and modality convergence

Physical input is not the control behavior contract. Hosts and runtime policy
normalize input into non-exhaustive semantic commands including:

- `FocusNext`, `FocusPrevious`;
- `FocusUp`, `FocusDown`, `FocusLeft`, `FocusRight`;
- `Activate`;
- `CancelOrBack`;
- `OpenMenu`, `OpenContextMenu`;
- logical scroll commands.

Pointer, keyboard, normalized controller navigation, accessibility-stub actions,
automation, and programmatic APIs converge on these commands. Command envelopes
retain a non-exhaustive origin/data record so M5 accessibility mapping can carry
source and command-specific facts without turning accessibility into physical
input or replacing the queue contract.

The runtime records the last accepted input modality—pointer, keyboard,
controller, accessibility, automation, or programmatic—without changing action
semantics. Programmatic `activate` helpers become wrappers around the same
routed `Activate` command and never invoke widget activation or application
update directly.

Command defaults are exact:

| Command class | Commands | Unconsumed default |
|---|---|---|
| Framework-default | `FocusNext`, `FocusPrevious`, `FocusUp`, `FocusDown`, `FocusLeft`, `FocusRight`, `Activate` | Existing runtime focus-navigation or actionable-widget activation policy. |
| Route-only | `CancelOrBack`, `OpenMenu`, `OpenContextMenu`, logical scroll commands | No action and no runtime mutation. |

Route-only commands participate once in normal capture/target/bubble routing.
There is no second implicit ancestor-delegation phase after bubbling; ancestors
already participated in capture and bubble. A widget or focus scope delegates
only by explicitly emitting another semantic command or typed application
action. That output commits at the normal transaction output position, receives
a new queue sequence, and never executes recursively.

An unprevented wheel event emits exactly one logical-scroll semantic command;
a prevented wheel emits none. The emitted command then follows the same
route-only contract, so an unconsumed command performs no production scrolling.
M4 does not hard-code application navigation, menus, or scrolling mutation.

Keyboard and text remain separate. Keyboard policy may produce commands;
committed text and composition events use dedicated streams. M4 defines routing
and lifetime, not editable text behavior, which remains M8 work.

### Focus scopes and navigation

The mounted root is the initial focus scope. A widget can declare a focus-scope
boundary through the open capability protocol. Every focusable node belongs to
its nearest live scope.

Linear next/previous navigation uses current mounted logical order within the
active scope. Directional navigation uses current layout rectangles and one
deterministic internal candidate-selection policy:

1. filter to live, enabled, focusable candidates eligible in the requested
   direction and current scope;
2. prefer candidates whose projection overlaps the origin on the orthogonal
   axis over equally reachable off-beam candidates;
3. rank remaining candidates by stable primary-axis gap, orthogonal displacement,
   projected overlap, and geometric overlap facts;
4. use mounted logical order as the final tie-break.

The exact score and weights remain private runtime policy. Observable outcomes
are frozen by the normative
[`M4 directional-focus corpus`](../conformance/m4-directional-focus-corpus.md),
whose vectors cover direct movement, beams/off-beam choices, partial and unequal
geometry, overlap and ties, nested scope policies, root boundaries, exact-
generation restoration, stale fallback, eligibility filtering, half-plane
rejection, and edge-touching geometry. Implementations may evolve the internal
formula only while every corpus expectation remains unchanged or an explicit
owner-approved architecture revision changes the corpus first.

Scope traversal policy is explicit. The M4 defaults are:

- the root scope wraps linear next/previous traversal and stops directional
  traversal when no candidate exists;
- a nested scope delegates at its boundary to its parent and does not wrap or
  trap unless configured;
- explicit scope policy may wrap, trap, stop, delegate, or request logical
  scrolling;
- a scope remembers its last live focused descendant and restores it only when
  that exact generation remains eligible; otherwise normal traversal chooses a
  replacement.

Focus changes commit atomically after the initiating route. A successful
transition queues `FocusOut` for the old exact mounted lifetime before `FocusIn`
for the new lifetime, with transition reason and related target where live.
Ancestor focus-within state is derived from old/new routes and invalidated in the
same commit.

Focus-change reasons include pointer, linear navigation, directional navigation,
programmatic request, removal, disablement, scope restoration, and shutdown.
Scope identity in M4 is not multi-window identity.

### IME composition ownership

An active composition session belongs to the exact focused mounted generation
that accepted composition start. Focus transfer, owner removal/replacement,
disablement, explicit cancellation, or shutdown invalidates the session and
queues composition cancellation before `FocusOut`.

A late update or commit for an invalidated session is stale and cannot be
retargeted to the new focus owner. Keyboard events marked as occurring during
composition remain keyboard events; they do not synthesize committed text.

### Pointer capture

The runtime owns one capture entry per active `PointerId`:

```text
PointerId -> live MountedNodeId
```

Capture requests are staged during propagation. The last valid request wins and
is applied at transaction commit. Transfer releases the old owner before
granting the new owner. Explicit release, primary pointer up, final stream
release, pointer cancellation, shutdown, and owner removal/replacement release
capture deterministically. A non-primary partial `PointerUp` does not by itself
release capture retained by an active primary pressed interaction.

While capture is active, subsequent pointer events route to the capture owner.
The runtime still computes the physical hit path for hover and release-inside
policy. Live owners receive targeted capture-lost/gained notifications before
the initiating event's application actions. A node being removed receives no
post-unmount event callback; unmount and trace are the cleanup boundary.

Capture IDs and owners are generational. A stale or foreign owner can never
capture a new node occupying the same arena slot.

### Pointer boundaries and geometry changes

M4 replaces proof booleans with pointer-aware retained interaction state. Public
inspection may derive aggregate `hovered` or `pressed` facts, while runtime
ownership records contributing pointer/button identities.

For pointer ingress, normalization resolves the new physical path and atomically
expands one ingress into an ordered bundle when the path changes:

1. pointer-leave notifications, inner-to-outer;
2. pointer-enter notifications, outer-to-inner;
3. the ordinary pointer event.

The bundle is appended at the queue tail as one accepted ingress expansion; it
does not overtake previously accepted work. Boundary notifications are targeted
and non-bubbling. Ordinary pointer move/down/up events use the routed pipeline.
Captured routing and physical hover/release-inside state remain independent.

The runtime retains the latest logical position and physical path for every
hover-capable active pointer. Whenever the authoritative hit-test/publication
generation changes because layout, visibility, clipping, stacking, or pointer
policy changed, the runtime re-hit-tests those stationary pointers before it
reports quiescence or accepts the next external pointer ingress. Retained
pointers are processed by pointer-registration sequence; for each pointer,
leave notifications append inner-to-outer before enter notifications append
outer-to-inner. A new publication does not change the target of an already
accepted transaction that used an older retained `SurfaceInputContext`. Hover
therefore cannot remain stale merely because the user did not move the pointer.

Removal, replacement, disablement, pointer cancellation, explicit interaction
cancellation, or shutdown clears incompatible pressed/capture state before a
later event can address it.

### Correct semantic activation

An actionable widget's default primary-pointer behavior is:

1. primary down reaches a live, enabled actionable target;
2. if default is not prevented, focus is requested, the pointer becomes pressed
   owner, and capture is acquired;
3. movement and geometry changes update whether the physical path remains inside
   that exact mounted lifetime;
4. primary up routes to the capture owner;
5. if default is not prevented, the same mounted lifetime remains live,
   enabled/actionable, and physically inside, the runtime queues semantic
   `Activate`;
6. primary pressed state and capture clear whether activation succeeds or is
   cancelled; the pointer stream itself remains live when other buttons remain
   active and closes only on final release or cancellation.

Release outside, cancellation, removal/replacement, disablement, or transition to
non-actionable state never activates.

Keyboard policy converges on the same command. Enter queues activation on
accepted non-repeated key down. Space starts pressed state on key down and queues
activation on a matching key up only while focus remains on the same live enabled
target. Accessibility and programmatic activation issue the same atomic command
without fabricating pointer or keyboard phases.

M4 renames the authored semantic callback from `on_press` to `on_activate`.
`on_press` described the M1–M3 proof but is incorrect once pointer release,
keyboard, controller, accessibility, automation, and programmatic sources share
one activation contract. This pre-1.0 migration keeps no compatibility alias.

### Mutation and reconciliation timing

Routed widget callbacks may mutate only runtime-owned widget state and collect
invalidation/output requests. App-owned durable state mutates only while an
action is dequeued through `UiApp::update` under ADR 0006.

Each action update is followed by transient-root rebuild and mounted
reconciliation before the next action. If reconciliation removes a target named
by later queued work, that work observes a stale target and is rejected rather
than retargeted.

## M4 implementation migration

Implementation must remove or subsume, not preserve in parallel:

- `InputIntent`;
- direct pointer-press activation;
- separate direct pointer/keyboard focus and activation policy paths;
- event APIs that synchronously call `UiApp::update`;
- the proof capture placeholder;
- device-specific programmatic activation bypasses;
- public host ingress that treats an unchecked mounted ID as sufficient pointer
  targeting;
- `on_press` as the semantic callback name.

Low-level test helpers may remain only when they call the canonical queue and
preserve normal surface, target, ordering, trace, and reconciliation behavior.

The required proof matrix is normative and lives in
[`../conformance/m4-conformance-matrix.md`](../conformance/m4-conformance-matrix.md).
It covers downstream widgets, Counter, every required input modality, command
families, routing/default controls, focus/capture/composition lifetimes,
stationary-pointer geometry changes, retired/missing pointer-release integrity,
foreign-state isolation, exact unconsumed route-only defaults, and trace order.

## Consequences

The runtime gains a single interaction authority aligned with mounted lifetimes.
Event routing cannot observe a half-reconciled tree, action order is
deterministic, physical devices do not become control semantics, and custom
widgets participate without central registration or concrete-type matching.

The event bridge, surface-input context, pointer-aware interaction storage, and
transition queues increase runtime complexity. That complexity is accepted
because capture, nested controls, scrolling, accessibility convergence, text
focus, overlays, and editor tools cannot be correct on direct press-to-action
helpers.

M4 does not add a native event loop, raw controller API, accessibility tree,
editable text control, renderer scene, broad control library, or multi-surface
host.

## Rejected alternatives

- **Direct target-only dispatch:** nested controls, overlays, editor tools, and
  default behavior need ancestor participation.
- **Immediate update/reconciliation inside a callback:** it invalidates the
  active route and makes ordering/reentrancy implicit.
- **A browser-scale listener registry:** one open widget capability supplies the
  required extension point without duplicate listener identity/removal rules.
- **One handled flag:** propagation and default behavior are independent.
- **Pointer capture without pointer identity:** multiple pointers and
  cancellation cannot be represented safely.
- **Hit target as capture target:** captured routing and physical inside/hover
  state are distinct.
- **Unchecked mounted target supplied by the host:** geometry and mounted
  generation must be tied to the same surface-input context.
- **Re-hit-test only on pointer movement:** layout changes beneath a stationary
  pointer would leave hover and release-inside state stale.
- **Keyboard characters as text input:** IME, composition, dead keys, and
  non-keyboard text sources require separate streams.
- **Device-specific widget actions:** all modalities must converge on semantic
  commands.
- **Keep `on_press` as semantic activation:** it encodes one physical phase and
  misdescribes keyboard/accessibility/programmatic activation.
- **A public fixed directional-distance formula:** tested internal policy can
  evolve deliberately without creating a public compatibility trap.
- **Concrete built-in control matching:** rejected by the M2 open widget
  protocol.
- **ECS/platform/window dependencies:** headless, standalone, and embedded hosts
  require a neutral runtime.

## Research basis

The design intentionally adapts rather than copies these primary sources:

- [WHATWG DOM event dispatch](https://dom.spec.whatwg.org/#concept-event-dispatch)
  for route snapshotting, target/current-target/phase facts, and separation of
  propagation from default prevention;
- [W3C Pointer Events Level 4](https://www.w3.org/TR/pointerevents4/#pointer-capture)
  for per-pointer capture, pending transfer, capture loss, and boundary updates
  caused by layout changes;
- [W3C UI Events](https://www.w3.org/TR/uievents/)
  for separate keyboard, committed text, composition, and focus-transition
  streams;
- [W3C CSS Spatial Navigation Level 1](https://www.w3.org/TR/css-nav-1/#focus-navigation-heuristics)
  for directional filtering, projected overlap, distance scoring, and scope
  behavior, treated as research rather than copied public API;
- [AccessKit action requests](https://docs.rs/accesskit/latest/accesskit/struct.ActionRequest.html)
  for future semantic-action source/data convergence without making
  accessibility a physical-input path.
