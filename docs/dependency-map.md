# Dependency Map

This document defines the allowed dependency direction for the clean RunenUI workspace.

The dependency graph must stay simple and acyclic. Lower-level crates define stable data models. Higher-level crates consume those models and add behavior.

## Current Workspace Direction

```text
runenui_core
  <- runenui_runtime
  <- examples/counter
```

`examples/counter` may depend on both `runenui_core` and `runenui_runtime`.

## Crate Rules

| Crate | May depend on | Must not depend on |
|---|---|---|
| `runenui_core` | no RunenUI crates | runtime, renderer, compiler, host, ECS, legacy crates |
| `runenui_runtime` | `runenui_core` | renderer backends, app hosts, ECS, compiler/program/artifact pipeline, legacy crates |
| `examples/counter` | public RunenUI crates | legacy crates, private internals |

## Long-Term Direction

Planned crates should follow this direction:

```text
runenui_core
  <- runenui_layout
  <- runenui_render
  <- runenui_runtime
  <- runenui_testing
  <- examples
```

A future document/compiler layer may depend on the core model:

```text
runenui_core
  <- runenui_document
  <- runenui_compiler
```

The core must not depend on the compiler.

## Forbidden Direction

These dependency directions are not allowed:

```text
runenui_core -> runenui_runtime
runenui_core -> renderer backend
runenui_core -> compiler
runenui_core -> legacy/*
runenui_runtime -> renderer backend
runenui_runtime -> ECS host
runenui_runtime -> legacy/*
```

## Legacy Rule

The `legacy/` directory is reference material only.

No crate under `legacy/` may be added as a workspace member or dependency without an explicit cutover design. Salvaged concepts must be reimplemented under clean RunenUI crate boundaries.
