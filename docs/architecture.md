# Architecture

> **Category: Target architecture**
>
> Current implementation facts are explicitly identified below. Unqualified pipeline descriptions are accepted targets, not implemented APIs.

RunenUI separates application state, transient authoring, persistent runtime identity, interaction, style, layout, semantics, hit testing, paint extraction, host integration, and rendering.

## Accepted target pipeline

```text
Application state
  -> root/view
Transient owned View/Element tree
  -> keyed reconciliation
Persistent mounted runtime tree
  -> computed style
  -> layout
  -> semantic tree + hit-test scene + paint scene
  -> host accessibility/event integration + renderer backend
```

The mounted tree is the runtime authority. It retains generational node identity, parent/child structure, widget-local state, lifecycle, invalidation, focus, hover, pressed state, pointer capture, scrolling state, semantic identity, and task/subscription ownership. The authored tree remains cheap, declarative, and transient.

The output products are deliberately distinct:

```text
Mounted tree
Semantic tree
Layout result
Hit-test scene
Paint/primitive scene
Diagnostics
```

A renderer consumes paint primitives and resources. It does not interpret semantic widget kinds such as `Button`. Hit testing consumes explicit hit-test data and remains independent of the renderer.

## Current implementation

The current implementation is a deterministic mounted headless proof with this
narrower shape:

M4A, M4B, M4C0–M4C5, and M4D1 are complete and owner-accepted. The accepted
M4C3 feature head `01b7ae018abeaff8d316764afba5bc8cde074381` passed exact-head CI
run `29996101708` and was squash-merged in PR #15 as
`2fc165b9386f55c061d61232400375b13ad175bf`. The accepted M4C4 feature head
`f3201a83583af0c1d148bec87cd9140ff42795b7` passed exact-head CI run
`30006170403` and was squash-merged in
[PR #22](https://github.com/dornglut/runen-ui/pull/22) as
`f95571634a9c6528e5834e9589b048ad5197bd15`. The accepted M4C5 feature head
`d0d2ef1d53a8ab1d940beb4155f5f991229f042e` passed exact-head CI run
`30843238697`, passed independent rereview, and was squash-merged in
[PR #27](https://github.com/dornglut/runen-ui/pull/27) as
`284ecdcfe107e0a7afc88e4bf4fc82eecc52a226`. The accepted M4D1 feature head
`990c49edb5b68c37dd3b7d37dd3f1196a9557c7a` passed canonical exact-head CI run
`31269401262` / #657 and the frozen complete-diff review, and was squash-merged in
[PR #39](https://github.com/dornglut/runen-ui/pull/39) as
`2fe269366386d7aee9de2a2573498b64ad486293`. M4D2 remains blocked until this
post-merge authority reconciliation is accepted and merged; M4D3 remains blocked
behind M4D2. The target M4 pipeline is active and incomplete.

```text
Application-owned State + Action
  -> UiApp::root(State) -> typed View -> erased Element<Action>
  -> sibling-local mounted reconciliation
  -> persistent generational MountedTree
  -> state-aware cached widget capabilities
  -> exact-target semantic-command, pointer, focus, keyboard, text, and composition capture/target/bubble routing
  -> one generalized sequenced work FIFO
  -> explicit four-budget readiness pump
  -> UiApp::update(State, Action) + complete reconciliation
  -> phase-aware topology/style/layout/paint/semantic/diagnostic facts
  -> provider-backed row/column measurement and arrangement when dirty
  -> SurfaceFrame + SurfaceStyleReport + SurfaceLayoutReport
```

`MountedNodeId` and `SemanticNodeId` are separate runtime-instance-local types
containing an opaque `Arc` token, arena slot, and non-wrapping generation.
Unique sibling keys reorder without changing lifetime; unkeyed children match by
unkeyed ordinal; duplicate keys preserve no ambiguous lifetime; and cross-parent
moves remount. The mounted tree owns widget state, lifecycle, focus, interaction
slots, invalidation, capability caches, and publication authority. Transient
elements are consumed and not retained as a parallel tree.

Public built-in and downstream widgets share the same state-aware checked
erasure bridge. Compatible update is transactional; mismatch replaces in the
current generation. Mount/update run in preorder, removal/replacement/shutdown
unmount in postorder while arena occupancy remains live through each hook, and
state drops after removal. Focus survives compatible
updates and clears only when its mounted lifetime or actionable/focusable facts
cease to be valid. `SurfaceFrame` still carries open proof facts, not the M5
semantic tree or M6 renderer-neutral paint protocol.

The current layout and styling implementation is credible and retained: typed
style values and token resolution, concrete computed style, provenance, explicit
constraints, a borrowed measurement provider, component-wise intrinsic/child
minimum combination, computed padding, linear arrangement, and aligned
overflow/capability diagnostics. Mounted capabilities are cached with explicit
integrity state. Operational phase planning and a retained proof publication
cache stores a topology-only mounted preorder snapshot, root constraints, an
exact style-token content snapshot, and the measurement provider's explicit
identity/revision compatibility promise. Tree changes rebuild all
topology-dependent facts. Compatible style and layout phases use current mounted
`StyleIntent` and `LayoutStyle`, so literal, authored token-reference, padding,
and gap changes cannot be hidden by the topology cache. Clean or isolated phases
skip unrelated capability work; private phase-entry probes independently verify
the public execution report. Mounted index, frame, style, and layout products
share logical preorder mounted IDs, semantic IDs, parents, and authored metadata
for every live node.

M1 repaired the proof surface around this implementation: logical distances and
sizes are validated, typed builders prevent incompatible configuration, child
composition has no arity ceiling, Unicode identifier identity is independent of
static/owned storage, identity/token duplicates use true preorder, finite derived
geometry saturates, and generated products are read-only. M2 then removed the
closed dispatch path, added recursive typed component action mapping, explicit
process-local widget/state type identity, and a checked lifecycle/state seam.
M3 replaces the seam with the mounted authority described by
[ADR 0004](adr/0004-mounted-runtime-reconciliation.md). Accepted
[ADR 0005](adr/0005-canonical-event-routing-and-commands.md),
[ADR 0006](adr/0006-effects-scheduling-and-trace-v2.md), the normative
[M4 conformance matrix](architecture/m4-conformance-matrix.md), and the
[directional-focus corpus](architecture/m4-directional-focus-corpus.md) define
M4. The current M4B implementation adds the core-owned application-work
contract, one ordered transaction planner, state-current application and
mounted subscription reconciliation, generational tasks/timers/host work,
four-budget readiness scheduling, wake/redraw handshakes, terminal closure, and
complete per-family causal scheduler trace proofs. The exact-target routed
semantic-command kernel is accepted through M4C1, displayed-generation surface
context through M4C2, pointer lifecycle through M4C3, focus scopes/modality
through M4C4, keyboard, committed-text/composition, plus deterministic
authored-ID automation resolution through M4C5, and normalized in-memory trace
schema and full M4 causal reconstruction through M4D1. The accepted M4C5 behavior
does not add editable text, native IME objects, a platform host, or semantic
accessibility resolution. Public automation work/trace-sequence exhaustion is a
deliberate recoverable exception that returns the exact authored request without
terminalizing; direct commands and already-accepted mutable work retain ordinary
terminal exhaustion policy. If mandatory composition cleanup cannot be delivered,
the runtime records causal suppression, retires the exact lifetime, terminalizes,
and preserves shutdown lineage rather than falsely claiming callback delivery.
M4D1's accepted canonical in-memory trace retains typed/redacted input,
composition, automation, action, terminal, shutdown, logical-time, work-sequence,
and causal-parent facts without a second history. Deterministic JSONL export,
external sink delivery, and replay remain unimplemented M4D2/M4D3 target
architecture. See the [public API contract](architecture/public-api.md),
[ADR 0003](adr/0003-extensible-view-widget-component-protocol.md), and
[work-tracking contract](work-tracking.md).

## Ownership rules

- Durable application meaning belongs to application state.
- Ephemeral interaction mechanics belong to mounted widget state.
- Native resources and platform state belong to the host.
- Renderer resources belong to the renderer/resource layer.
- Components compose views and map local actions; widgets declare runtime
  participation/state contracts; mounted widgets are persistent runtime instances.
- Mounted runtime mutation occurs on one logical UI thread.
- Public host-neutral protocol/value definitions live in `runenui_core`; the
  live namespace, mounted/storage authority, routing, scheduler, host
  integration, trace, and shutdown live in `runenui_runtime`, which depends on
  core. M4 introduces no third authority crate.

External crates can define widgets and participate in mounted state, lifecycle,
activation, layout, paint, semantic, diagnostic, invalidation, and inspection
paths without modifying RunenUI. M4–M8 own the remaining production subsystem
contracts.

## Application and effect model

The primary application model remains:

```text
state -> view -> action -> update -> state
```

Application update remains synchronous and application-state-owned. The current
implementation uses one runtime-owned generalized FIFO and a four-budget
explicit pump while preserving the two-argument `()` no-effects update. Direct
dispatch is not an authority. The core-owned contract from
[ADR 0006](adr/0006-effects-scheduling-and-trace-v2.md) implements ordered update
effects, default-empty `initial_effects`, and default-empty state-derived
application subscriptions.

Effects request owned actions, tasks, timers, keyed cancellation/replacement,
typed application host requests, and completion actions; they begin only after
the owning update/reconciliation commits. Runtime-private generational IDs
protect completion safety, while applications use validated owner-local
`WorkKey` values as durable cancellation intent.

The private work registry stores pending-start and running generations only.
Completion, refusal, cancellation, owner invalidation, and scheduler closure
remove records and keyed bindings immediately; stale queued envelopes resolve
by generation absence rather than retained terminal tombstones.

Mounted subscriptions are not imperative event work, and declaration values are
not retained as caches. The widget protocol
declares one complete state-derived desired set for an exact mounted owner after
committed mount. Later passes occur only after explicit owner-local invalidation.
The declaration callback runs against newest live widget state only when its
queued exact-owner reconciliation envelope reaches the queue front; stale-owner
envelopes suppress the callback.
Runtime reconciliation retains equal declarations, replaces changes, cancels
absence, rejects duplicate keys, and invalidates the generation before owner
unmount completes.

Local subscription sources implement a wake-aware `poll_next` protocol and are
polled only when eligible, at most once per readiness checkpoint. They share
creation-order authority and the `max_local_polls`/`polled_local_work` budget
with local tasks, so a sleeping source permits quiescence. Send subscription
sources are owned `Send` producers given one nonblocking start attempt with a
structured started/unavailable/full/closed/rejected outcome. Their ingress is
`Starting` during the callback and promotes to `Running` only after `Started`;
synchronous sends return the exact item as `NotStarted`. Concrete items
enter bounded completion ingress; full, closed, or stale submission returns the
exact item, and the UI-thread mapper runs only after the generation is validated
live.

Wake and redraw use separate request/acknowledge state machines. Local non-`Send`
work remains possible, stronger bounds apply only to concrete operations that
require them, and configured saturation outcomes never silently drop accepted
actions or completions. Queue and canonical-trace capacities are logical limits;
their storage grows with accepted work rather than reserving the complete
configured limit when the runtime mounts.

Terminal integrity and explicit shutdown share one idempotent scheduler-closure
authority. It closes completion/wake producers without invoking the external
wake transport, drains the queue and live registry, and clears every retained
task, timer, subscription, mapper, host payload/reservation, and pending
declaration. Subscription diagnostics use an independently configured bounded
oldest-first retention limit.

Creating a detached send-capable host response does not reserve its request.
One lock-protected `Open` response state admits exactly one detached ingress,
direct completion, or cancellation transition. Full detached ingress leaves it
open for exact-completion retry; cancellation removes an already queued detached
payload and the response slot before UI mapping. Terminal generations are
absence, never retained response tombstones.

Exact-generation revocation is one scheduler authority spanning registry,
producer ingress, completion payloads, futures, timers, sources, mappers, and
host requests. Mounted removal/replacement invokes it before the unmount hook.
Mandatory trace admission is checked and operation-specific, and enabled-trace
accepted actions use their own acceptance fact as causal parent. Wake request,
transport, delivery claims, and callback-in-flight state share one state mutex;
host callbacks run outside all framework synchronization guards, remain
serialized, and are claimed at most once per outstanding request.

The current pump applies separate processed-envelope, completion-import,
local-work-poll, and timer-promotion budgets at deterministic readiness checkpoints.
Budget exhaustion preserves application-action order, reports all remaining
serviceable work and future deadlines, and re-arms the coalesced wake edge when
work remains.

Each send-task start makes one executor attempt. Refusal is a recoverable terminal
outcome for that exact generation with no retry/pending queue or default action;
an optional UI-thread failure mapper may enqueue an action. The bounded canonical
trace remains authoritative over any bounded/try-based external sink, whose
failure cannot block, alter behavior, or recursively redeliver its diagnostic.

## Event model

The accepted target canonical path is:

```text
Host event or synthetic command
  -> sequenced ingress + surface-input validation
  -> target resolution
  -> capture / target / bubble
  -> semantic default behavior
  -> staged interaction commit
  -> commit-derived notifications
  -> queued application actions/commands/work
  -> update and reconciliation
```

Accepted [ADR 0005](adr/0005-canonical-event-routing-and-commands.md)
fixes route snapshotting, observable target/current-target/phase facts,
non-reentrant propagation, independent stop-propagation/default-prevention,
pointer identity/capture, exact displayed-generation surface input, focus scopes,
composition lifetime, deterministic transition ordering, and semantic command
convergence.

Pointer ingress tracks routed and physical targets separately. Boundary events
are deterministic, and retained pointer positions are re-hit-tested when layout
or hit-test generations change so stationary-pointer hover cannot become stale.
Retired/missing ordinary input is never retargeted, but same-runtime/surface
terminal up/cancel for an active pointer performs integrity-only pressed/capture/
stream cleanup; foreign runtime/surface input never mutates local state.
Default pointer activation is press, capture, pressed-state update, release, then
semantic activation only if the same mounted lifetime remains live, enabled, and
inside. Keyboard commands and text/IME input are separate event streams.

Focus and activation commands retain framework defaults. Unconsumed cancel/back,
menu, context-menu, and logical-scroll commands deterministically produce no
action or runtime mutation after their single capture/target/bubble route.
Delegation is explicit queued output, and wheel emits exactly one scroll command
only when its default is not prevented.

The authored semantic callback becomes `on_activate`; the physical-phase term
`on_press` is removed without a pre-1.0 compatibility alias.

## Layout and styling

RunenUI owns public layout semantics, constraints, results, diagnostics, and custom-layout extension points. A mature layout algorithm may be adopted behind an adapter only after an adopt-versus-build ADR; dependency vocabulary must not leak into RunenUI’s public contract.

Style resolution follows this conceptual order:

```text
platform and user preferences
  -> theme tokens
  -> control recipe
  -> variant
  -> interaction state
  -> local override
  -> computed style
```

Interaction-state recipes wait for mounted hover, pressed, focus, and disabled state. Layout-affecting style values must not form a disconnected parallel configuration model.

## Text and accessibility

RunenUI will use a mature text stack behind RunenUI-owned contracts; it will not implement Unicode shaping from scratch. Production text includes fallback, shaping, bidi, line breaking, wrapping, baselines, editing, selection, caret, clipboard, IME, and accessible text ranges.

Accessibility is mandatory for production controls. Semantics include stable identity, roles, labels, values, relationships, actions, bounds, hidden/inert state, and text ranges where relevant. Platform adapters such as AccessKit map from the semantic tree; renderer output is not the accessibility model.

## Hosts, surfaces, and renderers

One application runtime may own multiple logical surfaces that share application state and resources while retaining independent scale, layout roots, focus scopes, publication generations, and host lifecycle.

That is a later target. The mounted representation is multi-surface-ready because
mounted IDs do not encode platform windows and semantic identity is renderer-
independent. The current runtime has one mounted root, one active focus domain,
and one current publication domain; it has no independent per-surface focus,
multiple roots, surface lifecycle, cross-surface movement, or per-surface
generations.

Accepted M4 architecture adds an opaque single-domain surface-input context to
event ingress. It names the logical `SurfaceId`, coordinate-space revision, and
exact retained displayed generation; retired/foreign/missing contexts are
rejected without retargeting. This is a forward-compatible target seam, not a
claim that surface-input handling or multi-surface lifecycle exists in the
current implementation or before M10.

The required profiles are headless/test, standalone desktop, and embedded host. The renderer-neutral scene protocol is stabilized first, then proven by deterministic consumers, one conventional desktop backend, and only afterward an embedded/SDF consumer.

## Current workspace boundary

The active workspace intentionally contains `runenui_core`, `runenui_runtime`,
the `counter` example, the non-publishable test-owned
`runenui_external_widget_conformance` package, and `xtask`. New crates require
real ownership, dependency, optionality, independent-consumer, or conformance
pressure. A target crate diagram is not permission to create empty crates, and
the facade crate is deferred until lower-level APIs warrant a stable public
surface.

## Required ADRs before implementation choices

The View/Widget/type-erasure protocol and mounted reconciliation/storage
decisions are accepted in ADR 0003 and ADR 0004. Event routing/commands and
effects/scheduling/trace are accepted in ADR 0005 and ADR 0006 and are the active
M4 implementation authorities. The current implementation contains their
accepted queue, scheduler, routed semantic-command, displayed-generation
surface, pointer, focus/modality, keyboard/text/composition/automation,
terminal/shutdown, and M4D1-normalized in-memory trace authority. M4D2
export/sink, M4D3 replay, and final milestone closure remain implementation
gates. Public proof requirements are fixed in the M4 conformance matrix and
directional-focus corpus.

The following later choices still require dedicated analysis and review:
standard layout algorithm, production text stack, conventional renderer, crate
extraction points, unsafe policy for host/backend crates, animation policy beyond
the M4 deterministic clock, and semver strategy for extensible enums and traits.

See the [roadmap](roadmap.md) for dependency gates, the
[feature/support matrix](feature-support-matrix.md) for current coverage, and
[work tracking](work-tracking.md) for volatile execution state.
