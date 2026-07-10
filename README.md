# RunenUI

RunenUI is a Rust-native UI framework project focused on explicit, inspectable, host-neutral user interface architecture.

Applications own state and actions. Elements emit actions. The runtime calls `update`, computes layout, and publishes renderer-neutral primitive output for a surface.

## Vision

RunenUI is designed around a simple separation:

- application state belongs to the application
- UI structure is described as `Element` trees
- user interaction emits application `Action`s
- layout produces inspectable geometry
- rendering consumes neutral `Primitive` output
- hosts and renderers integrate through explicit boundaries

The goal is to support application UI, game UI, tools, editors, live preview, and engine integration without making any single host or renderer the source of truth.

## Design Direction

RunenUI prioritizes explicit Rust APIs, readable element authoring, deterministic state flow, accessibility-aware UI data, and renderer-neutral output.

The intended public authoring surface is `element!`. The builder API is the semantic foundation underneath it, so the core model remains explicit, testable, and usable without macros.

## Target API Preview

```rust
fn counter_screen(counter: &Counter) -> Element<CounterAction> {
    element! {
        column gap=8 {
            text "Counter"
            text { counter.count.to_string() } id="counter.value"

            row gap=8 {
                button "-" on_press=CounterAction::Decrement
                button "+" on_press=CounterAction::Increment
                button "Reset" on_press=CounterAction::Reset
            }
        }
    }
}
```

## Builder Foundation

The target syntax expands to ordinary builder calls:

```rust
fn counter_screen(counter: &Counter) -> Element<CounterAction> {
    column((
        text("Counter"),
        text(counter.count.to_string()).id("counter.value"),
        row((
            button("-").on_press(CounterAction::Decrement),
            button("+").on_press(CounterAction::Increment),
            button("Reset").on_press(CounterAction::Reset),
        ))
        .gap(8),
    ))
    .gap(8)
}
```

## Documentation

- [Architecture](docs/architecture.md)
- [Target API](docs/target-api.md)
- [Influences](docs/influences.md)
- [Vocabulary](docs/vocabulary.md)
- [Styling Target Architecture](docs/architecture/styling-target.md)
- [Token Reference Target](docs/architecture/token-reference-target.md)
- [Token Authoring Ergonomics](docs/architecture/token-authoring-ergonomics.md)
- [Computed Style Model](docs/architecture/computed-style-model.md)

## Project Maps

- [Crate Map](docs/crate-map.md)
- [Dependency Map](docs/dependency-map.md)
- [Status Map](docs/status-map.md)
- [Cutover Plan](docs/cutover-plan.md)
- [Legacy Audit](docs/legacy-audit.md)

## Validation

Format the workspace with:

```powershell
cargo format
```

Run the repository baseline with one command:

```powershell
cargo validate
```

This runs:

```powershell
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets --locked -- -D warnings
```

Validation is read-only. It does not apply formatting changes.

## Context Export

Generated context exports are written to `context/`.

```powershell
py tools/context/export_repo_context.py
```

Default output:

```text
context/RunenUI-ai-core-context.txt
```

For broader current-work context:

```powershell
py tools/context/export_repo_context.py --profile current-work
```

## Status

RunenUI is in early design and implementation formation. The examples in these docs describe the intended public shape while the crate layout and APIs are established.
