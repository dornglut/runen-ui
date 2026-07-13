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
[ADR 0004](adr/0004-mounted-runtime-reconciliation.md).
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

External crates can define widgets and participate in mounted state, lifecycle,
activation, layout, paint, semantic, diagnostic, invalidation, and inspection
paths without modifying RunenUI. M4–M8 own the remaining production subsystem
contracts.

## Application and effect model

The primary application model remains:

```text
state -> view -> action -> update -> state
```

Simple applications retain a synchronous update form. Production applications gain an explicit effects result or collector. Effects request tasks, timers, subscriptions, cancellation, host commands, wakeups, and completion actions; execution remains owned by the runtime/host and deterministic in tests.

## Event model

The accepted canonical path is:

```text
Host event
  -> normalization
  -> capture phase
  -> target phase
  -> bubble phase
  -> semantic default behavior
  -> application action
  -> update
  -> effects and invalidation
```

Pointer, keyboard, accessibility, automation, and programmatic activation converge on semantic control commands. Default pointer activation for an actionable widget is press, capture, pressed-state update, release, then activation only if release remains valid. Keyboard commands and text/IME input are separate event streams.

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

The following decisions require dedicated analysis and review: public View/Widget/type-erasure protocol; reconciliation storage and identity; event API; effects API; standard layout algorithm; production text stack; conventional renderer; crate extraction points; unsafe policy for host/backend crates; animation/time; and semver strategy for extensible enums and traits.

See the [roadmap](roadmap.md) for dependency gates and the [feature/support matrix](feature-support-matrix.md) for current coverage.
