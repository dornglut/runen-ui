//! Independent native timed-status consumer of the public fluent `RunenUI` host.

use std::time::Duration;

use runenui_app::App;
use runenui_core::{
    Effects, IntoEffects, NoHostProtocol, TimerEffect, UiApp, View, button, children, column, text,
};

#[derive(Default)]
struct StatusState {
    running: bool,
    timer_fired: bool,
}

enum StatusAction {
    ToggleRunning,
    InitialTimerFired,
}

struct StatusUi;

impl UiApp for StatusUi {
    type State = StatusState;
    type Action = StatusAction;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        column(children![
            text(if state.running { "Running" } else { "Paused" }),
            button(if state.running { "Pause" } else { "Start" })
                .on_activate(|| StatusAction::ToggleRunning),
            text(if state.timer_fired {
                "Initial scheduled check finished"
            } else {
                "Waiting for initial scheduled check"
            }),
        ])
    }

    fn initial_effects(_: &Self::State) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        Effects::timer(TimerEffect::once(Duration::from_millis(1_500), || {
            StatusAction::InitialTimerFired
        }))
    }

    fn update(
        state: &mut Self::State,
        action: Self::Action,
    ) -> impl runenui_core::IntoUpdateOutput<Self::Action, Self::HostProtocol> {
        match action {
            StatusAction::ToggleRunning => state.running = !state.running,
            StatusAction::InitialTimerFired => state.timer_fired = true,
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    App::new()
        .window("Status")
        .ui_with_state::<StatusUi>(StatusState::default())
        .run()?;
    Ok(())
}
