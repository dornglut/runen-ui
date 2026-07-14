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
wraps process-local Rust type identity, private safe erasure preserves
state-aware capabilities, and `map_action` recursively maps typed component
actions without changing widget or state identity. Public built-in authored views
convert into private behavior-only widget payloads. `ChildLayoutWidget`,
`ChildLayout`, and canonical `Container<Action>`/`container` authoring give
downstream and built-in child-layout widgets the same atomic ownership and gap
path. Every implementation declares `State` and `create_state`;
mount/update/unmount and activation receive runtime-owned contexts, all
capabilities observe persistent state, and stateless widgets use `State = ()`.
Built-in buttons use repeatable `on_activate` action factories, so each accepted
proof activation can produce a fresh action without requiring `Action: Clone`,
`Copy`, or `Debug`.
`WidgetInvalidation` is a manual selective bitset. `StyleTokens` carries a
monotonic diagnostic revision, while sound runtime cache compatibility compares
the complete token definitions and values. The `__runtime` bridge
is technically public only because core and runtime are separate crates; it is
outside the prelude, doc-hidden, unstable, unsupported for applications, and
semver-exempt before 1.0. It safely consumes transient elements into checked,
non-forgeable mounted widget/state plumbing without exposing payload or arena
construction.
The crate does not own mounted storage, input routing, effects, layout execution,
semantics, paint scenes, renderer backends, native hosts, application state, ECS,
or legacy dependencies.

See the [public API contract](../../docs/architecture/public-api.md), workspace
[status](../../docs/status-map.md), [support matrix](../../docs/feature-support-matrix.md),
and [roadmap](../../docs/roadmap.md).
