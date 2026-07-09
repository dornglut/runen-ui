# runenui_core

`runenui_core` owns the host-neutral typed UI description model for RunenUI.

This crate is the foundation for authoring UI as data. It will define the public `Element<Action>` tree, stable element identity, typed event bindings, layout intent, semantic roles, and core geometry types.

## Responsibilities

`runenui_core` may define:

* typed `Element<Action>` UI descriptions
* element identity and keying primitives
* semantic roles and accessibility-relevant element facts
* layout intent types such as gap, padding, sizing, and alignment
* typed event bindings such as press/change intent
* renderer-neutral geometry types

## Non-responsibilities

`runenui_core` must not own:

* runtime state
* input dispatch
* hit testing
* focus management
* app update execution
* layout solving
* rendering backends
* host integration
* ECS integration
* compiler/program/artifact pipelines
* legacy crate dependencies

For workspace-wide dependency rules, see [dependency-map](../../docs/dependency-map.md).

For implementation maturity, see [status-map](../../docs/status-map.md).
