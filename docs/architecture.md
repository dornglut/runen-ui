# Architecture

RunenUI separates application state, element authoring, runtime evaluation, layout, styling, accessibility, and renderer output.

The runtime is the central owner of UI evaluation. Hosts feed input into the runtime. Renderers consume published surface frames. Applications own their state and actions.

## Overview

RunenUI is organized around a small set of concepts:

- `State` is application-owned data.
- `Action` is application-owned intent emitted by elements.
- `update` changes state in response to an action.
- `Element` is the UI description derived from state.
- `Runtime` evaluates input, dispatches actions, computes layout, and publishes frames.
- `SurfaceFrame` is renderer-neutral output for a named surface.

## Core Loop

```text
InputEvent
  -> Runtime
  -> Action
  -> update(State, Action)
  -> root(State) -> Element<Action>
  -> Style resolution
  -> LayoutBox tree
  -> Accessibility tree
  -> Primitive output
  -> SurfaceFrame
```

The application provides state, an `update` function, and a root element function. The runtime owns the rest of the UI pipeline.

## Runtime

The runtime receives input events from a host, resolves focus and hit testing, dispatches element actions, calls the application `update` function, rebuilds the root element, computes layout, and publishes a surface frame.

The runtime is also the natural owner for tracing, inspection, accessibility tree production, and deterministic replay.

## Elements

Elements are UI descriptions. They are derived from application state and can emit application actions.

Elements describe structure, control intent, layout properties, style intent, accessibility data, and event bindings. They are not renderer objects.

The primary authoring surface is `element!`:

```rust
element! {
    column gap=8 {
        text "Counter"
        button "+" on_press=CounterAction::Increment
    }
}
```

The same model can be expressed through builder calls:

```rust
column((
    text("Counter"),
    button("+").on_press(CounterAction::Increment),
))
.gap(8)
```

## State and Actions

Applications own state and actions.

```rust
struct Counter {
    count: i32,
}

enum CounterAction {
    Increment,
    Decrement,
    Reset,
}
```

Actions are emitted by elements and passed to `update`:

```rust
fn update(counter: &mut Counter, action: CounterAction) {
    match action {
        CounterAction::Increment => counter.count += 1,
        CounterAction::Decrement => counter.count -= 1,
        CounterAction::Reset => counter.count = 0,
    }
}
```

## Styling

Styling is a distinct data pipeline between element authoring and layout/render output.

The target styling architecture is documented in [Styling Target Architecture](architecture/styling-target.md).

Token references and token-backed style values are documented in [Token Reference Target](architecture/token-reference-target.md).

Current token authoring ergonomics are documented in [Token Authoring Ergonomics](architecture/token-authoring-ergonomics.md). The current decision is to use typed Rust expressions through the builder API and `element!` expression attributes before adding shorthand syntax.

The resolved style data model is documented in [Computed Style Model](architecture/computed-style-model.md). The runtime cutover that makes one style-resolution product feed layout, surface output, and diagnostics is documented in [Computed Style Runtime Integration](architecture/computed-style-runtime-integration.md).

## Layout

Layout is a distinct runtime phase. Elements express layout intent through properties such as direction, gap, padding, sizing, alignment, and constraints. The runtime computes a `LayoutBox` tree from those inputs.

The `LayoutBox` tree is inspectable data. It gives hosts, renderers, tests, and tools a stable representation of computed geometry.

## Accessibility

Accessibility is structured UI data derived from elements, identity, roles, labels, state, actions, style, and layout.

The accessibility tree is part of the runtime model. This lets hosts expose semantic UI information even when the final renderer is custom, game-oriented, or non-DOM.

## Surface Frames

A `Surface` is a named UI output target.

A `SurfaceFrame` is the runtime's published renderer-facing output for a surface. The target model contains computed style, computed layout, accessibility data, action bindings, and renderer-neutral primitives. The current implementation still publishes bounds and node kinds separately from style diagnostics; the computed-style runtime integration defines the next cutover that places concrete `ComputedStyle` on each surface node without placing token or provenance data in the frame.

## Hosts and Renderers

A host embeds RunenUI into an application, engine, editor, or tool. It feeds input into the runtime and receives published surface frames.

A renderer consumes primitive output and draws it. Renderer backends can target different graphics systems while sharing the same element, state, action, style, layout, and accessibility model.
