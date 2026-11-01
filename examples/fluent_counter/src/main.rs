//! Genuine standalone executable consumer of the public fluent `RunenUI` facade.

use runenui_app::App;
use runenui_core::{NoHostProtocol, UiApp, View, button, children, column, text};

struct Counter {
    count: i32,
}

impl Counter {
    const fn new() -> Self {
        Self { count: 0 }
    }
}

enum CounterAction {
    Increment,
}

impl UiApp for Counter {
    type State = Self;
    type Action = CounterAction;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        column(children![
            text(format!("Count: {}", state.count)),
            button("+").on_activate(|| CounterAction::Increment),
        ])
    }

    fn update(
        state: &mut Self::State,
        action: Self::Action,
    ) -> impl runenui_core::IntoUpdateOutput<Self::Action, Self::HostProtocol> {
        match action {
            CounterAction::Increment => state.count += 1,
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    App::new().window("Counter").ui(Counter::new()).run()?;
    Ok(())
}
