# Target API

This document describes the intended public authoring shape for RunenUI.

The target API centers on `Element`, `Action`, `update`, `Runtime`, and `Surface`.

## Counter Example

```rust
use runenui::prelude::*;

#[derive(Default)]
struct Counter {
    count: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CounterAction {
    Increment,
    Decrement,
    Reset,
}

fn update(counter: &mut Counter, action: CounterAction) {
    match action {
        CounterAction::Increment => counter.count += 1,
        CounterAction::Decrement => counter.count -= 1,
        CounterAction::Reset => counter.count = 0,
    }
}

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

fn win_screen(counter: &Counter) -> Element<CounterAction> {
    element! {
        column gap=8 {
            text "You win"
            text { format!("Final count: {}", counter.count) }

            button "Reset" on_press=CounterAction::Reset
        }
    }
}

fn root(counter: &Counter) -> Element<CounterAction> {
    if counter.count >= 10 {
        win_screen(counter)
    } else {
        counter_screen(counter)
    }
}

fn main() {
    Runtime::builder()
        .surface("counter.main")
        .state(Counter::default())
        .update(update)
        .root(root)
        .run();
}
```

## Element Authoring

`element!` is the primary authoring syntax for readable nested UI.

```rust
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
```

The macro uses lowercase element names, key-value properties, Rust expressions in `{ ... }`, nested blocks for children, and event-specific bindings such as `on_press`.

## Builder Equivalent

`element!` is intended to expand into regular builder calls. The same counter screen can be represented without macro syntax:

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

The builder API is the semantic foundation. The macro is authoring sugar over that foundation.

## Runtime Setup

A minimal app connects a surface, state, an `update` function, and a root element function:

```rust
Runtime::builder()
    .surface("counter.main")
    .state(Counter::default())
    .update(update)
    .root(root)
    .run();
```

The runtime owns input dispatch, action delivery, layout, accessibility data, primitive extraction, and surface frame publishing.

## Effects Direction

The simple counter example uses an `update` function that only mutates state:

```rust
fn update(state: &mut State, action: Action) {
    // mutate state
}
```

That shape should stay available for simple applications. Larger applications also need to request work outside the immediate state transition: async tasks, host commands, navigation, file dialogs, clipboard access, timers, subscriptions, and external messages.

RunenUI should model those operations as effects. An effect is a request emitted by application logic and executed by the runtime or host integration layer after `update` returns.

## Effect Principles

Effects should keep the state transition readable while preserving runtime control:

- `update` remains the place where application state changes.
- effects describe external work; they do not execute it inline.
- async results re-enter the app as normal `Action` values.
- host-specific operations are requested through typed commands.
- effects are traceable, cancellable where appropriate, and visible to test tooling.
- the simple two-argument `update` form remains valid for apps that do not need effects.

## Advanced Update Shape

The target model can support a richer update form without changing the simple example:

```rust
fn update(
    state: &mut State,
    action: Action,
    effects: &mut Effects<Action>,
) {
    // mutate state
    // enqueue effect requests
}
```

The runtime builder can accept both forms through an adapter trait:

```rust
Runtime::builder()
    .surface("app.main")
    .state(State::default())
    .update(update)
    .root(root)
    .run();
```

A simple update function behaves as if it received an empty effect sink. An advanced update function can enqueue effect requests.

## Effect Categories

RunenUI should reserve room for several effect families:

| Effect | Purpose |
|---|---|
| `Task` | Run async work and map the result back into an `Action`. |
| `HostCommand` | Ask the embedding host to do something, such as open a file dialog or write to the clipboard. |
| `Navigation` | Request route or screen changes in host-managed navigation. |
| `Timer` | Emit an action after a delay or on an interval. |
| `Subscription` | Connect to external streams such as window state, filesystem events, sockets, or engine events. |
| `ExternalMessage` | Send a typed message to another subsystem. |

These categories are conceptual at this stage. They define the seams the implementation should preserve.

## Async Task Example

```rust
enum CounterAction {
    Increment,
    Save,
    SaveFinished(Result<(), SaveError>),
}

fn update(
    counter: &mut Counter,
    action: CounterAction,
    effects: &mut Effects<CounterAction>,
) {
    match action {
        CounterAction::Increment => {
            counter.count += 1;
        }

        CounterAction::Save => {
            let snapshot = counter.count;
            effects.task(async move {
                save_counter(snapshot).await
            }, CounterAction::SaveFinished);
        }

        CounterAction::SaveFinished(result) => {
            counter.last_save = Some(result);
        }
    }
}
```

The async task does not mutate `Counter` directly. It returns a value that is converted into `CounterAction::SaveFinished`, then the runtime dispatches that action through `update`.

## Host Command Example

```rust
enum EditorAction {
    PickFile,
    FilePicked(Option<FileHandle>),
}

fn update(
    editor: &mut Editor,
    action: EditorAction,
    effects: &mut Effects<EditorAction>,
) {
    match action {
        EditorAction::PickFile => {
            effects.host(HostCommand::open_file_dialog(), EditorAction::FilePicked);
        }

        EditorAction::FilePicked(file) => {
            editor.selected_file = file;
        }
    }
}
```

The host owns platform-specific behavior. The application receives the result as an action.

## Effect Lifecycle

A typical effect lifecycle is:

```text
Action
  -> update(State, Action, Effects)
  -> enqueue Effect
  -> Runtime records EffectRequested
  -> host/runtime executes effect
  -> effect completes, fails, or is cancelled
  -> completion maps to Action
  -> update(State, Action, Effects)
```

This keeps all state mutation inside `update` while still allowing real applications to perform external work.

## Ordering and Tracing

Effects should be ordered relative to the action that requested them. The runtime trace should be able to show:

```text
ActionDispatched
StateUpdated
EffectRequested
EffectStarted
EffectCompleted
ActionDispatched
StateUpdated
SurfaceFramePublished
```

This makes effects testable and debuggable. A test can assert that an action requested a specific effect without executing the real host operation.

## Cancellation

Long-running effects should be cancellable through stable effect identity:

```rust
effects.task_with_id(
    EffectId::new("search.current-query"),
    search(query),
    SearchAction::Finished,
);
```

A later action can cancel or replace the effect:

```rust
effects.cancel(EffectId::new("search.current-query"));
```

This is important for search boxes, live preview, file watchers, streaming data, and editor tooling.

## Design Constraint

Effects are part of the application/runtime boundary, not the renderer boundary. Renderers consume surface frames and primitives. Hosts and runtimes execute effects and feed resulting actions back into the application.
