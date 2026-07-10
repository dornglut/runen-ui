# Workspace Structure

This document records the staged workspace architecture for RunenUI.

RunenUI remains one workspace repository. Crates are split only when the split enforces a real ownership boundary, dependency boundary, optional feature boundary, or independent testing boundary. This is a target map, not permission to create empty placeholder crates.

## Current workspace

The current workspace is intentionally small:

```text
RunenUI/
├── crates/
│   ├── runenui_core/
│   └── runenui_runtime/
└── examples/
    └── counter/
```

Current responsibilities:

| Package | Owns | Must not own |
|---|---|---|
| `runenui_core` | Host-neutral UI vocabulary: elements, identities, layout units, element authoring macro, typed descriptors | runtime behavior, host integration, renderer backends, app state, legacy imports |
| `runenui_runtime` | Headless UI execution: app runtime, action dispatch, input handling, focus, runtime tree index, surface frames, trace/debug seams | app domain state definitions, native windows, concrete renderers, ECS host ownership, legacy imports |
| `examples/counter` | Product-owned proof app: state, actions, `update`, screen switching, public API usage | framework internals, renderer-specific behavior, hidden runtime plumbing |

## Long-term crate map

The long-term workspace may grow toward this set of crates, but only as implementation pressure justifies each boundary:

```text
crates/
├── runenui/                  # facade crate for normal applications
├── runenui_core/             # stable neutral vocabulary
├── runenui_runtime/          # UI execution and interaction runtime
├── runenui_layout/           # layout algorithms and measurement contracts
├── runenui_style/            # style properties, tokens, themes, computed style
├── runenui_render/           # renderer-neutral frame/primitive protocol
├── runenui_controls/         # standard controls
├── runenui_docking/          # docking/workspace subsystem
├── runenui_host/             # host contract: windows, input, clipboard, IME, accessibility bridge
├── runenui_platform_winit/   # concrete desktop host adapter
├── runenui_backend_wgpu/     # conventional standalone renderer backend
├── runenui_backend_sdf/      # SDF-specialized backend
├── runenui_frontend/         # ergonomic Rust authoring layer
├── runenui_source/           # optional external source formats and diagnostics
├── runenui_devtools/         # inspector, trace, diagnostics, replay, hot reload
└── runenui_testing/          # deterministic test harnesses and assertions
```

Do not create these crates until they have real code to own.

## Crate extraction criteria

A new crate is allowed only when it has at least one of these properties:

- independent public API value
- meaningful optionality behind Cargo features
- dependency pressure that Cargo should enforce
- substantial implementation size beyond module hygiene
- independent test harness or conformance value
- a boundary another repository or backend must consume

A new crate is not justified only because a concept has a name.

Keep these as modules until the criteria above are met:

```text
focus
events
buttons
geometry
animation
routes
clipboard
```

## Dependency direction

The intended direction is acyclic and inward-facing:

```text
                       runenui
                          │
          ┌───────────────┼────────────────┐
          ▼               ▼                ▼
 runenui_frontend   runenui_controls  runenui_docking
          │               │                │
          ├───────────────┴────────────────┤
          ▼                                ▼
 runenui_runtime                    runenui_style
          │                                │
          ├──────────────┬─────────────────┤
          ▼              ▼                 ▼
 runenui_layout    runenui_render     runenui_core
          │              │                 ▲
          └──────────────┴─────────────────┘

 runenui_host ───────────────► core/runtime contracts
 runenui_platform_winit ─────► runenui_host
 runenui_backend_wgpu ───────► runenui_render
 runenui_backend_sdf ────────► runenui_render
 runenui_devtools ───────────► public runtime/render observations
 runenui_testing ────────────► host/runtime/render public contracts
```

Critical rules:

```text
core          depends on nothing framework-specific
runtime       never depends on controls, docking, winit, or concrete renderer backends
render        never depends on a concrete renderer
host          never depends on winit
controls      never depend on a platform implementation
docking       never creates native windows directly
backend_*     never owns UI behavior
platform_*    never owns widget behavior
```

## Crate responsibilities

### `runenui`

Facade crate for normal applications.

It re-exports the stable authoring surface:

```rust
use runenui::prelude::*;
```

It should contain almost no implementation. It should be added late, after lower-level APIs have stabilized enough that a facade will not hide churn.

### `runenui_core`

Smallest stable vocabulary:

- IDs and keys
- geometry and logical units
- element descriptors
- neutral semantic values
- renderer- and host-neutral types
- the macro surface while it remains a direct layer over core descriptors

It must not depend on runtime, windowing, GPU APIs, controls, docking, app state, or legacy crates.

### `runenui_runtime`

UI execution layer:

- retained runtime tree
- node indexing
- action dispatch
- input event handling
- focus
- pointer targeting
- invalidation and scheduling, once introduced
- surface-frame publication
- runtime traces and public observation seams

It does not create native windows and does not draw pixels.

### `runenui_layout`

Owns generic layout algorithms:

- constraints
- measurement
- row/column/flex
- grid
- stack
- absolute positioning
- intrinsic sizing
- layout results

Extract only when the current layout module becomes more than simple runtime orchestration.

### `runenui_render`

Renderer-neutral frame protocol:

- drawing primitives
- clips
- transforms
- text runs
- image references
- z-order
- render resource handles
- frame metadata

Backends consume this crate. The crate must not know WGPU, SDF shaders, Runenwerk renderer internals, or platform windows.

### `runenui_style`

Visual policy:

- style properties
- tokens
- themes
- selectors, if needed
- cascade/inheritance, if needed
- computed styles

Start only after explicit style tokens and computed styles are needed by more than one control or render path.

### `runenui_controls`

Standard host-independent controls:

- button
- label
- text input
- checkbox/radio
- slider
- scroll/list/menu
- overlays

Controls combine core, runtime, layout, style, semantics, and render concepts. They must not depend on platform implementations.

### `runenui_docking`

Dedicated docking/workspace subsystem:

- dock tree
- split nodes
- tab groups
- panel movement
- drag/drop state
- drop-target computation
- workspace serialization
- floating-panel semantics

It must not create native windows directly. Native detachment goes through host contracts.

### `runenui_host`

Contract crate for hosting environments:

- windows and surfaces
- normalized input
- clipboard
- IME
- cursor changes
- drag and drop
- dialogs
- accessibility bridge
- redraw/wakeup requests

This is not a desktop implementation. Runenwerk implements these contracts from its engine runtime. Standalone desktop apps may use `runenui_platform_winit` later.

### `runenui_platform_winit`

Concrete desktop integration:

- event loop
- native window lifecycle
- mouse, keyboard, touch, and pen translation
- DPI changes
- IME events
- clipboard integration
- native cursor
- file drops

It depends inward on `runenui_host`. The host contract never depends on Winit.

### Renderer backends

`runenui_backend_wgpu` is the conventional standalone renderer backend.

`runenui_backend_sdf` is the specialized SDF backend.

Runenwerk-specific integration stays in Runenwerk, for example:

```text
Runenwerk/
└── crates/
    └── runenwerk_runenui/
```

### `runenui_frontend`

Ergonomic Rust authoring layer:

- component definitions
- screens
- routing
- update/effects helpers
- builder/descriptor composition

This crate should compile authoring declarations into neutral core elements. It should not own runtime behavior.

### `runenui_source`

Optional external declarative source support:

- RON or another source format
- parsing
- schema validation
- diagnostics
- source locations
- live source loading

Use `source`, not `ast`, because the public concern is the source frontend. ASTs are implementation details.

### `runenui_devtools`

Development-time observability:

- hierarchy inspector
- layout inspector
- focus inspection
- action/frame traces
- diagnostics overlay
- recording/replay
- hot-reload coordination

It must consume public runtime/render observations rather than reaching into private runtime internals.

### `runenui_testing`

Deterministic framework and application testing:

- headless host
- headless renderer
- synthetic input
- action assertions
- semantic-tree assertions
- surface-frame assertions
- snapshots
- deterministic clocks

This should be introduced before major feature expansion so later systems do not grow ad hoc test utilities.

## Repository boundaries

```text
Crystonix/RunenUI
├── generic UI framework
├── host contracts
├── standalone platform adapters
├── renderer contracts
├── optional standalone renderers
└── framework examples

Crystonix/Runenwerk
├── game/engine runtime
├── ECS
├── engine renderer
├── engine window/input ownership
└── RunenUI adapter

Product repositories
├── domain logic
├── product state
├── persistence/networking
├── workspace definitions
└── application composition
```

No generic `RunenHost` repository should be introduced now. Keep `runenui_host` as a future contract crate inside RunenUI unless multiple real consumers prove a shared implementation-level host runtime is needed.

## Implementation order

1. Stabilize `runenui_core` and `runenui_runtime`.
2. Clean current module structure before extracting crates.
3. Split the Counter example into app/UI/main files.
4. Extract `runenui_layout` only when layout has a concrete independent API.
5. Extract `runenui_render` only when the surface-frame protocol is ready to be consumed independently.
6. Add style/theme tokens and computed style data.
7. Add accessibility/semantics extraction.
8. Add `runenui_testing` before broad feature expansion.
9. Add `runenui_controls` once controls become reusable behavior packages.
10. Define `runenui_host` when a real host adapter is needed.
11. Add one real platform adapter and one renderer backend.
12. Add docking after focus, overlays, drag interaction, persistence, and multi-surface behavior are stable.
13. Add source loading and devtools after runtime observation APIs settle.
14. Add the `runenui` facade when lower-level public APIs are stable enough to re-export.

## Near-term roadmap

The immediate sequence is:

```text
PR #38: staged workspace architecture document
PR #39: core module cleanup
PR #40: runtime module cleanup
PR #41: split Counter into app.rs / ui.rs / main.rs
```

No new crates should be created in these PRs.
