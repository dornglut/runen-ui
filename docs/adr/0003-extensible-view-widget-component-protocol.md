# ADR 0003: Use Erased Transient Elements over Typed Widget Implementations

> **Category: ADR**
>
> **Status:** Accepted
>
> **M3 note:** ADR 0004 supersedes this ADR's provisional lifecycle-only state,
> transient preorder targeting, and transient publication sections. The M2
> view/element/widget/component vocabulary and open safe-erasure direction remain
> historical inputs and active where ADR 0004 does not replace them.
>
> **Decision date:** 2026-07-12
>
> **Reviewed baseline:** `19ff06c77d1d21d04ea54c8193f4a206663d0975`

## Context

The M1 authoring proof stores `Text`, `Button`, and `Container` in the closed
`ElementKind<Action>` enum. Runtime traversal, focusability, activation,
measurement, arrangement, surface publication, and debug rendering all match
that enum. A downstream control therefore requires coordinated edits in both
framework crates even though its behavior is otherwise host- and
renderer-neutral. Adding `ElementKind::Custom` would move the same registration
gate behind one escape hatch and leave built-ins privileged.

M2 must establish the protocol that M3 can reconcile without implementing M3's
persistent mounted tree. It must preserve application-owned state and typed
actions, builder-first Rust authoring, safe Rust, host/renderer neutrality, and
local rather than global generic bounds.

## Vocabulary and representation

- A **view** is any typed, transient Rust value implementing `View<Action>` and
  able to produce one erased `Element<Action>`. Built-in builders and downstream
  widget descriptions are views.
- An **element** is the owned, transient, type-erased node produced by
  a view. It stores authored ID/key, style/layout intent, diagnostics, children,
  and one erased widget implementation. It is rebuilt from application state
  and is not runtime identity or runtime-local state.
- A **component** is an ordinary Rust function or type that composes views. It
  may author a local action type and map it into its parent's action type. A
  component is not automatically a runtime node, identity owner, or lifecycle
  participant.
- A **widget** is a concrete implementation of the public `Widget<Action>`
  protocol. It declares its runtime-local state type and contributes bounded
  activation, measurement, paint-proof, semantic-proof, diagnostic, and
  lifecycle behavior.
- A **mounted widget** is a future M3 runtime instance pairing reconciled
  identity, a compatible widget implementation, and persistent runtime-local
  state. M2 defines compatibility and state/lifecycle operations but does not
  create a production mounted tree.
- A **built-in authored view** such as `Text`, `Button<Action>`, or
  `Container<Action>` owns validated authoring facts and converts once into an
  element containing a private behavior-only widget implementation. External
  widgets use the identical erased runtime protocol through `Element::new` or
  the canonical child-bearing container builder.

The names are intentionally non-overlapping. `View` is the conversion protocol;
`Element` is its erased transient product; `Widget` is runtime-participant
behavior; `Component` is composition.

## External designs considered

The decision is informed by primary framework documentation and source:

- [Xilem's architecture](https://docs.rs/crate/xilem/latest/source/ARCHITECTURE.md)
  separates a lightweight rebuilt view tree from a retained element/widget tree,
  treats ordinary functions as components, and erases only where dynamic
  reconfiguration requires it. RunenUI adopts the view/component distinction and
  reconciliation direction, but not Xilem's full generic view protocol or
  reactive dependency model in M2.
- [Iced's `Element`](https://docs.iced.rs/iced/type.Element.html),
  [`Widget`](https://docs.iced.rs/iced/advanced/widget/trait.Widget.html), and
  [`tree::Tag`](https://docs.iced.rs/iced/advanced/widget/tree/struct.Tag.html)
  demonstrate owned trait-object erasure, typed message mapping, and `TypeId`-
  backed state compatibility. RunenUI adopts those narrow ideas, but keeps proof
  capabilities renderer-neutral and avoids making one renderer/event/layout
  super-trait the permanent production API.
- [Druid's `WidgetPod`](https://docs.rs/druid/latest/druid/struct.WidgetPod.html)
  owns hierarchy participation and runtime widget state while forwarding
  lifecycle, update, layout, event, and paint operations. RunenUI adopts the
  runtime-owned state/lifecycle boundary; M2 deliberately stops before a pod,
  arena, or persistent lifecycle execution path.
- [egui's documented immediate-mode model](https://github.com/emilk/egui#why-immediate-mode)
  shows the authoring simplicity of ordinary functions and acknowledges that
  retained interaction state still needs IDs and memory. RunenUI keeps the
  function-authoring benefit but rejects pure immediate identity because M3 must
  preserve keyed runtime state, focus, capture, lifecycle resources, and
  granular invalidation across rebuilds.
- [Slint's generated component contract](https://docs.rs/slint/latest/slint/#generated-components)
  cleanly separates exported components from runtime implementation. RunenUI
  adopts the explicit component boundary but rejects code generation and a
  second UI language as its semantic foundation.

## Decision

### View conversion and element ownership

Replace `IntoElement<Action>` with the single public `View<Action>` conversion
trait. `View::into_element` consumes a typed builder or existing element and
produces `Element<Action>`. `IntoElements` is removed; `Views<Action>` remains a
narrow heterogeneous/homogeneous child-collection conversion and is not another
single-view protocol. `element!` and `children!` continue as thin calls into
these public traits; builders remain authoritative.

`Element<Action>` owns a safe erased widget box and a separate vector of child
elements. Keeping children at the element level lets traversal, authored
identity, style, diagnostics, recursive action mapping, and future
reconciliation operate without concrete widget matches. Widget behavior never
requires global registration.

Public built-in authored types never implement `Widget<Action>` directly.
`Text::into_element` transfers content and common authored facts into an element
with private `TextWidget`; `Button::into_element` transfers label, enabled/action
source, and common facts into private `ButtonWidget<Action>`. Runtime payloads
contain behavior facts only—never duplicate ID, key, style, layout, children, or
authoring diagnostics. Consequently `Element::new(text(...))` and the button
equivalent fail to compile instead of silently discarding builder configuration.

Child ownership is inseparable from child layout. `ChildLayoutWidget<Action>`
extends `Widget<Action>` with `child_layout() -> ChildLayout`; M2 currently
supports `ChildLayout::Linear { axis }`. A normal widget is structurally
childless. `Container<Action>` is the one canonical typed authored builder for
every child-layout widget, and `container(widget, children)` is its downstream
convenience function. It stores the erased child-layout widget, arbitrary
children, common authored fields, and container-only gap exactly once, then
produces the element atomically. There is no public post-erasure child setter or
generic leaf gap API.

Built-in `row` and `column` are convenience constructors over private
`LinearContainerWidget` values passed through the same `Container::new` path.
The empty marker and `Element::with_children` design were removed because they
allowed child ownership without a defined arrangement policy.

The public `Widget<Action>` trait is typed and may use an associated `State`.
An internal object-safe adapter erases each implementation. This permits an
ergonomic typed implementation contract while keeping the unsafe-free erased
representation private and non-forgeable. Ordinary inspection occurs through
read-only `Element`/runtime facts, not `Any` payload access or concrete runtime
downcasts.

### Type erasure alternatives

- Direct public `dyn Widget<Action>` is rejected because an associated typed
  state/lifecycle API is not object-safe and would expose erasure mechanics.
- A generic view tree erased only inside runtime was considered, but would make
  conditional heterogeneous composition and the current stored app root much
  more complex without an M2 performance proof.
- `Arc`-backed immutable descriptions were considered, but cloning is not an M2
  requirement and shared ownership would obscure action/state ownership.
- A hand-written unsafe vtable is rejected by the safe-Rust contract.
- Enum plus external escape hatch and a global registry are rejected because
  built-ins would keep a privileged dispatch path.

Owned boxes make ownership and borrowing explicit, keep compile-time expansion
bounded at the erasure boundary, and impose one allocation per transient node.
M3 may optimize representation only after profiling without changing the public
widget protocol.

### Widget type identity

`WidgetTypeId` wraps Rust `TypeId` and carries no authored identity. The erased
adapter obtains it from the concrete widget implementation type. Equality is
process-local and suitable only for safe state matching and future in-process
reconciliation; it is neither serialized nor stable across builds. A separate
debug type name is inspectable but never participates in equality.

Two instances of the same concrete type report the same identity. Different
concrete types differ. Generic instantiations follow Rust identity and therefore
differ when their concrete type parameters differ. Action mapping delegates the
wrapped widget identity instead of creating a new runtime widget kind: mapping
changes action plumbing, not layout/state compatibility.

Authored `ElementId`, authored `ElementKey`, transient `RuntimeNodeId`, widget
type identity, and future mounted identity remain separate concepts.

### Typed component action mapping

`Element<ChildAction>::map_action` consumes the subtree and a typed
`Fn(ChildAction) -> ParentAction`. It stores the mapping in an erased wrapper at
each widget action boundary and recursively maps every child. Mapping is
deferred until the widget produces an action; no string messages or application
action `Any` downcasts are used.

Nested mapping composes through ordinary wrapper calls. The mapper is shared
within the mapped subtree, is single-threaded, and therefore needs neither
`Send` nor `Sync`. Mapping preserves widget type identity, authored ID/key,
style/layout intent, children, diagnostics, and all non-action capabilities.
The mapping operation alone requires an owned `'static` closure because the
transient tree stores it. No global `Action: Clone`, `Send`, `Sync`, or `'static`
bound is added. Explicit mutable activation extracts an action from a transient
source; a successful dispatch immediately rebuilds the view. This removes the M1
`Action: Clone` activation limitation without treating the slot as persistent
runtime-local state.

### Activation ownership

`Widget::activate(&mut self)` and `Element::activate(&mut self)` make destructive
action extraction visible in the type system. Borrowed `activation()` and
`semantics()` inspection cannot consume an action. The runtime first inspects
the indexed node immutably, then uses the narrow mutable preorder extraction
path; it dispatches and rebuilds immediately after `Some(action)`.

The doc-hidden `extract_action_at_preorder_for_runtime` method is a temporary
cross-crate bridge: core cannot depend on runtime, its raw index is valid only
for the current transient tree, extraction may consume a one-shot action, and
the value is neither authored nor mounted identity. It has no downstream
compatibility guarantee. M3 replaces it with generational mounted targeting.

A transient action source is one-shot: its first enabled extraction may return
the owned non-`Clone` action and a second extraction returns `None`. Disabled or
otherwise rejected activation does not call the mutable hook and consumes
nothing. Actionable/focusable/semantic facts describe configured behavior and
remain stable after extraction; an exhausted direct tree therefore remains
actionable but produces no action. A successful runtime activation authors a
fresh source during the immediate rebuild, so the rebuilt control can activate
again. Mapped wrappers forward mutable activation and map only a produced
action. If application update panics, normal runtime consistency guarantees do
not apply.

Four ownership models were compared. Owned node consumption was rejected
because it makes indexed lookup and retaining unaffected siblings impractical.
Localized `Action: Clone` was rejected because it restores the M1 limitation.
A separate action-source abstraction was rejected as premature duplication of
the widget/event boundary. Hidden interior mutation behind `&self` was rejected
because it contradicts borrowed inspection. Explicit mutable activation is the
smallest safe model and maps directly to a future mounted mutable participant.

### Runtime-local state and lifecycle seam

Each `Widget` declares an associated `State: 'static` and creates it explicitly;
stateless widgets must write `type State = ();` and `fn create_state(&self) {}`.
The other capability methods have defaults, but Rust has no stable associated-
type default for this contract, so statelessness is straightforward rather than
automatic. `WidgetStateTypeId`, separate from widget type
identity, wraps the state `TypeId`. The private erased adapter owns safe `Any`-
based state access and returns a deterministic mismatch error before invoking a
typed hook. No unchecked cast or panic is permitted.

Compatibility is checked in a fixed order: concrete widget implementation
identity first, declared state identity second, and the private erased payload
downcast last. `WidgetStateMismatch` exposes distinct `WidgetType`, `StateType`,
and `ErasedStatePayload` cases with truthful expected/actual accessors. This is
important when two incompatible widgets both declare `State = ()`; equal state
IDs cannot conceal the widget-type mismatch. Action mapping preserves the
underlying widget and state identities, so state created immediately before or
after an otherwise equivalent mapping remains compatible.

The public bounded `Element::create_widget_state` and
`Element::run_lifecycle` seam creates opaque initial state and can run `Mount`,
`Update`, then `Unmount` proof hooks while recording deterministic lifecycle
diagnostics/requests. Passing state created for another widget reports a
deterministic mismatch. This seam proves the contract; `AppRuntime` does not
retain this state or execute production lifecycle across rebuilds in M2.

M3 will own storage, keyed/type/position matching, generational mounted IDs,
hook scheduling, state retention/drop, focus retention, and invalidation. The
M2 type IDs and checked access are inputs to that design, not a partial mounted
arena.

The M2 proof capability methods remain state-independent. Only the isolated
lifecycle hook receives `&mut State`; activation, measurement, paint, semantics,
and diagnostics cannot observe that value. Therefore M2 proves state identity,
initialization, typed lifecycle compatibility, and hook mutation only. It does
not claim that `Widget<Action>` is the complete mounted participant interface or
that retained state can already influence mounted behavior.

Four seams were compared. Passing `&State`/`&mut State` to every method would
pretend the transient runtime already owns a mounted borrow schedule. Splitting
mutable lifecycle/event access from immutable layout/paint/semantic access is a
credible mounted design, but still depends on M3 storage and phase ordering.
Separating transient description from mounted behavior into two traits may
ultimately make ownership clearest, but requires the reconciliation ADR. M2
therefore retains the narrow proof signatures. M3 will deliberately introduce a
breaking state-aware mounted behavior API—using contexts, split access, or a
trait separation selected with the storage design—before claiming retained
state-dependent widget behavior. M2 does not fake that proof with temporary
state storage.

### Focused proof capabilities

M2 uses small renderer- and host-neutral proof values rather than committing the
future M4–M8 production APIs:

- activation reports borrowed enabled/actionable facts; mutable `activate`
  extracts an owned action;
- intrinsic measurement describes fixed, text, or explicitly unsupported proof
  behavior. Child layout is a separate optional `ChildLayout` capability. For a
  child-layout widget, M2 combines the intrinsic minimum and measured child
  content component-wise by maximum, then applies content constraints, padding,
  and outer constraints. This is a proof rule, not the final M7 custom-layout
  policy;
- surface resolution snapshots both `measure()` and, when present,
  `child_layout()` exactly once per node/publication. Measurement and arrangement
  reuse those owned snapshots. Unsupported intrinsic measurement emits
  `runenui.measurement.unsupported` but still lays out every child. Unknown
  intrinsic measurement emits `runenui.measurement.unrecognized`. A future
  unknown child-layout variant emits `runenui.child-layout.unrecognized`, uses a
  vertical linear fallback, and still publishes every descendant. Generic
  control text uses `ControlLabel` end to end;
- paint proof contributes a deterministic category and description, not a paint
  primitive scene;
- semantic proof contributes deterministic role/name/enabled/actionable/action
  intent facts, not a stable accessibility tree;
- diagnostics contribute deterministic code/message facts through normal
  traversal and surface inspection.

After the mandatory state declaration and constructor, defaults make a widget
non-interactive, zero-sized, semantically generic, and diagnostically empty; an
external widget implements only the capabilities it owns. The trait remains
intentionally small. M3 first introduces state-aware mounted behavior; M4 event routing,
M5 semantics/testing, M6 paint/hit scenes, M7 production layout/style, and M8
text will replace or expand their proof-level values without treating them as
complete production protocols.

## Rejected alternatives

- **Keep `ElementKind` plus `Custom`:** rejected; preserves central dispatch and
  a privileged built-in path.
- **Global widget registry:** rejected; introduces process-global ordering,
  registration, and linking failure modes with no need.
- **One giant future-complete widget trait:** rejected; would prematurely bind
  event, renderer, semantics, text, and layout designs.
- **Pure immediate-mode identity:** rejected; conflicts with M3 persistence and
  lifecycle requirements.
- **Components as mounted widgets:** rejected; composition alone must not create
  identity or state.
- **Signals/observables as primary state:** rejected; application state, typed
  actions, and explicit update remain authoritative.
- **ECS entities as mandatory identity:** rejected; RunenUI must remain usable
  without ECS or Runenwerk.
- **Renderer-owned widgets:** rejected; renderers consume resolved proof/scene
  facts and never own UI semantics or behavior.

### Activation output correction

Mutable `Widget::activate` returns `WidgetActivationOutput<Action>` rather than
`Option<Action>`. The optional action and persistent `state_changed` fact are
independent and survive recursive action mapping. This is a breaking pre-1.0
correction: state-only mutation must be explicit, and an empty output is the
authoritative no-mutation/no-action result.

## Consequences

Built-in text, button, and container behavior must migrate to public widget
adapters behind separate authored builders and the old closed element
variants/views must be removed.
Runtime traversal and all current proof products dispatch through erased widget
capabilities. Downstream crates can implement widgets with no framework edits.

The breaking migration replaces `IntoElement` with `View`, replaces built-in
kind inspection with common widget/capability inspection, and evolves
`SurfaceNodeKind` into open proof facts. This is appropriate under the pre-1.0
policy and avoids a compatibility shell.

M2 completes only when one non-publishable downstream package uses public APIs
to prove custom action mapping, type/state identity, lifecycle conformance,
event activation, layout, paint, semantics, diagnostics, and inspection on
stable and Rust 1.93.0. M3 remains unimplemented.

Every valid authored node is represented in `RuntimeTreeIndex`, `SurfaceFrame`,
`SurfaceStyleReport`, and `SurfaceLayoutReport` with identical node count,
preorder ID sequence, and parent relationships. Internal traversal no longer
filters unmeasured or unarranged valid nodes. This prevents indexed, focusable,
or actionable ghost descendants and keeps hit testing aligned with activation.
