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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CounterEndpoint {
    Minimum,
    Maximum,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CounterRejection {
    pub count: i32,
    pub action: CounterAction,
    pub endpoint: CounterEndpoint,
}

/// Application-owned feedback; successful followers preserve the latest
/// unacknowledged rejection. Reset acknowledges it explicitly.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CounterAppState {
    pub counter: Counter,
    pub latest_rejection: Option<CounterRejection>,
}
impl CounterAppState {
    pub const fn new(counter: Counter) -> Self {
        Self {
            counter,
            latest_rejection: None,
        }
    }
}

pub const fn update(counter: &mut Counter, action: CounterAction) -> Result<(), CounterRejection> {
    let after = match action {
        CounterAction::Increment => counter.count.checked_add(1),
        CounterAction::Decrement => counter.count.checked_sub(1),
        CounterAction::Reset => Some(0),
    };
    match after {
        Some(value) => {
            counter.count = value;
            Ok(())
        }
        None => Err(CounterRejection {
            count: counter.count,
            action,
            endpoint: match action {
                CounterAction::Decrement => CounterEndpoint::Minimum,
                CounterAction::Increment | CounterAction::Reset => CounterEndpoint::Maximum,
            },
        }),
    }
}

pub struct CounterApp;

impl CounterApp {
    const fn producer_budget() -> runenui_core::ProducerBudget {
        // The ordinary screen has 11 mounted owners; the win screen has 8.
        // Three buttons contribute at most 12 focus-notification outputs, plus
        // one ordinary event, activation, scrolling and default output each.
        // No app callback starts work or declares a subscription. Pointer
        // routing can start one cursor service per turn. Down and its funded
        // Up share a root, so two cursor records may coexist until host response
        // or cancellation retires the older record. There is no editable owner
        // or drag/drop source.
        // A screen change can dirty at most all 11 mounted owners at once.
        const MOUNTED_OWNERS: usize = 11;
        const OUTPUTS: usize = 4 * 3 + 4;
        const CURSOR_STARTS: usize = 1;
        runenui_core::ProducerBudget::new(
            1 + MOUNTED_OWNERS + OUTPUTS + 3 * CURSOR_STARTS,
            OUTPUTS + CURSOR_STARTS,
        )
        .with_reconciliation(MOUNTED_OWNERS, MOUNTED_OWNERS, MOUNTED_OWNERS)
        .with_work_starts(runenui_core::WorkStartBudget::new(
            0,
            0,
            0,
            0,
            CURSOR_STARTS,
        ))
        .with_work_residency(runenui_core::WorkResidencyBudget::new(
            0,
            0,
            0,
            0,
            0,
            2 * CURSOR_STARTS,
        ))
    }
}

impl UiApp for CounterApp {
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        Self::producer_budget()
    }

    fn action_producer_budget(
        _state: &Self::State,
        _action: &Self::Action,
    ) -> runenui_core::ProducerBudget {
        // Keep the bound valid across queued predecessors that cross WIN_COUNT.
        Self::producer_budget()
    }

    fn interaction_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        Self::producer_budget()
    }

    type State = CounterAppState;
    type Action = CounterAction;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        root(&state.counter)
    }

    fn update(
        state: &mut Self::State,
        action: Self::Action,
    ) -> impl runenui_core::IntoUpdateOutput<Self::Action, Self::HostProtocol> {
        match update(&mut state.counter, action) {
            Err(rejection) => state.latest_rejection = Some(rejection),
            Ok(()) if action == CounterAction::Reset => state.latest_rejection = None,
            Ok(()) => {}
        }
    }
}

#[cfg(test)]
mod contract_tests {
    use super::*;
    use runenui_runtime::{AppRuntime, PumpBudget, RuntimeStatus};
    #[test]
    fn signed_endpoints_and_queued_rejection_feedback_are_application_owned() {
        for (count, overflow, follower) in [
            (i32::MAX, CounterAction::Increment, CounterAction::Decrement),
            (i32::MIN, CounterAction::Decrement, CounterAction::Increment),
        ] {
            let mut runtime =
                AppRuntime::<CounterApp>::mount(CounterAppState::new(Counter { count }))
                    .unwrap_or_else(|_| unreachable!());
            let full = PumpBudget::new(128, 128, 128, 128);
            runtime.pump(full).unwrap_or_else(|_| unreachable!());
            runtime
                .submit_action(overflow)
                .unwrap_or_else(|_| unreachable!());
            runtime
                .submit_action(follower)
                .unwrap_or_else(|_| unreachable!());
            runtime.pump(full).unwrap_or_else(|_| unreachable!());
            assert_eq!(
                runtime
                    .state()
                    .latest_rejection
                    .as_ref()
                    .map(|value| value.count),
                Some(count)
            );
            assert_eq!(
                runtime.state().counter.count,
                if count == i32::MAX {
                    count - 1
                } else {
                    count + 1
                }
            );
            assert_eq!(runtime.status(), RuntimeStatus::Running);
            runtime
                .submit_action(CounterAction::Reset)
                .unwrap_or_else(|_| unreachable!());
            runtime
                .submit_action(CounterAction::Decrement)
                .unwrap_or_else(|_| unreachable!());
            runtime.pump(full).unwrap_or_else(|_| unreachable!());
            assert_eq!(runtime.state().latest_rejection, None);
            assert_eq!(runtime.state().counter.count, -1);
        }
    }
}
