//! Headless counter proof for `RunenUI`.
//!
//! The example owns its state, actions, update function, and screens. The
//! runtime owns typed action dispatch, update execution, root rebuilds, semantic
//! activation, and trace recording.

use runenui_core::{Element, button, column, row, text};
use runenui_runtime::{AppRuntime, UiApp};

const WIN_COUNT: i32 = 10;

#[derive(Clone, Debug, Eq, PartialEq)]
struct Counter {
    count: i32,
}

impl Counter {
    const fn new() -> Self {
        Self { count: 0 }
    }

    const fn has_won(&self) -> bool {
        self.count >= WIN_COUNT
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CounterAction {
    Decrement,
    Increment,
    Reset,
}

struct CounterApp;

impl UiApp for CounterApp {
    type State = Counter;
    type Action = CounterAction;

    fn root(state: &Self::State) -> Element<Self::Action> {
        root(state)
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        update(state, action);
    }
}

struct CounterScreen;

impl CounterScreen {
    fn root(counter: &Counter) -> Element<CounterAction> {
        column((
            text("Counter"),
            text(counter.count.to_string()).id("counter.value"),
            row((
                button("-")
                    .id("counter.decrement")
                    .on_press(CounterAction::Decrement),
                button("+")
                    .id("counter.increment")
                    .on_press(CounterAction::Increment),
                button("Reset")
                    .id("counter.reset")
                    .on_press(CounterAction::Reset),
            ))
            .gap(8_u16),
        ))
        .gap(8_u16)
    }
}

struct WinScreen;

impl WinScreen {
    fn root(counter: &Counter) -> Element<CounterAction> {
        let count = counter.count;

        column((
            text("You win"),
            text(format!("Count: {count}")).id("counter.value"),
            button("Reset")
                .id("counter.reset")
                .on_press(CounterAction::Reset),
        ))
        .gap(8_u16)
    }
}

fn root(counter: &Counter) -> Element<CounterAction> {
    if counter.has_won() {
        WinScreen::root(counter)
    } else {
        CounterScreen::root(counter)
    }
}

const fn update(counter: &mut Counter, action: CounterAction) {
    match action {
        CounterAction::Decrement => counter.count -= 1,
        CounterAction::Increment => counter.count += 1,
        CounterAction::Reset => counter.count = 0,
    }
}

fn main() {
    let mut runtime = AppRuntime::<CounterApp>::mount(Counter::new());

    for _ in 0..WIN_COUNT {
        runtime.activate("counter.increment");
    }

    runtime.activate("counter.reset");

    let count = runtime.state().count;
    let trace_events = runtime.trace().events().len();

    println!("counter.count={count} trace_events={trace_events}");
}

#[cfg(test)]
mod tests {
    use super::{AppRuntime, Counter, CounterAction, CounterApp, WIN_COUNT, root};
    use runenui_core::ElementKind;
    use runenui_runtime::ActivationResult;

    fn root_text(counter: &Counter) -> Result<String, &'static str> {
        let root = root(counter);
        let ElementKind::Container(container) = root.kind() else {
            return Err("expected root container");
        };
        let Some(title) = container.children().first() else {
            return Err("expected title element");
        };
        let ElementKind::Text(text) = title.kind() else {
            return Err("expected title text");
        };
        Ok(text.content().to_owned())
    }

    fn value_text(counter: &Counter) -> Result<String, &'static str> {
        let root = root(counter);
        let ElementKind::Container(container) = root.kind() else {
            return Err("expected root container");
        };
        let Some(value) = container.children().get(1) else {
            return Err("expected value element");
        };
        let ElementKind::Text(text) = value.kind() else {
            return Err("expected value text");
        };
        Ok(text.content().to_owned())
    }

    #[test]
    fn counter_screen_is_used_before_win_count() -> Result<(), &'static str> {
        let counter = Counter { count: 9 };

        assert_eq!(root_text(&counter)?, "Counter");
        assert_eq!(value_text(&counter)?, "9");
        Ok(())
    }

    #[test]
    fn win_screen_is_used_at_win_count() -> Result<(), &'static str> {
        let counter = Counter { count: 10 };

        assert_eq!(root_text(&counter)?, "You win");
        assert_eq!(value_text(&counter)?, "Count: 10");
        Ok(())
    }

    #[test]
    fn reset_returns_to_counter_screen() -> Result<(), &'static str> {
        let mut runtime = AppRuntime::<CounterApp>::mount(Counter { count: 10 });

        assert_eq!(
            runtime.activate("counter.reset"),
            ActivationResult::Dispatched
        );

        assert_eq!(runtime.state(), &Counter { count: 0 });
        assert_eq!(root_text(runtime.state())?, "Counter");
        Ok(())
    }

    #[test]
    fn semantic_increment_activation_reaches_win_screen() -> Result<(), &'static str> {
        let mut runtime = AppRuntime::<CounterApp>::mount(Counter::new());

        for _ in 0..WIN_COUNT {
            assert_eq!(
                runtime.activate("counter.increment"),
                ActivationResult::Dispatched
            );
        }

        assert_eq!(runtime.state(), &Counter { count: 10 });
        assert_eq!(root_text(runtime.state())?, "You win");
        Ok(())
    }

    #[test]
    fn direct_dispatch_still_works_without_authored_ids() {
        let mut runtime = AppRuntime::<CounterApp>::mount(Counter::new());

        runtime.dispatch(CounterAction::Increment);

        assert_eq!(runtime.state(), &Counter { count: 1 });
    }
}
