# Crate Map

This document defines the intended ownership boundaries for the clean `RunenUI` workspace.

The map is intentionally small at the start. New crates should only be added when an existing crate boundary would otherwise become unclear.

## Current Crates

| Crate | Status | Owns | Must not own |
|---|---|---|---|
| `runenui_core` | skeleton | Host-neutral typed UI description model | runtime behavior, rendering, compiler pipeline, host integration, legacy dependencies |
| `runenui_runtime` | skeleton | Headless runtime boundary | app state definitions, renderer backends, ECS host ownership, legacy dependencies |
| `examples/counter` | skeleton | First public architecture proof | framework internals, renderer-specific behavior, legacy dependencies |

## Planned Crates

| Crate | Purpose | Add when |
|---|---|---|
| `runenui_layout` | Layout algorithms and computed layout boxes | layout behavior grows beyond simple runtime orchestration |
| `runenui_render` | Renderer-neutral primitive and surface-frame output types | primitive extraction needs its own stable public model |
| `runenui_testing` | Headless test harness and story/conformance helpers | the counter proof needs reusable assertions |
| `runenui_text` | Text measurement, shaping seams, and text-editing primitives | text behavior exceeds simple labels |
| `runenui_theme` | Theme tokens and styling resolution | styling needs a stable data model |
| `runenui_accessibility` | Accessibility tree adapters and validation helpers | accessibility output becomes substantial enough to isolate |
| `runenui_document` | Optional serialized document model | RON, visual editor, or external document support becomes active work |
| `runenui_compiler` | Optional document/compiler pipeline | document validation, migrations, source maps, or artifact caching become active work |
| `runenui_macros` | Optional authoring sugar | the builder API is stable enough to support macro expansion |

## Boundary Rule

The public foundation is the typed Rust model:

```text
Element<Action>
  -> Runtime
  -> SurfaceFrame
```

A future document/compiler layer may produce or validate this model, but the core framework must not require the compiler path for normal Rust-authored UI.
