#![allow(refining_impl_trait)]

use std::rc::Rc;

use runenui_core::{
    CommandOrigin, Element, NoHostProtocol, SemanticCommand, UiApp, View, WorkKey, text,
};
use runenui_external_widget_conformance::{
    ExternalActivationSubscriptionWidget, ExternalSubscriptionLog, ExternalSubscriptionWidget,
};
use runenui_runtime::{
    AppRuntime, PumpBudget, SubscriptionDiagnostic, SubscriptionOwnerKind, TraceRecordKind,
};

enum Action {
    Refresh,
    Change,
    Suppress,
    Duplicate,
    Restore,
    Remove,
}

struct State {
    log: Rc<ExternalSubscriptionLog>,
    visible: bool,
    enabled: bool,
    duplicate: bool,
    revision: u64,
}

struct SubscriptionApp;

impl UiApp for SubscriptionApp {
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        // One owner; two source declarations, old cancellation/new starts and their terminal frontier.
        runenui_core::ProducerBudget::new(8, 1)
            .with_reconciliation(1, 1, 1)
            .with_subscriptions(0, 2)
            .with_work_starts(runenui_core::WorkStartBudget::new(2, 0, 0, 0, 0))
            .with_work_residency(runenui_core::WorkResidencyBudget::new(2, 0, 0, 2, 0, 0))
    }

    fn action_producer_budget(
        _state: &Self::State,
        _action: &Self::Action,
    ) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(8, 1)
            .with_reconciliation(1, 1, 1)
            .with_subscriptions(0, 2)
            .with_work_starts(runenui_core::WorkStartBudget::new(2, 0, 0, 0, 0))
            .with_work_residency(runenui_core::WorkResidencyBudget::new(2, 0, 0, 2, 0, 0))
    }

    fn interaction_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(8, 1)
            .with_reconciliation(1, 1, 1)
            .with_subscriptions(0, 2)
            .with_work_starts(runenui_core::WorkStartBudget::new(2, 0, 0, 0, 0))
            .with_work_residency(runenui_core::WorkResidencyBudget::new(2, 0, 0, 2, 0, 0))
    }

    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        if state.visible {
            Element::new(
                ExternalSubscriptionWidget::new(Rc::clone(&state.log))
                    .revision(state.revision)
                    .enabled(state.enabled)
                    .duplicate(state.duplicate),
            )
            .key("subscriber")
        } else {
            text("removed").into_element()
        }
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            Action::Refresh => {}
            Action::Change => state.revision = 2,
            Action::Suppress => state.enabled = false,
            Action::Duplicate => {
                state.enabled = true;
                state.duplicate = true;
                state.revision = 3;
            }
            Action::Restore => {
                state.duplicate = false;
                state.revision = 4;
            }
            Action::Remove => state.visible = false,
        }
    }
}

fn pump<Application: UiApp>(runtime: &mut AppRuntime<Application>) {
    let _ = runtime
        .pump(PumpBudget::new(
            usize::MAX,
            usize::MAX,
            usize::MAX,
            usize::MAX,
        ))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
}

#[test]
fn mounted_subscription_frontier_survives_refused_followers_and_zero_pumps() {
    use runenui_runtime::{RuntimeConfig, RuntimeStatus, SubmitActionErrorKind, TraceConfig};

    for trace_capacity in [0, 256] {
        let log = Rc::new(ExternalSubscriptionLog::default());
        let mut runtime = AppRuntime::<SubscriptionApp>::mount_with_config(
            State {
                log: Rc::clone(&log),
                visible: true,
                enabled: true,
                duplicate: false,
                revision: 1,
            },
            RuntimeConfig::default()
                .with_queue_capacity(16)
                .with_trace_config(TraceConfig::new(trace_capacity)),
        )
        .unwrap_or_else(|error| unreachable!("the declared initial frontier fits: {error}"));
        pump(&mut runtime);
        assert_eq!(runtime.status(), RuntimeStatus::Running);

        let actions = (0..4)
            .flat_map(|_| {
                [
                    Action::Change,
                    Action::Suppress,
                    Action::Duplicate,
                    Action::Restore,
                    Action::Refresh,
                ]
            })
            .chain(std::iter::once(Action::Remove));
        for action in actions {
            let before = (
                runtime.state().visible,
                runtime.state().enabled,
                runtime.state().duplicate,
                runtime.state().revision,
            );
            let expected = match &action {
                Action::Refresh => before,
                Action::Change => (before.0, before.1, before.2, 2),
                Action::Suppress => (before.0, false, before.2, before.3),
                Action::Duplicate => (before.0, true, true, 3),
                Action::Restore => (before.0, before.1, false, 4),
                Action::Remove => (false, before.1, before.2, before.3),
            };
            let declarations_before = log.declarations();
            let polls_before = log.polled_declarations().len();
            runtime
                .submit_action(action)
                .unwrap_or_else(|_| unreachable!("one whole turn fits"));
            let mut refused = false;
            for _ in 0..16 {
                if let Err(error) = runtime.submit_action(Action::Refresh) {
                    assert_eq!(error.kind(), SubmitActionErrorKind::Full);
                    assert!(matches!(error.into_action(), Action::Refresh));
                    refused = true;
                    break;
                }
            }
            assert!(
                refused,
                "finite admission must refuse a follower before mutation"
            );
            let batch = runtime
                .pump(PumpBudget::new(0, 0, 0, 0))
                .unwrap_or_else(|_| unreachable!("zero-budget observation is valid"));
            assert_eq!(batch.report().processed_envelopes(), 0);
            assert_eq!(log.declarations(), declarations_before);
            assert_eq!(log.polled_declarations().len(), polls_before);
            assert_eq!(
                (
                    runtime.state().visible,
                    runtime.state().enabled,
                    runtime.state().duplicate,
                    runtime.state().revision
                ),
                before
            );
            assert_eq!(runtime.status(), RuntimeStatus::Running);
            pump(&mut runtime);
            assert_eq!(runtime.status(), RuntimeStatus::Running);
            assert_eq!(
                (
                    runtime.state().visible,
                    runtime.state().enabled,
                    runtime.state().duplicate,
                    runtime.state().revision
                ),
                expected
            );
        }
        let polls_after_removal = log.polled_declarations().len();
        pump(&mut runtime);
        assert_eq!(log.polled_declarations().len(), polls_after_removal);
    }
}

#[test]
#[allow(clippy::assert_is_empty)]
fn downstream_mounted_subscription_reconciles_identity_duplicates_and_lifecycle() {
    let log = Rc::new(ExternalSubscriptionLog::default());
    let mut runtime = AppRuntime::<SubscriptionApp>::mount(State {
        log: Rc::clone(&log),
        visible: true,
        enabled: true,
        duplicate: false,
        revision: 1,
    })
    .unwrap_or_else(|error| unreachable!("fixture initial admission must succeed: {error}"));

    assert_eq!(log.declarations(), 0);
    pump(&mut runtime);
    assert_eq!(log.declarations(), 1);
    assert!(!log.polled_declarations().is_empty());
    assert!(
        log.polled_declarations()
            .iter()
            .all(|declaration| *declaration == 1)
    );

    runtime
        .submit_action(Action::Refresh)
        .unwrap_or_else(|_| unreachable!());
    pump(&mut runtime);
    assert_eq!(log.declarations(), 2);
    assert!(
        log.polled_declarations()
            .iter()
            .all(|declaration| *declaration == 1),
        "equal declarations must retain the original source"
    );

    runtime
        .submit_action(Action::Change)
        .unwrap_or_else(|_| unreachable!());
    pump(&mut runtime);
    assert_eq!(log.declarations(), 3);
    assert!(log.polled_declarations().contains(&3));

    runtime
        .submit_action(Action::Suppress)
        .unwrap_or_else(|_| unreachable!());
    pump(&mut runtime);
    let polls_after_absence = log.polled_declarations().len();
    pump(&mut runtime);
    assert_eq!(log.polled_declarations().len(), polls_after_absence);

    runtime
        .submit_action(Action::Duplicate)
        .unwrap_or_else(|_| unreachable!());
    pump(&mut runtime);
    assert_eq!(
        runtime.subscription_diagnostics(),
        &[SubscriptionDiagnostic::DuplicateKey {
            owner: SubscriptionOwnerKind::Mounted,
            key: WorkKey::new("external.subscription").unwrap_or_else(|_| unreachable!()),
        }]
    );
    let polls_after_duplicate = log.polled_declarations().len();
    pump(&mut runtime);
    assert_eq!(log.polled_declarations().len(), polls_after_duplicate);

    runtime
        .submit_action(Action::Restore)
        .unwrap_or_else(|_| unreachable!());
    pump(&mut runtime);
    assert!(log.polled_declarations().contains(&5));

    runtime
        .submit_action(Action::Remove)
        .unwrap_or_else(|_| unreachable!());
    pump(&mut runtime);
    let polls_after_unmount = log.polled_declarations().len();
    let declarations_after_unmount = log.declarations();
    pump(&mut runtime);
    assert_eq!(log.polled_declarations().len(), polls_after_unmount);
    assert_eq!(log.declarations(), declarations_after_unmount);
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ActivationAction {
    Primary,
    Auxiliary,
}

struct ActivationSubscriptionApp;

impl UiApp for ActivationSubscriptionApp {
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        // One owner; activation emits two actions and one mandatory declaration invalidation.
        runenui_core::ProducerBudget::new(5, 3)
            .with_reconciliation(1, 1, 1)
            .with_subscriptions(0, 0)
    }

    fn action_producer_budget(
        _state: &Self::State,
        _action: &Self::Action,
    ) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(5, 3)
            .with_reconciliation(1, 1, 1)
            .with_subscriptions(0, 0)
    }

    fn interaction_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(5, 3)
            .with_reconciliation(1, 1, 1)
            .with_subscriptions(0, 0)
    }

    type State = (Rc<ExternalSubscriptionLog>, Vec<ActivationAction>);
    type Action = ActivationAction;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        Element::new(ExternalActivationSubscriptionWidget::new(
            Rc::clone(&state.0),
            || ActivationAction::Primary,
            || ActivationAction::Auxiliary,
        ))
        .key("activation-subscriber")
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        state.1.push(action);
    }
}

#[test]
#[allow(clippy::assert_is_empty)]
fn downstream_activation_invalidates_current_declaration_before_ordered_actions() {
    let log = Rc::new(ExternalSubscriptionLog::default());
    let mut runtime = AppRuntime::<ActivationSubscriptionApp>::mount((Rc::clone(&log), Vec::new()))
        .unwrap_or_else(|error| unreachable!("fixture initial admission must succeed: {error}"));
    let _ = runtime
        .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(log.observed_states(), [0]);

    let target = runtime.index().nodes()[0].id().clone();
    runtime
        .submit_command(
            target,
            SemanticCommand::Activate,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("the exact live target is accepted"));
    let _ = runtime
        .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(log.observed_states(), [0]);
    let _ = runtime
        .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(log.observed_states(), [0, 1]);
    assert!(runtime.state().1.is_empty());

    pump(&mut runtime);
    assert_eq!(
        runtime.state().1,
        [ActivationAction::Primary, ActivationAction::Auxiliary]
    );
    assert_eq!(log.declarations(), 2);
}

#[derive(Clone, Copy)]
enum NewestAction {
    SetNewest,
    Primary,
    Auxiliary,
}

struct NewestState {
    log: Rc<ExternalSubscriptionLog>,
    widget_state: usize,
}

struct NewestDeclarationApp;

impl UiApp for NewestDeclarationApp {
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        // One owner; activation emits two actions and one mandatory declaration invalidation.
        runenui_core::ProducerBudget::new(5, 3)
            .with_reconciliation(1, 1, 1)
            .with_subscriptions(0, 0)
    }

    fn action_producer_budget(
        _state: &Self::State,
        _action: &Self::Action,
    ) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(5, 3)
            .with_reconciliation(1, 1, 1)
            .with_subscriptions(0, 0)
    }

    fn interaction_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(5, 3)
            .with_reconciliation(1, 1, 1)
            .with_subscriptions(0, 0)
    }

    type State = NewestState;
    type Action = NewestAction;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        Element::new(
            ExternalActivationSubscriptionWidget::new(
                Rc::clone(&state.log),
                || NewestAction::Primary,
                || NewestAction::Auxiliary,
            )
            .updated_state(state.widget_state),
        )
        .key("newest-declaration")
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        if matches!(action, NewestAction::SetNewest) {
            state.widget_state = 7;
        }
    }
}

#[test]
fn queued_mounted_reconciliation_observes_the_newest_live_widget_state() {
    let log = Rc::new(ExternalSubscriptionLog::default());
    let mut runtime = AppRuntime::<NewestDeclarationApp>::mount(NewestState {
        log: Rc::clone(&log),
        widget_state: 0,
    })
    .unwrap_or_else(|error| unreachable!("fixture initial admission must succeed: {error}"));
    let _ = runtime
        .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    let target = runtime.index().nodes()[0].id().clone();

    runtime
        .submit_command(
            target,
            SemanticCommand::Activate,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("the exact live target is accepted"));
    runtime
        .submit_action(NewestAction::SetNewest)
        .unwrap_or_else(|_| unreachable!());
    let _ = runtime
        .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(log.observed_states(), [0]);
    let _ = runtime
        .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(log.observed_states(), [0]);
    let _ = runtime
        .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(log.observed_states(), [0, 7]);
}

#[derive(Clone, Copy)]
enum RemovedDirtyAction {
    Remove,
    Primary,
    Auxiliary,
}

struct RemovedDirtyState {
    log: Rc<ExternalSubscriptionLog>,
    visible: bool,
}

struct RemovedDirtyApp;

impl UiApp for RemovedDirtyApp {
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        // One owner; activation emits two actions and one mandatory declaration invalidation.
        runenui_core::ProducerBudget::new(5, 3)
            .with_reconciliation(1, 1, 1)
            .with_subscriptions(0, 0)
    }

    fn action_producer_budget(
        _state: &Self::State,
        _action: &Self::Action,
    ) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(5, 3)
            .with_reconciliation(1, 1, 1)
            .with_subscriptions(0, 0)
    }

    fn interaction_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(5, 3)
            .with_reconciliation(1, 1, 1)
            .with_subscriptions(0, 0)
    }

    type State = RemovedDirtyState;
    type Action = RemovedDirtyAction;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        if state.visible {
            Element::new(ExternalActivationSubscriptionWidget::new(
                Rc::clone(&state.log),
                || RemovedDirtyAction::Primary,
                || RemovedDirtyAction::Auxiliary,
            ))
            .key("removed-dirty")
        } else {
            text("removed").into_element()
        }
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        if matches!(action, RemovedDirtyAction::Remove) {
            state.visible = false;
        }
    }
}

#[test]
fn removed_dirty_owner_suppresses_the_declaration_callback_at_its_envelope() {
    let log = Rc::new(ExternalSubscriptionLog::default());
    let mut runtime = AppRuntime::<RemovedDirtyApp>::mount(RemovedDirtyState {
        log: Rc::clone(&log),
        visible: true,
    })
    .unwrap_or_else(|error| unreachable!("fixture initial admission must succeed: {error}"));
    let _ = runtime
        .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    let target = runtime.index().nodes()[0].id().clone();
    runtime
        .submit_command(
            target.clone(),
            SemanticCommand::Activate,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("the exact live target is accepted"));
    runtime
        .submit_action(RemovedDirtyAction::Remove)
        .unwrap_or_else(|_| unreachable!());

    let _ = runtime
        .pump(PumpBudget::new(3, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(log.declarations(), 1);
    assert!(runtime.trace().records().any(|record| {
        matches!(
            record.kind(),
            TraceRecordKind::MountedSubscriptionReconciliationSuppressedStale
        ) && record.work_sequence().is_some()
            && record
                .target()
                .is_some_and(|trace_target| trace_target.mounted_node_id() == &target)
    }));
}
