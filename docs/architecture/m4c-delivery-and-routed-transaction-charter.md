# M4C Delivery and Routed-Transaction Charter

> **Category: Target architecture**
>
> **Status:** Review draft for owner acceptance
>
> **Milestone:** M4C–M4D

This document closes the implementation-defining decisions between accepted
[ADR 0005](../adr/0005-canonical-event-routing-and-commands.md), the canonical
[M4 conformance matrix](m4-conformance-matrix.md), and executable M4C work.
ADR 0005 remains the behavioral authority. This charter fixes delivery
ownership, public protocol boundaries, transaction admission, mutation and
failure semantics, observation authority, trace causality, and slice order.

The conformance matrix remains the normative behavior inventory. This document
is its delivery and implementation authority and cannot weaken a matrix
observation.

## Gate and execution order

M4B must be owner-accepted and merged before any M4C branch begins. The first
post-M4B branch is documentation-only M4C0. M4C1 may begin only after M4C0 is
accepted and merged.

```text
M4B acceptance and merge
  -> M4C0 conformance ownership and decision closure
  -> M4C1 routed semantic-command kernel
  -> M4C2 displayed-generation surface context
  -> M4C3 pointer lifecycle and release-inside activation
  -> M4C4 focus scopes, directional navigation, and modality
  -> M4C5 keyboard, committed text, IME, and canonical-path closure
  -> M4D1 complete trace schema and causality
  -> M4D2 export, redaction, and bounded external sink
  -> M4D3 replay, migration, and M4 conformance closure
```

Branches are sequential and start from the updated `master` produced by the
previous accepted pull request. M4C and M4D branches are never stacked on an
unmerged feature branch.

## M4C0 documentation gate

M4C0 implements no framework behavior. It must:

- give every M4 matrix row a stable ID;
- assign every row to one primary delivery slice;
- retain secondary integration owners where later work completes an earlier
  protocol;
- add explicit positive, negative, and trace proof ownership;
- use only `blocked`, `implementation-complete`, `proof-complete`, and
  `owner-accepted` as row statuses;
- split aggregate rows that cross surface, pointer, automation, accessibility,
  or trace ownership;
- align the roadmap, public API, status map, feature-support matrix, README,
  changelog, and matrix with the accepted slice boundaries;
- correct any remaining superseded M4B wording;
- publish the exact M4C1 row list and acceptance gate.

A row becomes `owner-accepted` only after public behavior, negative proof, trace
proof, stable/MSRV validation, exact-head CI, owner review, and merge all pass.

## Matrix ID families

| Family | Primary contract | Delivery owner |
|---|---|---|
| `APP-*` | Application lifecycle and update transactions | M4B |
| `WORK-*` | Tasks, timers, subscriptions, and host requests | M4B |
| `PUMP-*` | FIFO, readiness, budgets, and quiescence | M4B |
| `WAKE-*` | Wake, redraw, saturation, and terminal scheduler behavior | M4B |
| `ID-*` | Core-owned runtime-local public protocol identity | M4C1 |
| `ROUTE-*` | Route snapshot, phase invocation, propagation, and defaults | M4C1 |
| `CMD-*` | Semantic commands, delegation, and exact-target source convergence | M4C1 |
| `SURFACE-*` | Surface context and displayed-generation retention | M4C2 |
| `PTR-*` | Pointer stream identity, validation, and physical targeting | M4C3 |
| `CAP-*` | Pressed ownership and true pointer capture | M4C3 |
| `BOUNDARY-*` | Enter/leave and stationary-pointer geometry updates | M4C3 |
| `FOCUS-*` | Focus transitions, scopes, and restoration | M4C4 |
| `DF-01`–`DF-20` | Directional-focus public-outcome corpus | M4C4 |
| `MOD-*` | Retained input modality | M4C4 |
| `KEY-*` | Keyboard routing and activation policy | M4C5 |
| `TEXT-*` | Committed-text routing | M4C5 |
| `IME-*` | Composition ownership, routing, and cancellation | M4C5 |
| `AUTOMATION-*` | Authored-ID automation resolution and ambiguity | M4C5 |
| `ACCESS-*` | Semantic-to-mounted accessibility mapping | M5 |
| `TRACE-EVENT-*` | Complete M4 event/scheduler trace schema | M4D1 |
| `TRACE-EXPORT-*` | JSONL, redaction, and bounded external sink | M4D2 |
| `REPLAY-*` | Replay foundation and causal reconstruction | M4D3 |
| `MIGRATION-*` | Removal of transitional authorities | M4D3 |
| `M4-CLOSE-*` | Final milestone acceptance | M4D3 |

## Shared runtime namespace

`runenui_core` owns one hidden runtime namespace token used by every runtime-local
public protocol identity. Runtime creates the live namespace through a doc-hidden,
semver-exempt bridge. Downstream code cannot extract a live namespace from an ID
or forge an ID into an existing runtime.

Conceptually:

```rust
#[doc(hidden)]
pub struct RuntimeNamespace {
    marker: Arc<RuntimeNamespaceMarker>,
}
```

The same namespace is reused by:

- core-owned `MountedNodeId` in M4C1;
- `SemanticNodeId` while it remains a mounted-lifetime identity;
- `SurfaceId` and `SurfaceInputContext` in M4C2;
- later public runtime-local protocol identities.

`MountedNodeId` stores an opaque namespace, a checked `u32` slot, and a
non-wrapping `u64` generation. Runtime may use a wider internal arena index but
performs checked conversion when constructing the public ID.

`MountedNodeId` is `Clone + Eq + Hash + Send + Sync`, is not `Copy`, is not
serialized, and exposes no namespace, slot, or generation accessor. Debug output
does not expose a usable namespace address or validation-bypass value. Equality
and hashing include namespace, slot, and generation.

Hidden negative-test construction may create only an unrelated namespace. It
cannot extract or reuse the live namespace from an existing ID.

## Core-owned logical values

M4C1 moves protocol values needed by core-owned event APIs into `runenui_core`:

- `MonotonicInstant`;
- `MonotonicTimeError`;
- `WorkSequence`.

Runtime retains:

- `MonotonicClock`;
- `ManualClock`;
- live clock ownership and advancement;
- queue sequence allocation;
- timer scheduling.

Core value constructors remain private. Runtime uses doc-hidden checked
constructors and may re-export the moved values for deliberate import
compatibility without retaining a second type authority.

## M4C1 event protocol

M4C1 introduces only the complete vocabulary required by routed semantic
commands. Pointer, keyboard, text, IME, surface, focus, capture, and scrolling
vocabulary is added by its owning slice and is not represented by placeholder
variants or no-op methods.

```rust
#[non_exhaustive]
pub enum EventPhase {
    Capture,
    Target,
    Bubble,
}

#[non_exhaustive]
pub enum EventSource {
    Programmatic,
    Automation,
    Accessibility,
    Controller,
}

#[non_exhaustive]
pub enum CommandDerivation {
    Direct,
    Delegated,
}

pub struct CommandOrigin {
    source: EventSource,
    derivation: CommandDerivation,
}

#[non_exhaustive]
pub enum SemanticCommand {
    Activate,
    CancelOrBack,
    OpenMenu,
    OpenContextMenu,
}

#[non_exhaustive]
pub enum UiEvent {
    SemanticCommand(SemanticCommandEvent),
}

pub struct SemanticCommandEvent {
    command: SemanticCommand,
    origin: CommandOrigin,
}
```

Exact field privacy and accessor names follow repository conventions. The
concepts and ownership do not change.

There is no independent source field beside `CommandOrigin`; contradictory
source/origin combinations are unrepresentable. Causal parentage belongs to the
runtime envelope and trace, not the protocol payload.

Logical scrolling is introduced by M4C3 together with wheel normalization.
Focus-navigation commands are introduced by M4C4 together with their complete
defaults and corpus proofs.

## Widget event mutation

The open widget protocol gains one state-aware event capability:

```rust
#[must_use]
pub struct WidgetEventOutput {
    state_changed: bool,
}

impl WidgetEventOutput {
    pub const fn none() -> Self;
    pub const fn changed() -> Self;
    pub const fn state_changed(&self) -> bool;
}

fn event(
    &mut self,
    state: &mut Self::State,
    event: &UiEvent,
    context: &mut EventContext<'_, Action>,
) -> WidgetEventOutput;
```

Widget-local state mutates in place after complete transaction preflight and is
not rollback-capable. `state_changed` reports that mutation explicitly.
Runtime-owned focus, capture, pressed, composition, modality, and boundary
changes are staged only by the slices that introduce them.

Known admission failures reject before the first mutable callback. Unexpected
failure after any widget-state mutation begins enters terminal `Poisoned`.
RunenUI does not claim rollback of arbitrary widget state.

Recursive action mapping maps every emitted action and preserves state change,
ordinary invalidation, subscription invalidation, delegated commands, and
mounted-owned work.

## Event context

`EventContext<'a, Action>` is borrowed, transaction-scoped, runtime-created, and
cannot outlive the callback. It exposes no app state, mounted arena, executor,
host protocol, or unrestricted runtime authority.

M4C1 read-only facts are:

- phase;
- original target;
- current target;
- optional related target;
- command origin;
- logical `WorkSequence`;
- logical `MonotonicInstant`;
- default cancelability and prevention state.

M4C1 provisional requests are:

- emit an owned typed action;
- emit a delegated semantic command;
- request `WidgetInvalidation`;
- request owner-local subscription invalidation;
- request ADR 0006 mounted-owned work;
- stop propagation;
- prevent cancelable default behavior.

Focus, capture, composition, modality, and boundary methods do not exist until
their owning slice implements them.

## Submission and exact rejection ownership

Public command submission is queued and non-reentrant:

```rust
pub fn submit_command(
    &mut self,
    target: MountedNodeId,
    command: SemanticCommand,
    origin: CommandOrigin,
) -> Result<CommandSubmission, SubmitCommandError>;
```

`CommandSubmission` contains the accepted `WorkSequence`.

```rust
pub struct UnacceptedCommand {
    target: MountedNodeId,
    command: SemanticCommand,
    origin: CommandOrigin,
}
```

Errors distinguish full, closed, terminal with exact reason, foreign target,
stale target, missing target, work-sequence exhaustion, and enabled-trace-
sequence exhaustion. Every rejection returns the exact `UnacceptedCommand` and
consumes no queue or trace sequence.

Submission-time and processing-time validation remain distinct:

- submission rejection means no envelope was accepted and ownership is returned;
- processing rejection means an accepted target became stale before processing;
  no callback runs and trace records the exact outcome.

## Routed outcome observation

Submission returns acceptance only. `PumpReport` remains aggregate scheduler
observation and does not become an event-result queue.

Canonical trace is the public per-event observation authority during M4. An
internal `RoutedEventOutcome` may support processing and focused tests, but no
second public result history or polling registry exists.

## Route and bridge preflight

At the queue front, runtime:

1. revalidates exact target status;
2. snapshots the root-to-target route as owned cloned IDs;
3. validates every route node and checked event bridge;
4. computes the exact transaction admission plan;
5. rejects known failure before the first callback;
6. invokes capture root-to-parent;
7. invokes target exactly once;
8. invokes bubble parent-to-root;
9. evaluates semantic default unless prevented;
10. commits and appends derived queue output.

The route never borrows arena positions across callbacks. The mounted tree cannot
reconcile during routing. Callback-submitted work remains provisional and never
executes recursively.

## Propagation and defaults

There is no ambiguous `handled` bit.

`stop_propagation` prevents later callbacks after the current callback returns.
It does not undo earlier callbacks or suppress default behavior.

`prevent_default` suppresses only cancelable semantic default behavior. It does
not stop propagation or suppress mandatory integrity cleanup.

There is no `stop_immediate_propagation`; one mounted widget has no independent
same-node listener registry.

## Exact transaction admission

One checked `RoutedTransactionAdmissionPlan` owns complete preflight:

```rust
struct RoutedTransactionAdmissionPlan {
    route_invocations: usize,
    max_outputs: usize,
    max_subscription_reconciliations: usize,
    queue_slots: usize,
    local_task_slots: usize,
    send_task_slots: usize,
    timer_slots: usize,
    work_generations: usize,
    trace_records: usize,
}
```

One shared configured `transaction_outputs` allowance covers every route callback
and semantic default.

It counts:

- each application action;
- each delegated command;
- each mounted effect or cancellation;
- each unique exact-owner subscription invalidation.

Repeated invalidation for the same owner coalesces and counts once.

It does not count:

- `WidgetEventOutput::state_changed`;
- ordinary invalidation;
- stop propagation;
- prevent default.

Default activation uses the same remaining allowance. Queue slots, family
capacities, private generations, reconciliation envelopes, and mandatory trace
capacity are derived with checked arithmetic. Boundary tests cover exact
capacity, required-minus-one, trace disabled, work-sequence exhaustion, and
trace-sequence exhaustion.

## Commit order

M4C1 commits:

1. in-place widget-state mutation facts and ordinary invalidation;
2. commit-derived notifications implemented by the current slice;
3. one coalesced subscription-reconciliation envelope per invalidated owner;
4. routed callback actions and delegated commands in emission order;
5. semantic-default actions and delegated commands;
6. mounted-owned work in request order.

M4C1 has no focus, capture, composition, modality, or boundary notifications.
No output is silently dropped. Known failure rejects before mutation. Unexpected
post-mutation failure poisons and starts no provisional external work.

## Semantic command defaults

Unprevented `Activate` routes through all phases, re-queries the original target's
live enabled/actionable capability after callback invalidation, and invokes the
target activation capability once as semantic default.

`WidgetActivationOutput<Action>` is folded into the same event admission and
commit plan. Activation never calls app update, reconciles, recursively routes,
or submits through a public activation helper.

Prevented, stale, disabled, or non-actionable targets do not invoke the activation
factory. Routed callback outputs may still commit when default is prevented.

`CancelOrBack`, `OpenMenu`, and `OpenContextMenu` route once. Their unconsumed
default is no action and no runtime mutation. No second ancestor pass exists.

## Delegation target

`EventContext::emit_command(command)` targets the current routed node. It creates
a provisional delegated command with `CommandDerivation::Delegated` and preserves
the initiating source.

The delegated command is appended after the current transaction, receives a new
`WorkSequence`, and never executes recursively. M4C1 introduces no arbitrary
`emit_command_to` API. Semantic default always applies to the original target.

## Source convergence and resolution ownership

M4C1 proves exact-mounted-target routed `Activate` for programmatic, automation,
accessibility-stub, and normalized-controller sources. They share queue, route,
default, action, update, reconciliation, and trace behavior. Raw platform
controller and accessibility types do not enter the neutral protocol.

M4C1 does not claim source-specific target resolution:

- authored-ID automation lookup, missing identity, ambiguity, and stale target
  diagnosis are owned by M4C5;
- semantic-to-mounted accessibility mapping and stale semantic identity are owned
  by M5.

## Transitional pointer and keyboard policy

M4C1 removes `InputIntent`, direct programmatic activation, direct pointer-press
activation, and direct keyboard activation.

Old pointer press does not synthesize `Activate`; release-inside policy belongs
to M4C3. Old key proofs do not become final semantic activation; Enter repeat and
Space down/up ownership belong to M4C5.

At most explicitly named focus-only proof helpers may remain temporarily. They
emit no app action, invoke no activation callback, are documented as transitional,
and have an exact M4C3/M4C4/M4C5 removal owner. Counter uses programmatic routed
`Activate` until physical input slices land.

## Trace causal graph

The command envelope `WorkSequence` is the transaction identity:

```text
CommandSubmissionAccepted
  -> RoutedEventStarted
    -> RouteSnapshotCreated
      -> EventPhaseInvoked*
        -> RoutedActionCollected / DelegatedCommandCollected*
          -> DefaultApplied | DefaultSuppressed
            -> RoutedEventCommitted
```

Outputs continue:

```text
RoutedActionCollected
  -> ActionSubmissionAccepted
    -> ApplicationTransactionStarted
```

```text
DelegatedCommandCollected
  -> CommandSubmissionAccepted
    -> RoutedEventStarted
```

`PropagationStopped` and `DefaultPrevented` identify the exact callback that
caused them. Processing-time rejection records the accepted command and exact
outcome without invoking a callback.

Trace admission reserves the complete operation-specific graph before the first
callback. Trace capacity zero changes no behavior. M4D1 may normalize and extend
the schema but does not repair missing M4C causal parentage.

## Slice boundaries

### M4C1 — Routed semantic-command kernel

Shared namespace and core value migration; core-owned `MountedNodeId`; semantic-
command protocol; `WidgetEventOutput`; checked bridge; immutable route;
propagation/default behavior; exact admission; queued submission; exact-target
source convergence; semantic `Activate`; route-only cancel/menu/context behavior;
trace foundations; direct activation-path removal.

### M4C2 — Displayed-generation surface context

`SurfaceId`; `SurfaceInputContext`; coordinate revision; retained current and
previous hit-test generations; configurable bounded retention; exact historical
targeting; retired, missing, foreign-runtime, and foreign-surface outcomes.

### M4C3 — Pointer lifecycle

Pointer/device identity; pointer/wheel payloads; logical scrolling; validation
order; physical path; pressed ownership; capture; boundaries; stationary-pointer
re-hit testing; multi-pointer ordering; terminal unavailable-context cleanup;
release-inside activation; pointer helper removal.

### M4C4 — Focus and modality

Focus scopes; next/previous and directional commands; DF-01–DF-20; scope policy;
exact-generation restoration; focus transition order; focus-within invalidation;
retained modality; normalized controller navigation.

### M4C5 — Keyboard, text, IME, automation, and M4C closure

Keyboard down/up; Enter repeat; Space pressed ownership; committed text; IME
composition ownership/cancellation; authored-ID automation resolution and
ambiguity; remaining direct input-path removal; complete Counter/downstream M4C
conformance.

### M4D1 — Complete trace schema

Complete event, surface, pointer, focus, composition, modality, and scheduler
trace normalization; logical causality; suppressed delivery; full M4
reconstruction fields.

### M4D2 — Export and sink

Versioned JSONL; default text/IME redaction; optional app labels without
`Action: Debug`; bounded/try sink; sink diagnostics; recursion guard; behavioral
isolation.

### M4D3 — Replay and M4 closure

Replay foundation; Counter reconstruction; final migration; public API/status/
support cleanup; every remaining matrix row; stable/MSRV validation; exact-head
CI; M4 completion.

## Acceptance rule

This charter becomes accepted M4C delivery authority only after explicit owner
acceptance. M4C0 then propagates it into the canonical roadmap, matrix, public
API, status/support documents, README, and changelog without changing behavioral
meaning. M4C1 cannot begin merely because this review-draft file exists on an
unmerged M4B branch.
