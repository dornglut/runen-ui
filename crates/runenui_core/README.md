# `runenui_core`

> **Category: Current contract**

`runenui_core` is the host- and renderer-neutral authored-data crate in the
current RunenUI headless proof.

It owns validated logical lengths, authored IDs/keys, typed token references and
non-overwriting token definitions, style intent/resolution, and the open
transient `View`/`Element`/`Widget` architecture. `text`, `button`, `row`, and
`column` return typed views/widgets using the same protocol as downstream
implementations. `View` erases one value, iterator/collection `Views` scales
homogeneous children, and `children!` collects any number of heterogeneous
children.

Dynamic identifier and float constructors are fallible. Identifier semantics are
Unicode text-based regardless of allocation-free static or owned storage; literal
macros enforce the same grammar at compile time. Builder identifier convenience
records invalid authoring diagnostics rather than storing invalid identity. Token
families are non-exhaustive for future style evolution. The runtime diagnoses
duplicate tree IDs and sibling keys. The old
`Px`/`Length` split, unused length-token family, generic no-op element setters,
argument-builder duplicates, and tuple arity implementations are gone.

`ElementKind` and the old built-in element views are removed. `WidgetTypeId`
wraps process-local Rust type identity, private safe erasure preserves bounded
capabilities, `map_action` recursively maps typed component actions, and opaque
state/lifecycle access is checked. Public built-in authored views convert into
private behavior-only widget payloads, preventing `Element::new` from silently
losing builder configuration. `ChildLayoutWidget`, `ChildLayout`, and canonical
`Container<Action>`/`container` authoring give downstream and built-in
child-layout widgets the same atomic ownership and gap path. Action extraction
is explicitly mutable and supports owned
non-`Clone` actions without interior mutation. Every implementation must declare
`State` and `create_state`; a stateless widget writes `type State = ();` and an
empty constructor explicitly. M2 capabilities remain state-independent except
for the lifecycle proof, so M3 must introduce the state-aware mounted behavior
contract. Keys and widget state do not preserve mounted identity until M3.
The crate does not own runtime state, input routing, effects, layout execution,
semantics, paint scenes, renderer backends, native hosts, application state, ECS,
or legacy dependencies.

See the [public API contract](../../docs/architecture/public-api.md), workspace
[status](../../docs/status-map.md), [support matrix](../../docs/feature-support-matrix.md),
and [roadmap](../../docs/roadmap.md).
