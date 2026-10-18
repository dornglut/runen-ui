#![allow(refining_impl_trait)]

// Proof-only characterization for #480. The public application update itself
// emits no effects. A compatible retained widget may emit actions only after
// application state has already changed and reconciliation reaches the widget.
// This test does not implement a new admission policy.
use runenui_core::{
    Element, NoHostProtocol, UiApp, Widget, WidgetUpdateContext,
};
use runenui_runtime::{
    AppRuntime, PumpBudget, PumpOutcome, RuntimeConfig, RuntimeLimits, RuntimeStatus,
    RuntimeTerminalReason,
};

#[derive(Clone, Copy)]
enum Action {
    Change,
    Follower,
    MountedOutput,
}

struct State {
    revision: usize,
    emitted_per_widget_update: usize,
    processed_actions: usize,
}

#[derive(Debug)]
struct EmittingCompatibleWidget {
    revision: usize,
    outputs: usize,
}

impl Widget<Action> for EmittingCompatibleWidget {
    type State = usize;

    fn create_state(&self) -> Self::State {
        self.revision
    }

    fn update(&self, mounted_revision: &mut Self::State, context: &mut WidgetUpdateContext<Action>) {
        if *mounted_revision != self.revision {
            *mounted_revision = self.revision;
            for _ in 0..self.outputs {
                context.emit(Action::MountedOutput);
            }
        }
    }
}

struct NoApplicationEffectsApp;

impl UiApp for NoApplicationEffectsApp {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> Element<Self::Action> {
        Element::new(EmittingCompatibleWidget {
            revision: state.revision,
            outputs: state.emitted_per_widget_update,
        })
        .key("retained-output-probe")
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        state.processed_actions += 1;
        if matches!(action, Action::Change) {
            state.revision += 1;
        }
        // Intentionally no application effects: the only new work comes from
        // the *mounted* compatible widget's post-mutation update callback.
    }
}

#[test]
fn app_zero_outputs_do_not_bound_retained_widget_update_outputs() {
    const CAPACITY: usize = 16;

    // App's direct output bound is zero in all four cases; the retained
    // widget's output cardinality changes without replacing mounted identity.
    // One popped ingress action frees exactly one FIFO position.
    for (mounted_outputs, already_queued_followers, should_poison, remaining) in [
        (0, 15, false, 15),
        (1, 15, false, 16),
        (2, 14, false, 16),
        (2, 15, true, 0),
    ] {
        let limits = RuntimeLimits::default()
            .with_waiting_envelopes(CAPACITY)
            .with_transaction_outputs(2);
        let config = RuntimeConfig::default().with_limits(limits);
        let mut runtime = AppRuntime::<NoApplicationEffectsApp>::mount_with_config(
            State {
                revision: 0,
                emitted_per_widget_update: mounted_outputs,
                processed_actions: 0,
            },
            config,
        );
        let initial = runtime
            .pump(PumpBudget::new(usize::MAX, 0, 0, 0))
            .unwrap_or_else(|_| unreachable!("initial publication pump"))
            .report()
            .to_owned();
        assert!(initial.is_quiescent());

        runtime
            .submit_action(Action::Change)
            .unwrap_or_else(|_| unreachable!("first action accepted"));
        for _ in 0..already_queued_followers {
            runtime
                .submit_action(Action::Follower)
                .unwrap_or_else(|_| unreachable!("accepted waiting follower"));
        }
        let report = runtime
            .pump(PumpBudget::new(1, 0, 0, 0))
            .unwrap_or_else(|_| unreachable!("one-envelope pump"))
            .report()
            .to_owned();
        assert_eq!(report.processed_envelopes(), 1);
        assert_eq!(runtime.state().processed_actions, 1);
        assert_eq!(runtime.state().revision, 1);
        assert_eq!(report.remaining_queued_envelopes(), remaining);

        if should_poison {
            assert_eq!(
                runtime.status(),
                RuntimeStatus::Terminal(RuntimeTerminalReason::Poisoned)
            );
            assert_eq!(
                report.outcome(),
                PumpOutcome::Terminal(RuntimeTerminalReason::Poisoned)
            );
        } else {
            assert_eq!(runtime.status(), RuntimeStatus::Running);
        }
    }
}
