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

The current implementation is a deterministic headless proof with this narrower shape:

```text
Application-owned State + Action
  -> UiApp::root(State) -> typed View -> erased Element<Action>
  -> transient preorder RuntimeTreeIndex
  -> synchronous input/focus/activation policies
  -> UiApp::update(State, Action)
  -> full root rebuild and focus clear
  -> per-publication style resolution
  -> provider-backed row/column measurement and arrangement
  -> SurfaceFrame + SurfaceStyleReport + SurfaceLayoutReport
```

Current `RuntimeNodeId` values are preorder indexes for one built tree.
`ElementKey` is stored but is not used for reconciliation. M2 replaced the
closed kind enum with public typed `Widget<Action>` implementations erased safely
inside `Element<Action>`. Public built-in views convert to private behavior-only
widget implementations; downstream leaf widgets use `Element::new`, while
downstream and built-in child-layout widgets use the canonical
`Container<Action>` builder. All share activation, intrinsic measurement,
child-layout, paint-proof, semantic-proof, diagnostic, lifecycle-contract, and
inspection paths. M2 state is available only to the isolated lifecycle proof;
M3 must introduce the state-aware mounted behavior contract. `SurfaceFrame`
carries open proof facts, not the accepted M5
semantic tree or M6 renderer-neutral paint protocol.

The current layout and styling implementation is credible and retained: typed style values and token resolution, concrete computed style, provenance, explicit constraints, a borrowed measurement provider, separate one-query intrinsic and child-layout snapshots, component-wise intrinsic/child minimum combination, computed padding, linear arrangement, and aligned overflow/capability diagnostics. Index, frame, style, and layout products share preorder IDs and parents for every valid node.

M1 repaired the proof surface around this implementation: logical distances and
sizes are validated, typed builders prevent incompatible configuration, child
composition has no arity ceiling, Unicode identifier identity is independent of
static/owned storage, identity/token duplicates use true preorder, finite derived
geometry saturates, and generated products are read-only. M2 then removed the
closed dispatch path, added recursive typed component action mapping, explicit
process-local widget/state type identity, and a checked lifecycle/state seam.
See the [public API contract](architecture/public-api.md) and
[ADR 0003](adr/0003-extensible-view-widget-component-protocol.md).

## Ownership rules

- Durable application meaning belongs to application state.
- Ephemeral interaction mechanics belong to mounted widget state.
- Native resources and platform state belong to the host.
- Renderer resources belong to the renderer/resource layer.
- Components compose views and map local actions; widgets declare runtime
  participation/state contracts; mounted widgets are persistent M3 instances.
- Mounted runtime mutation occurs on one logical UI thread.

External crates can define widgets and participate in the current proof-level
activation, layout, paint, semantic, diagnostic, lifecycle-conformance, and
inspection paths without modifying RunenUI. M3–M8 own persistent reconciliation
and the respective production subsystem contracts.

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
