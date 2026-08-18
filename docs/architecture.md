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
narrower shape. M4 and M5 are complete and owner-accepted. M5E's reviewed
feature head `7f3e0c9e881ff384516459db66436e662c5fb790` passed exact-head CI
#1294 / `32130312467`, was guarded-squash-merged in PR #67 as
`b07ae423d6a3573a4dd8a96a7ce5d6b5b1f0be1e`, and shares exact complete tree
`c5dc7fa000496d76c35e98f3a481fc1de5762f4c` with that squash. Accepted-main
CI #1296 / `32135074552` validated the exact squash through read-only PR #68,
which was closed unmerged. Final authority reconciliation PR #69 records all
five M5E rows as owner-accepted and closes M5 current-contract authority. M6 is
the next milestone and begins with the missing renderer-protocol architecture
gate, not with a backend or unreviewed #59 implementation.

The accepted M4 history remains unchanged: M4C3 was squash-merged in PR #15 as
`2fc165b9386f55c061d61232400375b13ad175bf`, M4C4 in
[PR #22](https://github.com/dornglut/runen-ui/pull/22) as
`f95571634a9c6528e5834e9589b048ad5197bd15`, M4C5 in
[PR #27](https://github.com/dornglut/runen-ui/pull/27) as
`284ecdcfe107e0a7afc88e4bf4fc82eecc52a226`, M4D1 in
[PR #39](https://github.com/dornglut/runen-ui/pull/39) as
`2fe269366386d7aee9de2a2573498b64ad486293`, M4D2 in
[PR #41](https://github.com/dornglut/runen-ui/pull/41) as
`8c67655ffce438c2e35e6478e7299bd704033b8b`, and M4D3 in
[PR #43](https://github.com/dornglut/runen-ui/pull/43) as
`596f0d823b9833d71a038cc4aebe834c7b94e4a6`. The final M4 authority
reconciliation closed M4 and activated M5.

```text
Application-owned State + Action
  -> UiApp::root(State) -> typed View -> erased Element<Action>
  -> sibling-local mounted reconciliation
  -> persistent generational MountedTree
  -> state-aware cached widget capabilities
  -> canonical owner-local SemanticContribution validation/reconciliation
  -> separate runtime semantic arena + owner/key bindings
  -> exact-target semantic-command, semantic-action, pointer, focus, keyboard, text, and composition capture/target/bubble routing
  -> one generalized sequenced work FIFO
  -> explicit four-budget readiness pump
  -> UiApp::update(State, Action) + complete reconciliation
  -> phase-aware topology/style/layout/paint/semantic/diagnostic facts
  -> provider-backed row/column measurement and arrangement when dirty
  -> SurfaceFrame + SurfaceStyleReport + SurfaceLayoutReport
  -> independent SemanticPublication + SemanticDiagnosticReport sibling products
  -> public downstream deterministic testing through runenui_testing
```

`MountedNodeId` and `SemanticNodeId` are distinct runtime-instance-local opaque
types that share the same runtime namespace but are issued by separate
runtime-owned generational arenas. `MountedNodeId` addresses one mounted widget
lifetime. `SemanticNodeId` addresses one exact semantic-node lifetime owned by a
mounted owner plus stable owner-local `SemanticKey`; it is not derived from the
mounted arena slot/generation. Compatible owner/key retention and semantic
contribution reorder preserve the semantic ID. Key removal or mounted owner
removal/replacement revokes the semantic lifetime; later semantic-slot reuse
advances generation and never retargets a stale ID. M5B publishes those IDs only
through the independent read-only semantic snapshot/update product. M5C accepts
exact current IDs only through `SemanticActionRequest` and resolves them against
the private mounted owner/key binding; neither product exposes a public semantic-
to-mounted routing shortcut. M5D testing targets retain exact semantic snapshot
surface/node scope and likewise expose no mounted-owner shortcut.

Unique sibling element keys reorder without changing mounted lifetime; unkeyed
children match by unkeyed ordinal; duplicate keys preserve no ambiguous mounted
lifetime; and cross-parent moves remount. Separately, semantic contributions use
`SemanticKey::PRIMARY` or validated named keys unique within the exact mounted
owner. The mounted tree owns widget state, lifecycle, focus, interaction slots,
invalidation, capability caches, the semantic arena/binding store, and
publication authority. Transient elements are consumed and not retained as a
parallel tree.

Public built-in and downstream widgets share the same state-aware checked
erasure bridge. `Widget::semantics(state, SemanticContributionContext)` returns
the canonical action-type-independent `SemanticContribution`: an ordered
owner-local forest containing zero or more semantic nodes plus the explicit
mounted-child splice marker where required. Contribution validation rejects
duplicate semantic keys, missing/duplicate/unnecessary mounted-child markers,
and missing owner-local relationship targets without first/last recovery or
implicit repair. Nodes carry platform-neutral role, name/description, value,
authored disabled/hidden/inert state, semantic action intent, relationships,
plain-text extension facts, and either exact owner bounds or validated
owner-local `LogicalRect` bounds. The M5 semantic action vocabulary is limited
to `Activate`, `RequestFocus`, `OpenMenu`, and `OpenContextMenu`;
`SemanticCommand::LogicalScroll` remains accepted routed M4 command authority and
is not semantic-node authoring.

M5B composes accepted owner contributions into one deterministic,
renderer-independent, exact-`SurfaceId` semantic product. Transparent mounted
owners splice child semantic roots without fabricated wrappers; explicit
mounted-child markers own placement order; publication-local lookup indexes
resolve exact owner bindings, visible owner/key targets, and unique/ambiguous
authored relationship owners while vectors/tree traversal remain the sole
observable ordering authority. Runtime derives absolute logical bounds, resolved
relationships, composed disabled state and supported-action identity, and
visible-PRIMARY semantic focus. Missing, hidden, stale, or ambiguous targets
fail closed with typed deterministic diagnostics rather than first/last fallback.

M5C adds exact public surface-scoped semantic action ingress through
`SemanticActionRequest` values constructed with
`SemanticActionRequest::new(surface, target, action)` and submitted by
`AppRuntime::submit_semantic_action`. Submission validates exact current surface,
semantic identity/binding, current publication membership, semantic freshness,
support, composed state, action-specific readiness, and canonical queue/work/
trace capacity without invoking widget callbacks. Accepted work joins the
existing command FIFO and retains the exact private surface/semantic/key/mounted-
owner/action binding. Queue-front processing revalidates before callbacks; stale
accepted work records one canonical processing rejection under the accepted
`WorkSequence` and never retargets. `RequestFocus` uses the accepted M4
Focusable/Automatic eligibility; PRIMARY and named activation keep their distinct
readiness rules; menu/context support does not acquire an unrelated actionable
gate. After callbacks, semantic `Activate` and `RequestFocus` defaults revalidate
again without synchronous semantic refresh. Explicit prevention and callback-
caused semantic invalidation remain distinct canonical trace outcomes. Semantic-
origin callback metadata is read-only and is not inherited by ordinary or
delegated commands. No semantic LogicalScroll, second queue/default engine, or
native accessibility adapter is introduced.

M5D adds the public downstream `runenui_testing` crate without moving runtime
authority out of `runenui_runtime`. `TestHarness<App>` composes one ordinary
`AppRuntime<App>` with deterministic public `ManualClock` authority, nonzero
configurable fixed-surface publication, explicit bounded pumping and finite
settling, snapshot-scoped semantic queries/targets, ordinary public pointer/
keyboard/text/composition/automation/action/command/semantic-action ingress, and
read-only state/focus/reconciliation/frame/layout/hit/paint/semantic/trace/replay
inspection. Semantic query ambiguity is explicit; scoped targets preserve exact
`SurfaceId + SemanticNodeId`; no testing helper reconstructs `MountedNodeId` or
guesses surface scope from a bare semantic ID. The crate enables no
`internal-test-seams`, hidden mutation bridge, wall-clock waiting, unbounded
settle, parallel runtime model, or semantic LogicalScroll compatibility path.

Compatible widget update is transactional; mismatch replaces in the current
mounted generation. Mount/update run in preorder, removal/replacement/shutdown
unmount in postorder while mounted arena occupancy remains live through each
hook, and state drops after removal. Semantic owner lifetimes are revoked before
mounted removal. Focus survives compatible updates and clears only when its
mounted lifetime or actionable/focusable facts cease to be valid. Semantic
contribution is cached separately: unrelated compatible updates do not requery
it, while widget semantic invalidation and direct mounted-child structural
change do. Layout dirtiness refreshes absolute semantic bounds without rerunning
unchanged semantic contribution, and routed focus changes dirty only the semantic
product. Invalid authored contribution withdraws that owner's semantics without
falsely marking mounted-state corruption; erased state/bridge mismatch and
semantic-index corruption fail closed and retain the integrity distinction.

Renderer-facing products no longer own or carry production semantics.
`SurfaceFrame`, `SurfaceNode`, and renderer debug output remain renderer-side
facts; `SemanticPublication` and `SemanticDiagnosticReport` are independent
mandatory sibling products of the complete `SurfacePublication`. Complete versus
renderer-only comparison/extraction is explicit so consumers cannot silently
omit semantic siblings. The semantic snapshot exposes deterministic roots/order
and exact-ID lookup without mutable runtime authority. Semantic revision begins
at 1, advances only on adapter-visible semantic change through checked
non-wrapping preflight, does not advance for diagnostics-only/readiness-only
changes, and produces deterministic added/changed/removed/root/focus deltas;
wrong surface or wrong/skipped prior revision requires full resynchronization.

Surface publication itself is one staged
`admit -> read-only/staged plan -> candidate-dependent final preflight -> commit`
transaction. Knowable runtime/status, publication-counter, required stationary
re-hit queue/work, trace-reservation, and redraw/control failures are preflighted
before downstream capability callbacks. Ordinary required stationary re-hit
queue `Full` is recoverable backpressure with zero cache/semantic/publication/
snapshot/trace/redraw/rehit commit and redraw still pending. Semantic identity,
capability, renderer/layout/hit/diagnostic candidates and fail-closed owner
withdrawals remain staged until candidate-dependent semantic revision preflight
passes. Redraw, hit-test, coordinate, and semantic revision exhaustion retain
exact typed terminal classifications; no wrap, saturation, reservation loss, or
partial publication is permitted.

The current layout and styling implementation is credible and retained: typed
style values and token resolution, concrete computed style, provenance, explicit
constraints, a borrowed measurement provider, component-wise intrinsic/child
minimum combination, computed padding, linear arrangement, and aligned
overflow/capability diagnostics. Canonical `LogicalSize` and `LogicalRect` are
core-owned host-neutral geometry types and runtime deliberately re-exports the
same authority where needed. Mounted capabilities are cached with explicit
integrity state. Operational phase planning and a retained proof publication
cache stores a topology-only mounted preorder snapshot, root constraints, an
exact style-token content snapshot, and the measurement provider's explicit
identity/revision compatibility promise. Tree changes rebuild all
topology-dependent renderer facts. Compatible style and layout phases use current
mounted `StyleIntent` and `LayoutStyle`, so literal, authored token-reference,
padding, and gap changes cannot be hidden by the topology cache. Clean or
isolated phases skip unrelated capability work; private phase-entry probes
independently verify the public execution report. Mounted index, frame, style,
and layout products share logical-preorder mounted IDs, parents, and authored
metadata for every live node; semantic IDs are intentionally published through
the separate M5B product. The current non-structural planner still deep-clones
whole `SurfaceCache` values; #59 owns replacement with persistent/staged retained
publication before or during M6 without weakening M5B atomicity.

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
M4. The accepted M4B implementation adds the core-owned application-work
contract, one ordered transaction planner, state-current application and
mounted subscription reconciliation, generational tasks/timers/host work,
four-budget readiness scheduling, wake/redraw handshakes, terminal closure, and
complete per-family causal scheduler trace proofs. The exact-target routed
semantic-command kernel is accepted through M4C1, displayed-generation surface
context through M4C2, pointer lifecycle through M4C3, focus scopes/modality
through M4C4, keyboard, committed-text/composition, plus deterministic
authored-ID automation resolution through M4C5, normalized in-memory trace
schema and full M4 causal reconstruction through M4D1, deterministic JSONL
projection plus subordinate bounded sink delivery through M4D2, and inert
offline JSONL replay plus final migration and closure proofs through M4D3.

M5A replaces the M2 semantic-proof callback authority with production semantic
contribution vocabulary and independent semantic lifetime storage. M5B completes
the renderer-independent semantic publication/update/diagnostic layer and clean
renderer semantic cutover described above. M5C completes exact surface-scoped
semantic-node action ingress/accessibility resolution through the existing M4
command/routed/default/trace authority. M5D completes the public deterministic
downstream testing harness described above. Recursive component action mapping
preserves semantic contribution content exactly. Core/runtime still has no
AccessKit/native dependency and no second semantic action queue. The accepted
[M5 semantics and testing charter](architecture/m5-semantics-and-testing-charter.md)
and [M5 conformance matrix](architecture/m5-conformance-matrix.md) define M5
acceptance. M5E's accepted integrated conformance/migration package completes M5
without changing the accepted runtime architecture or adding native adapter/M6
behavior.

The accepted M4C5 behavior does not add editable text, native IME objects, or a
platform host. Public automation work/trace-sequence exhaustion is a deliberate
recoverable exception that returns the exact authored request without
terminalizing; direct commands and already-accepted mutable work retain ordinary
terminal exhaustion policy. If mandatory composition cleanup cannot be
delivered, the runtime records causal suppression, retires the exact lifetime,
terminalizes, and preserves shutdown lineage rather than falsely claiming
callback delivery. M4D1's accepted canonical in-memory trace retains typed/
redacted input, composition, automation, action, terminal, shutdown,
logical-time, work-sequence, and causal-parent facts without a second history.
M4D2 projects those immutable records as deterministic JSONL v1, retains raw
committed text/preedit only under explicit independent `FullText` capture,
accepts optional static action labels without an `Action: Debug` bound, and uses
a subordinate lazily bounded sink whose serialization occurs only on consumer
drain. `Delivered`, `Full`, and first `Closed` remain same-record diagnostic
facts and consume no second trace sequence. M4D3 consumes only that serialized
projection in an inert offline causal model with replay-only identities,
contiguous retained-sequence and parent validation, explicit dropped-prefix
incompleteness, and Counter reconstruction after the live runtime is gone. M5D
exposes that accepted replay authority through its public harness without making
replay a runtime or semantic expectation engine. M5C semantic binding/rejection/
default records extend the same canonical schema and remain inert replay
observations. See the [public API contract](architecture/public-api.md),
[ADR 0003](adr/0003-extensible-view-widget-component-protocol.md), and
[work-tracking contract](work-tracking.md).

## Ownership rules

- Durable application meaning belongs to application state.
- Ephemeral interaction mechanics belong to mounted widget state.
- Widget semantic authoring owns platform-neutral owner-local contribution only;
  runtime owns live semantic identity, private mounted-owner/key resolution,
  absolute bounds, semantic focus projection, publication revisions,
  relationship resolution, support composition, and exact semantic action
  admission/routing authority.
- Native resources and platform state belong to the host.
- Renderer resources belong to the renderer/resource layer.
- Components compose views and map local actions; widgets declare runtime
  participation/state contracts; mounted widgets are persistent runtime instances.
- Mounted runtime mutation occurs on one logical UI thread.
- Public host-neutral protocol/value definitions live in `runenui_core`; the
  live namespace, mounted/semantic storage authority, routing, scheduler, host
  integration, trace, shutdown, semantic publication, and semantic action
  resolution live in `runenui_runtime`, which depends on core.
- Public deterministic testing convenience lives in downstream
  `runenui_testing`, which depends on core/runtime and must not become runtime
  authority or enable private test seams.

External crates can define widgets and participate in mounted state, lifecycle,
activation, layout, paint, canonical semantic contribution, diagnostic,
invalidation, semantic-publication consumption, semantic action submission,
public deterministic harness testing, and inspection paths without modifying
RunenUI. M6–M8 own the remaining production subsystem contracts before
host/backend production work.

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
trace remains authoritative over the accepted M4D2 subordinate export sink.
Atomic logical-capacity reservation bounds pending immutable record references;
transport does not wait for receiver capacity, and JSON encoding occurs only
when the consumer drains the receiver. Full or closed sink state cannot alter
canonical sequence/order or application behavior and cannot recursively create
or redeliver diagnostic records.

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
only when its default is not prevented. Exact semantic action admission is a
separate checked ingress into this same command/routed/default path; it does not
create another dispatch engine.

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

Accessibility is mandatory for production controls. M5A implements the
platform-neutral authoring half of semantic roles, labels/descriptions, values,
authored state/action intent, relationships, bounds policy, text facts, and
independent runtime semantic identity. M5B implements the renderer-independent,
exact-surface semantic snapshot/update/diagnostic product with absolute bounds,
resolved relationships, composed state/support and visible-PRIMARY focus. M5C
implements exact `SurfaceId + SemanticNodeId + SemanticAction` ingress through
private mounted-owner/key resolution into the existing command/routed/default/
trace authority, with exact queue-front and post-callback revalidation. M5D adds
public deterministic semantic queries/testing. Platform adapters such as
AccessKit map from the accepted semantic product and M5C action ingress; renderer
output is not the accessibility model and AccessKit is not authoritative in
core/runtime.

## Hosts, surfaces, and renderers

One application runtime may own multiple logical surfaces that share application state and resources while retaining independent scale, layout roots, focus scopes, publication generations, and host lifecycle.

That is a later target. The mounted representation is multi-surface-ready because
mounted IDs do not encode platform windows and semantic identity is renderer-
independent. The current runtime has one mounted root, one active focus domain,
and one current publication domain; it has no independent per-surface focus,
multiple roots, surface lifecycle, cross-surface movement, or per-surface
generations.

Accepted M4 architecture includes an opaque single-domain surface-input context
in event ingress. It names the logical `SurfaceId`, coordinate-space revision,
and exact retained displayed generation; retired/foreign/missing contexts are
rejected without retargeting. This is current single-domain headless proof
behavior and a forward-compatible seam, not a claim that multi-surface lifecycle
or per-window host integration exists before M10. M5 semantic products/actions
reuse exact opaque `SurfaceId` scope without pulling M10 lifecycle forward.

The required profiles are headless/test, standalone desktop, and embedded host. The renderer-neutral scene protocol is stabilized first, then proven by deterministic consumers, one conventional desktop backend, and only afterward an embedded/SDF consumer.

## Current workspace boundary

The active workspace intentionally contains `runenui_core`, `runenui_runtime`,
the public downstream `runenui_testing` crate, the `counter` example, the non-
publishable test-owned `runenui_external_widget_conformance` package, and
`xtask`. New crates require real ownership, dependency, optionality, independent-
consumer, or conformance pressure. A target crate diagram is not permission to
create empty crates, and the facade crate is deferred until lower-level APIs
warrant a stable public surface. `runenui_testing` is deliberately downstream of
runtime and owns testing ergonomics only; runtime does not depend on it.

## Required ADRs before implementation choices

The View/Widget/type-erasure protocol and mounted reconciliation/storage
decisions are accepted in ADR 0003 and ADR 0004. Event routing/commands and
effects/scheduling/trace are accepted in ADR 0005 and ADR 0006 and define the
completed M4 runtime authorities. The current implementation contains their
accepted queue, scheduler, routed semantic-command, displayed-generation
surface, pointer, focus/modality, keyboard/text/composition/automation,
terminal/shutdown, M4D1-normalized in-memory trace, M4D2 deterministic
JSONL/redaction/bounded-sink authority, and M4D3 inert offline replay/final
closure proof surface. Public M4 proof requirements are fixed in the M4
conformance matrix and directional-focus corpus.

The accepted M5 semantics/testing charter and M5 conformance matrix own the
completed semantics/testing program. M5A, #55, M5B, M5C, M5D, and M5E are
accepted; the final M5 reconciliation closes current-contract authority without
adding M6 behavior. M6 is next, but its roadmap explicitly requires an accepted
render-protocol ADR before renderer-neutral scene implementation. Issue #59 is a
bounded retained-publication readiness concern to resolve before or during M6
within that accepted scene/publication design.

The following later choices still require dedicated analysis and review:
standard layout algorithm, production text stack, conventional renderer, crate
extraction points, unsafe policy for host/backend crates, animation policy beyond
the M4 deterministic clock, and semver strategy for extensible enums and traits.

See the [roadmap](roadmap.md) for dependency gates, the
[feature/support matrix](feature-support-matrix.md) for current coverage, and
[work tracking](work-tracking.md) for volatile execution state.