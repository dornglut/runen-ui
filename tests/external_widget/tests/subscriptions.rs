#![allow(refining_impl_trait)]

use std::rc::Rc;

use runenui_core::{Element, NoHostProtocol, UiApp, View, WorkKey, text};
use runenui_external_widget_conformance::{
    ExternalActivationSubscriptionWidget, ExternalSubscriptionLog, ExternalSubscriptionWidget,
};
use runenui_runtime::{
    ActivationResult, AppRuntime, PumpBudget, SubscriptionDiagnostic, SubscriptionOwnerKind,
    TraceRecordKind,
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
    runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
}

#[test]
fn downstream_mounted_subscription_reconciles_identity_duplicates_and_lifecycle() {
    let log = Rc::new(ExternalSubscriptionLog::default());
    let mut runtime = AppRuntime::<SubscriptionApp>::mount(State {
        log: Rc::clone(&log),
        visible: true,
        enabled: true,
        duplicate: false,
        revision: 1,
    });

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
fn downstream_activation_invalidates_current_declaration_before_ordered_actions() {
    let log = Rc::new(ExternalSubscriptionLog::default());
    let mut runtime = AppRuntime::<ActivationSubscriptionApp>::mount((Rc::clone(&log), Vec::new()));
    runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(log.observed_states(), [0]);

    let target = runtime.index().nodes()[0].id().clone();
    assert!(matches!(
        runtime.activate_node(&target),
        ActivationResult::Queued(_)
    ));
    runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));
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
    });
    runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));
    let target = runtime.index().nodes()[0].id().clone();

    runtime
        .submit_action(NewestAction::SetNewest)
        .unwrap_or_else(|_| unreachable!());
    assert!(matches!(
        runtime.activate_node(&target),
        ActivationResult::Queued(_)
    ));
    runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(log.observed_states(), [0]);
    runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));
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
    });
    runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));
    let target = runtime.index().nodes()[0].id().clone();
    runtime
        .submit_action(RemovedDirtyAction::Remove)
        .unwrap_or_else(|_| unreachable!());
    assert!(matches!(
        runtime.activate_node(&target),
        ActivationResult::Queued(_)
    ));

    runtime.pump(PumpBudget::new(2, usize::MAX, usize::MAX, usize::MAX));
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
