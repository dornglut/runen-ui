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

    fn increment(&mut self) {
        // The introductory model never wraps or panics at the i32 boundary.
        self.count = self.count.saturating_add(1);
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
            CounterAction::Increment => state.increment(),
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    App::new().window("Counter").ui(Counter::new()).run()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counter_increment_saturates_at_the_numeric_boundary() {
        let mut counter = Counter { count: i32::MAX };
        counter.increment();
        assert_eq!(counter.count, i32::MAX);
    }
}
