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

```text
Application-owned State + Action
  -> UiApp::root(State) -> typed View -> erased Element<Action>
  -> sibling-local mounted reconciliation
  -> persistent generational MountedTree
  -> state-aware cached widget capabilities
  -> synchronous mounted input/focus/activation policies
  -> UiApp::update(State, Action)
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
[ADR 0004](adr/0004-mounted-runtime-reconciliation.md). M4 architecture is now
under active owner review through revised proposed
[ADR 0005](adr/0005-canonical-event-routing-and-commands.md),
[ADR 0006](adr/0006-effects-scheduling-and-trace-v2.md), and the normative
[M4 conformance matrix](architecture/m4-conformance-matrix.md) and
[directional-focus corpus](architecture/m4-directional-focus-corpus.md); no M4
implementation support is implied.
See the [public API contract](architecture/public-api.md) and
[ADR 0003](adr/0003-extensible-view-widget-component-protocol.md).

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

Application update remains synchronous and application-state-owned. Proposed
[ADR 0006](adr/0006-effects-scheduling-and-trace-v2.md) adds one runtime-owned
FIFO action/work loop while preserving a simple two-argument update. `update`
returns optional ordered effects and `()` is the no-effects result. A separate
default-empty `initial_effects` authority describes one-time work after initial
mount, and a default-empty state-derived `subscriptions` authority declares
ongoing streams after initial mount and every successful action/reconciliation.

Effects request owned actions, tasks, timers, keyed cancellation/replacement,
typed application host requests, and completion actions; they begin only after
the owning update/reconciliation commits. Runtime-private generational IDs
protect completion safety, while applications use validated owner-local
`WorkKey` values as durable cancellation intent.

Mounted subscriptions are not imperative event work. The widget protocol
declares one complete state-derived desired set for an exact mounted owner after
committed mount. Later passes occur only after explicit owner-local invalidation.
Runtime reconciliation retains equal declarations, replaces changes, cancels
absence, rejects duplicate keys, and invalidates the generation before owner
unmount completes.

Wake and redraw use separate request/acknowledge state machines. Local non-`Send`
work remains possible, stronger bounds apply only to concrete operations that
require them, and configured saturation outcomes never silently drop accepted
actions or completions.

The pump uses explicit readiness checkpoints and separate processed-envelope,
completion-import, local-poll, and timer-promotion budgets. Readiness is accepted
and sequenced on the UI thread at the queue tail; budget exhaustion preserves
order, re-arms wake, and reports non-quiescent progress.

Each send-task start makes one executor attempt. Refusal is a recoverable terminal
outcome for that exact generation with no retry/pending queue or default action;
an optional UI-thread failure mapper may enqueue an action. The bounded canonical
trace remains authoritative over any bounded/try-based external sink, whose
failure cannot block, alter behavior, or recursively redeliver its diagnostic.

## Event model

The proposed canonical path is:

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

Proposed [ADR 0005](adr/0005-canonical-event-routing-and-commands.md)
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

Accessibility is mandatory for production controls. Semantics are renderer-independent and include stable identity, role, name, state, values, relationships, actions, bounds, and text ranges. Desktop platform bridges consume this semantic product.

## Hosts, surfaces, and renderers

One application runtime may own multiple logical surfaces that share application state and resources while retaining independent scale, layout roots, focus scopes, publication generations, and host lifecycle.

That is a later target. M3 is multi-surface-ready only because mounted IDs do not
encode platform windows and semantic identity is renderer-independent. The
current runtime has one mounted root, one active focus domain, and one current
publication domain; it has no independent per-surface focus, multiple roots,
surface lifecycle, cross-surface movement, or per-surface generations.

M4 adds an opaque single-domain surface-input context to event ingress. It names
the logical `SurfaceId`, coordinate-space revision, and exact retained displayed
generation; retired/foreign/missing contexts are rejected without retargeting.
This is a forward-compatible seam, not a claim that multi-surface lifecycle
exists before M10.

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
effects/scheduling/trace are revised proposed decisions in ADR 0005 and ADR 0006
and remain implementation gates until owner acceptance. Their normative public
proof requirements are fixed in the M4 conformance matrix and directional-focus
corpus.

The following later choices still require dedicated analysis and review:
standard layout algorithm, production text stack, conventional renderer, crate
extraction points, unsafe policy for host/backend crates, animation policy beyond
the M4 deterministic clock, and semver strategy for extensible enums and traits.

See the [roadmap](roadmap.md) for dependency gates and the [feature/support matrix](feature-support-matrix.md) for current coverage.
