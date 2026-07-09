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

The primary authoring shape is `element!`, a small Rust UI DSL for nested element trees. The macro is intended to expand into regular `Element` builder calls so the underlying API remains explicit and testable.

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

## Documentation

- [Architecture](docs/architecture.md)
- [Target API](docs/target-api.md)
- [Influences](docs/influences.md)
- [Vocabulary](docs/vocabulary.md)

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
