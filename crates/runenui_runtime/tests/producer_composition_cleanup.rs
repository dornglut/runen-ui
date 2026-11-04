#![allow(refining_impl_trait)]

use std::{cell::RefCell, rc::Rc};

use runenui_core::{
    ChildBearingWidget, CommandOrigin, CompositionEvent, Element, EventContext, EventPhase,
    NoHostProtocol, ProducerBudget, SemanticCommand, UiApp, UiEvent, View, Widget,
    WidgetEventOutput, WidgetProducerBudget, WidgetProducerPhase, WidgetTextInput, container,
};
use runenui_runtime::{
    AppRuntime, PumpBudget, RuntimeConfig, RuntimeStatus, SubmitActionErrorKind, TraceConfig,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Action {
    Remove,
    RevokeText,
    Noop,
}

struct State {
    present: bool,
    text_capable: bool,
    noops: usize,
    log: Rc<RefCell<Vec<&'static str>>>,
}

#[derive(Debug)]
struct Parent;

impl Widget<Action> for Parent {
    type State = ();

    fn create_state(&self) {}

    fn producer_budget(&self, _: WidgetProducerPhase, (): &()) -> WidgetProducerBudget {
        WidgetProducerBudget::new(0).with_subscriptions(0)
    }
}

impl ChildBearingWidget<Action> for Parent {}

#[derive(Debug)]
struct Editor {
    log: Rc<RefCell<Vec<&'static str>>>,
    text_capable: bool,
}

impl Widget<Action> for Editor {
    type State = ();

    fn create_state(&self) {}

    fn producer_budget(&self, phase: WidgetProducerPhase, (): &()) -> WidgetProducerBudget {
        WidgetProducerBudget::new(usize::from(phase == WidgetProducerPhase::Event))
            .with_subscriptions(0)
    }

    fn text_input(&self, (): &()) -> WidgetTextInput {
        WidgetTextInput::new(self.text_capable, self.text_capable)
    }

    fn event(
        &mut self,
        (): &mut (),
        event: &UiEvent,
        context: &mut EventContext<'_, Action>,
    ) -> WidgetEventOutput {
        if context.phase() == EventPhase::Target
            && matches!(event, UiEvent::Composition(CompositionEvent::Cancel(_)))
        {
            self.log.borrow_mut().push("cancel");
            let _ = context.try_emit(Action::Noop);
        }
        WidgetEventOutput::none()
    }

    fn unmount(&self, (): &mut (), _: &mut runenui_core::WidgetUnmountContext) {
        self.log.borrow_mut().push("unmount");
    }
}

struct App;

impl UiApp for App {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn initial_producer_budget(_: &State) -> ProducerBudget {
        ProducerBudget::new(2, 0)
            .with_reconciliation(2, 0, 2)
            .with_subscriptions(0, 0)
    }

    fn action_producer_budget(_: &State, _: &Action) -> ProducerBudget {
        // Updates only remove or retain the two original owners. Retirement can
        // invoke exactly one output-producing composition-cancel callback;
        // its Noop descendant retains the resulting tree without fanout.
        ProducerBudget::new(4, 1)
            .with_reconciliation(0, 2, 0)
            .with_subscriptions(0, 0)
    }

    fn interaction_producer_budget(_: &State) -> ProducerBudget {
        // One emitting leaf and a pure parent: four focus notifications and
        // one ordinary routed event, plus retained-owner reconciliation.
        ProducerBudget::new(8, 5)
            .with_reconciliation(0, 2, 0)
            .with_subscriptions(0, 0)
    }

    fn root(state: &State) -> impl View<Action> {
        let children = if state.present {
            vec![
                Element::new(Editor {
                    log: Rc::clone(&state.log),
                    text_capable: state.text_capable,
                })
                .id("editor")
                .key("editor")
                .focusable(true),
            ]
        } else {
            Vec::new()
        };
        container(Parent, children).key("parent")
    }

    fn update(state: &mut State, action: Action) {
        match action {
            Action::Remove => state.present = false,
            Action::RevokeText => state.text_capable = false,
            Action::Noop => state.noops += 1,
        }
    }
}

const FULL: PumpBudget = PumpBudget::new(usize::MAX, usize::MAX, usize::MAX, usize::MAX);

fn composing_runtime(capacity: usize, trace_capacity: usize) -> AppRuntime<App> {
    let mut app = pending_composing_runtime(capacity, trace_capacity);
    assert!(
        app.pump(FULL)
            .unwrap_or_else(|_| unreachable!())
            .report()
            .is_quiescent()
    );
    app
}

fn pending_composing_runtime(capacity: usize, trace_capacity: usize) -> AppRuntime<App> {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut app = AppRuntime::<App>::mount_with_config(
        State {
            present: true,
            text_capable: true,
            noops: 0,
            log: Rc::clone(&log),
        },
        RuntimeConfig::default()
            .with_queue_capacity(capacity)
            .with_trace_config(TraceConfig::new(trace_capacity)),
    )
    .unwrap_or_else(|error| unreachable!("the two-owner initial frontier fits: {error}"));
    assert!(
        app.pump(FULL)
            .unwrap_or_else(|_| unreachable!())
            .report()
            .is_quiescent()
    );
    let editor = app.index().nodes()[1].id().clone();
    app.submit_command(
        editor,
        SemanticCommand::RequestFocus,
        CommandOrigin::programmatic(),
    )
    .unwrap_or_else(|error| unreachable!("the full interaction profile fits: {error}"));
    assert!(
        app.pump(FULL)
            .unwrap_or_else(|_| unreachable!())
            .report()
            .is_quiescent()
    );
    app.start_composition(None)
        .unwrap_or_else(|_| unreachable!("live text owner"));
    app
}

#[test]
fn accepted_removal_funds_composition_cleanup_with_full_follower_frontiers() {
    assert_funded_retirement(Action::Remove);
}

#[test]
fn accepted_capability_loss_funds_composition_cleanup_with_full_follower_frontiers() {
    assert_funded_retirement(Action::RevokeText);
}

#[test]
fn lifetime_frontier_refuses_an_independent_retirement_then_releases_for_exact_retry() {
    let mut app = composing_runtime(8, 512);
    let refused = app
        .submit_action(Action::Remove)
        .err()
        .unwrap_or_else(|| unreachable!("independent frontier cannot fit beside the lifetime"));
    assert_eq!(refused.kind(), SubmitActionErrorKind::Full);
    let action = refused.into_action();
    assert!(app.state().present);
    assert!(app.state().log.borrow().is_empty());
    let generation = app
        .input_ownership()
        .unwrap_or_else(|_| unreachable!())
        .keyboard()
        .composition_generation()
        .cloned()
        .unwrap_or_else(|| unreachable!());
    app.cancel_composition(generation)
        .unwrap_or_else(|_| unreachable!("lifetime claim funds cancel"));
    app.pump(FULL).unwrap_or_else(|_| unreachable!());
    assert_eq!(&*app.state().log.borrow(), &["cancel"]);
    app.submit_action(action)
        .unwrap_or_else(|_| unreachable!("the exact owned action fits after retirement"));
    app.pump(FULL).unwrap_or_else(|_| unreachable!());
    assert!(!app.state().present);
    assert_eq!(app.status(), RuntimeStatus::Running);
    assert_eq!(&*app.state().log.borrow(), &["cancel", "unmount"]);
}

fn assert_funded_retirement(retirement: Action) {
    // Eight credits belong to the still-live composition; the independent
    // retirement action requires four more before any mutation.
    for (capacity, trace_capacity) in [12, 16, 32]
        .into_iter()
        .flat_map(|capacity| [0, 512].map(|trace| (capacity, trace)))
    {
        let mut app = composing_runtime(capacity, trace_capacity);
        let log = Rc::clone(&app.state().log);
        app.submit_action(retirement)
            .unwrap_or_else(|_| unreachable!("retirement frontier fits"));
        let mut followers = 0;
        let refused = loop {
            match app.submit_action(Action::Noop) {
                Ok(_) => {
                    followers += 1;
                    assert!(
                        followers < capacity,
                        "finite queue must refuse the next root"
                    );
                }
                Err(error) => break error,
            }
        };
        assert_eq!(refused.kind(), SubmitActionErrorKind::Full);
        assert_eq!(refused.into_action(), Action::Noop);
        assert_eq!(
            followers,
            (capacity - 8) / 4 - 1,
            "all independent roots fit beside the lifetime's eight credits"
        );
        let zero = app
            .pump(PumpBudget::new(0, 0, 0, 0))
            .unwrap_or_else(|_| unreachable!());
        assert_eq!(zero.report().processed_envelopes(), 0);
        assert!(app.state().present);
        assert!(log.borrow().is_empty());

        let removal = app
            .pump(PumpBudget::new(1, 0, 0, 0))
            .unwrap_or_else(|_| unreachable!());
        assert_eq!(removal.report().processed_envelopes(), 1);
        assert_eq!(
            app.status(),
            RuntimeStatus::Running,
            "ordinary queue pressure must not erase funded cleanup"
        );
        assert_eq!(app.state().present, retirement != Action::Remove);
        assert_eq!(
            app.state().noops,
            0,
            "derived cleanup output remains FIFO work"
        );
        let expected: &[&str] = if retirement == Action::Remove {
            &["cancel", "unmount"]
        } else {
            &["cancel"]
        };
        assert_eq!(&*log.borrow(), expected);
        assert_eq!(app.focused_text_input_capability(), WidgetTextInput::NONE);
        assert!(
            app.pump(FULL)
                .unwrap_or_else(|_| unreachable!())
                .report()
                .is_quiescent()
        );
        assert_eq!(app.status(), RuntimeStatus::Running);
        assert_eq!(
            app.state().noops,
            followers + 1,
            "every accepted follower and the cleanup output commits once"
        );
        assert_eq!(&*log.borrow(), expected);
        assert!(
            app.submit_action(Action::Noop).is_ok(),
            "all completed root frontiers release their capacity"
        );
    }
}

// U475-REVIEW-SHUTDOWN-COMPOSITION-01: the existing lifetime, not ambient
// headroom, funds synchronous cancellation before unmount and scope retirement.
#[test]
fn shutdown_delivers_composition_cancel_with_full_follower_frontiers() {
    for (saturated, trace) in [false, true]
        .into_iter()
        .flat_map(|saturated| [0, 512].map(move |trace| (saturated, trace)))
    {
        let mut app = composing_runtime(12, trace);
        if saturated {
            app.submit_action(Action::Noop)
                .unwrap_or_else(|_| unreachable!());
            assert_eq!(
                app.submit_action(Action::Noop)
                    .err()
                    .unwrap_or_else(|| unreachable!())
                    .kind(),
                SubmitActionErrorKind::Full
            );
        }
        let batch = app.shutdown().unwrap_or_else(|_| unreachable!());
        assert_eq!(&*app.state().log.borrow(), &["cancel", "unmount"]);
        assert_eq!(
            app.state().noops,
            0,
            "shutdown does not execute queued actions"
        );
        assert_eq!(
            batch.report().cancelled_queued_envelopes(),
            1 + usize::from(saturated)
        );
        assert_shutdown_retirement(&batch);
    }
}

#[test]
fn shutdown_cancels_inflight_composition_envelopes_without_losing_their_root() {
    for (pending, many, trace) in [false, true].into_iter().flat_map(|pending| {
        [false, true]
            .into_iter()
            .flat_map(move |many| [0, 512].map(move |trace| (pending, many, trace)))
    }) {
        let mut app = if pending {
            pending_composing_runtime(12, trace)
        } else {
            composing_runtime(12, trace)
        };
        let generation = app
            .input_ownership()
            .unwrap_or_else(|_| unreachable!())
            .keyboard()
            .composition_generation()
            .cloned()
            .unwrap_or_else(|| unreachable!());
        app.cancel_composition(generation.clone())
            .unwrap_or_else(|_| unreachable!());
        let mut cancellations = 1;
        if many {
            loop {
                match app.cancel_composition(generation.clone()) {
                    Ok(_) => {
                        cancellations += 1;
                        assert!(
                            cancellations <= 8,
                            "overlap remains inside the eight-credit frontier"
                        );
                    }
                    Err(refused) => {
                        assert_eq!(
                            refused.kind(),
                            runenui_runtime::SubmitCompositionErrorKind::Full
                        );
                        break;
                    }
                }
            }
        }
        app.submit_action(Action::Noop)
            .unwrap_or_else(|_| unreachable!());
        let batch = app.shutdown().unwrap_or_else(|_| unreachable!());
        assert_eq!(&*app.state().log.borrow(), &["cancel", "unmount"]);
        assert_eq!(app.state().noops, 0);
        // Cancelled Start/Cancel inputs and the callback's tail output are
        // disposed, not processed or relabelled as settled native input.
        assert_eq!(
            batch.report().cancelled_queued_envelopes(),
            cancellations + usize::from(pending) + 2
        );
        assert_shutdown_retirement(&batch);
        let repeated = app.shutdown().unwrap_or_else(|_| unreachable!());
        assert!(repeated.report().already_complete());
        assert_eq!(repeated.ordered_records(), []);
        assert_eq!(&*app.state().log.borrow(), &["cancel", "unmount"]);
    }
}

#[test]
fn extraction_and_drop_share_the_funded_composition_shutdown_path() {
    for trace in [0, 512] {
        let mut extraction = composing_runtime(12, trace);
        extraction
            .submit_action(Action::Noop)
            .unwrap_or_else(|_| unreachable!());
        let state = extraction.into_state();
        assert_eq!(&*state.log.borrow(), &["cancel", "unmount"]);
        assert_eq!(state.noops, 0);
        let mut dropped = pending_composing_runtime(12, trace);
        let log = Rc::clone(&dropped.state().log);
        dropped
            .submit_action(Action::Noop)
            .unwrap_or_else(|_| unreachable!());
        drop(dropped);
        assert_eq!(&*log.borrow(), &["cancel", "unmount"]);
    }
}

fn assert_shutdown_retirement(batch: &runenui_runtime::InputShutdownBatch) {
    assert_eq!(batch.final_ownership().status(), RuntimeStatus::Closed);
    assert_eq!(batch.ordered_records().iter().filter(|record| matches!(record,
        runenui_runtime::InputArbitrationRecord::ScopeRetired(retirement)
            if matches!(retirement.reason(), runenui_runtime::InputScopeRetirementReason::Shutdown)
    )).count(), 1);
    assert!(!batch.ordered_records().iter().any(|record| matches!(
        record,
        runenui_runtime::InputArbitrationRecord::InputSettled(_)
    )));
}
