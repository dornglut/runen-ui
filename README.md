# RunenUI

> **Category: Current contract**

RunenUI is a pre-1.0 Rust-native UI framework project. Its production goal is a stable, extensible UI kernel for deterministic headless testing, standalone desktop applications on Windows, macOS, and Linux, and embedding in engine- or editor-owned hosts.

Today RunenUI is a coherent **headless architecture proof**. It is not a production UI framework, native desktop toolkit, renderer backend, or finished control library. Current APIs are experimental and may change incompatibly while the foundations are corrected.

## What exists today

The active workspace proves:

- application-owned state and typed actions with explicit `update`;
- immutable `Element<Action>` description trees and builder/`element!` authoring;
- deterministic headless dispatch, basic focus, press activation, and tracing;
- typed style values, tokens, computed style, provenance, and diagnostics;
- explicit layout constraints and a renderer-neutral measurement-provider seam;
- constrained row/column measurement and arrangement with aligned frame, style, and layout diagnostics;
- a Counter application exercising the current public crates.

Important limitations remain: runtime identity is rebuilt from preorder position, keys are not reconciled, focus is cleared after dispatch, input behavior is proof-level, text measurement is deterministic character counting, `SurfaceFrame` contains semantic control kinds rather than paint primitives, and mounted lifecycle, effects, semantics/accessibility, production text, native hosts, renderer backends, and production controls are absent.

## Production profiles

RunenUI targets three required profiles:

1. **Headless/test:** deterministic mounted execution, synthetic input and time, deterministic effects/tasks, semantic/layout/hit/paint inspection, and replayable traces without a native window or GPU.
2. **Standalone desktop:** Windows, macOS, and Linux with DPI and multi-window support, clipboard, cursor, IME, drag/drop, accessibility, a production event loop, and one conventional renderer backend.
3. **Embedded host:** a host-owned window and frame loop with host-provided input, resources, timing, clipboard, text, and wakeups, consuming the same renderer-neutral scene protocol without ECS, Runenwerk, or renderer assumptions in RunenUI.

Mobile, web, external UI source formats, docking, visual editing, and advanced devtools are later targets and do not block the first production release.

## Architecture direction

The accepted runtime direction is hybrid:

```text
Application state
    -> transient immutable View/Element tree
    -> keyed reconciliation
    -> persistent mounted runtime tree
    -> computed style and layout
    -> semantic tree + hit-test scene + paint scene
    -> host accessibility/event integration + renderer backend
```

The transient authored tree is not persistent runtime state. The mounted tree will retain generational identity, lifecycle, widget-local interaction state, focus/capture, dirty state, semantic identity, and task/subscription ownership. Renderers will consume paint primitives and resources, not semantic widget kinds.

This target is documented architecture, not a claim about the current implementation.

## Canonical project documents

- [Current status](docs/status-map.md)
- [Feature and support matrix](docs/feature-support-matrix.md)
- [Production roadmap](docs/roadmap.md)
- [Architecture](docs/architecture.md)
- [Documentation retention and disposition](docs/documentation-retention-plan.md)
- [Validation](docs/tooling/validation.md)

When sources disagree, the current status, support matrix, roadmap, architecture, and accepted ADRs take precedence over older incremental design documents. Historical material is not implementation authority.

## Current API proof

The Builder API is the semantic foundation; `element!` is optional sugar over the same closed proof vocabulary:

```rust
use runenui_core::{Element, button, column, text};

#[derive(Clone, Copy)]
enum CounterAction {
    Increment,
}

fn counter_screen(value: i32) -> Element<CounterAction> {
    column((
        text(format!("Count: {value}")),
        button("+").on_press(CounterAction::Increment),
    ))
    .gap(8)
}
```

This example reflects the implemented API. It does not imply component action mapping, external custom widgets, mounted reconciliation, correct release-based activation, production controls, or native rendering.

## Validation

The repository baseline is:

```powershell
cargo fmt --all --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
rustup run 1.93.0 cargo test --workspace --locked
git diff --check
cargo validate
```

`cargo validate` is the canonical local entry point. See [Validation](docs/tooling/validation.md) for its current implementation and policy.

Generated context exports are written to the ignored `context/` directory:

```powershell
py tools/context/export_repo_context.py
```

Normal profiles exclude historical legacy material. See [Context Export](tools/context/README.md).

## Release status

RunenUI has not reached a stable public API or production release. Package publication remains disabled until release infrastructure and milestone gates exist. `1.0.0` is reserved for completion of the required production profiles and the M11 hardening gate.
