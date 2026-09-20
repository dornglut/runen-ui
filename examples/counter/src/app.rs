use runenui_core::{NoHostProtocol, UiApp, View};

use crate::ui::root;

pub const WIN_COUNT: i32 = 10;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Counter {
    pub count: i32,
}

impl Counter {
    pub const fn new() -> Self {
        Self { count: 0 }
    }

    pub const fn has_won(&self) -> bool {
        self.count >= WIN_COUNT
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CounterAction {
    Decrement,
    Increment,
    Reset,
}

pub const fn update(counter: &mut Counter, action: CounterAction) {
    match action {
        CounterAction::Decrement => counter.count -= 1,
        CounterAction::Increment => counter.count += 1,
        CounterAction::Reset => counter.count = 0,
    }
}

pub struct CounterApp;

impl UiApp for CounterApp {
    type State = Counter;
    type Action = CounterAction;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        root(state)
    }

    fn update(
        state: &mut Self::State,
        action: Self::Action,
    ) -> impl runenui_core::IntoUpdateOutput<Self::Action, Self::HostProtocol> {
        update(state, action);
    }
}
