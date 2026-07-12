# `runenui_core`

> **Category: Current contract**

`runenui_core` is the host- and renderer-neutral authored-data crate in the
current RunenUI headless proof.

It owns validated logical lengths, authored IDs/keys, typed token references and
non-overwriting token definitions, style intent/resolution, and the current
closed text/button/container descriptions. `text`, `button`, `row`, and `column`
return typed builders; only kind-valid configuration exists. `IntoElement` erases
one builder, iterator/collection `IntoElements` scales homogeneous children, and
`children!` collects any number of heterogeneous static children.

Dynamic identifier and float constructors are fallible. Identifier semantics are
Unicode text-based regardless of allocation-free static or owned storage; literal
macros enforce the same grammar at compile time. Builder identifier convenience
records invalid authoring diagnostics rather than storing invalid identity. Token
families are non-exhaustive for future style evolution. The runtime diagnoses
duplicate tree IDs and sibling keys. The old
`Px`/`Length` split, unused length-token family, generic no-op element setters,
argument-builder duplicates, and tuple arity implementations are gone.

`ElementKind` remains a deliberately closed M1 proof type. External widget and
component protocols are M2 work; keys do not preserve mounted identity until M3.
The crate does not own runtime state, input routing, effects, layout execution,
semantics, paint scenes, renderer backends, native hosts, application state, ECS,
or legacy dependencies.

See the [M1 public API contract](../../docs/architecture/public-api.md), workspace
[status](../../docs/status-map.md), [support matrix](../../docs/feature-support-matrix.md),
and [roadmap](../../docs/roadmap.md).
