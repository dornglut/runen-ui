# RunenUI

> **Category: Current contract**

RunenUI is a pre-1.0 Rust-native UI framework project. Its production goal is a stable, extensible UI kernel for deterministic headless testing, standalone desktop applications on Windows, macOS, and Linux, and embedding in engine- or editor-owned hosts.

Today RunenUI is a coherent **headless architecture proof**. It is not a production UI framework, native desktop toolkit, renderer backend, or finished control library. Current APIs are experimental and may change incompatibly while the foundations are corrected.

## What exists today

The active workspace proves:

- application-owned state and typed actions with explicit `update`;
- typed transient views erased into open `Element<Action>` trees, ordinary
  component functions, recursive typed action mapping, and builder/`element!`
  authoring;
- downstream `Widget<Action>` implementations with process-local type identity,
  safe erasure, checked state/lifecycle conformance, public child-bearing
  construction through `ChildLayoutWidget` and `Container<Action>`, explicit
  mutable action extraction, and the same erased protocol used by private
  built-in widget implementations;
- deterministic headless dispatch, basic focus, press activation, and tracing;
- typed style values, tokens, computed style, provenance, and diagnostics;
- explicit layout constraints, a renderer-neutral measurement-provider seam,
  and separate one-query intrinsic/child-layout snapshots per publication;
- constrained row/column measurement and arrangement with aligned frame, style, and layout diagnostics;
- preorder/parent-aligned index, frame, style, and layout products with no
  hidden actionable descendants;
- a Counter application exercising the current public crates.

Important limitations remain: runtime identity is rebuilt from preorder position,
keys are not reconciled, focus is cleared after dispatch, input behavior is
proof-level, text measurement is deterministic character counting, and mounted
state/lifecycle execution, effects, production semantics/accessibility, paint
scenes, production text, native hosts, renderer backends, and production controls
are absent. M2 paint and semantic facts are deterministic extension proofs, not
the M5/M6 production products.

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
    -> transient owned View/Element tree
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
- [Contributing](CONTRIBUTING.md)
- [Security policy](SECURITY.md)
- [API stability](docs/api-stability.md)
- [Release policy](docs/release-policy.md)

When sources disagree, the current status, support matrix, roadmap, architecture, and accepted ADRs take precedence over older incremental design documents. Historical material is not implementation authority.

## Current API proof

The Builder API is the semantic foundation; `element!` is optional sugar over the same open view protocol:

```rust
use runenui_core::{Element, View, button, children, column, text};

#[derive(Clone, Copy)]
enum CounterAction {
    Increment,
}

fn counter_screen(value: i32) -> Element<CounterAction> {
    column(children![
        text(format!("Count: {value}")),
        button("+").on_press(CounterAction::Increment),
    ])
    .gap(8_u16)
    .into_element()
}
```

Typed builders reject incompatible configuration at compile time, `children!`
has no fixed arity ceiling, and dynamic numeric/identifier constructors validate
their inputs. Components can author a local action and map their subtree into a
parent action without knowing that parent type:

```rust
use runenui_core::{Element, View, button};

enum ChildAction { Save }
enum ParentAction { Child(ChildAction) }

fn child() -> Element<ChildAction> {
    button("Save").on_press(ChildAction::Save).into_element()
}

fn parent() -> Element<ParentAction> {
    child().map_action(ParentAction::Child)
}
```

This does not imply mounted reconciliation, persistent widget state, correct
release-based activation, production controls, accessibility, paint scenes, or
native rendering.

M2 widget state is intentionally narrow: every widget explicitly declares and
creates its state (`type State = (); fn create_state(&self) {}` for a stateless
widget), but only the isolated lifecycle proof receives it. State-aware mounted
capabilities remain a deliberate breaking M3 design, not an implemented claim.

## Validation

The repository baseline is:

```powershell
cargo +stable fmt --all
cargo validate
```

Format intentional changes with latest stable rustfmt, matching CI. `cargo validate` is the locked, read-only shared local/CI implementation. It runs stable formatting checks, locked tests, Clippy with denied warnings, Rust 1.93.0 MSRV tests, repository metadata checks, and repository-relative Markdown link validation from the resolved workspace root. Also run `git diff --check` and slice-specific checks. See [Validation](docs/tooling/validation.md).

Generated context exports are written to the ignored `context/` directory:

```powershell
py tools/context/export_repo_context.py
```

Normal profiles exclude historical legacy material. See [Context Export](tools/context/README.md).

## Release status

RunenUI has not reached a stable public API or production release. All workspace packages are `0.1.0` and publication is disabled until release infrastructure and milestone gates exist. `1.0.0` is reserved for completion of the required production profiles and the M11 hardening gate.

RunenUI is dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
