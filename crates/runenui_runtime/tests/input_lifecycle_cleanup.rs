#![allow(refining_impl_trait)]

use std::{cell::RefCell, rc::Rc};

use runenui_core::{
    ChildBearingWidget, CommandOrigin, CompositionCancelReason, CompositionEvent, Element,
    EventContext, EventPhase, NoHostProtocol, SemanticCommand, UiApp, UiEvent, View, Widget,
    WidgetEventOutput, WidgetTextInput, container,
};
use runenui_runtime::{
    AppRuntime, PumpBudget, RuntimeConfig, RuntimeLimits, RuntimeStatus, SubmitActionErrorKind,
    TraceRecord, TraceRecordKind, TraceSequence, WorkSequence,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Action {
    Remove,
    Noop,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Fact {
    Cancel(CompositionCancelReason),
    Unmounted,
}

#[derive(Debug)]
struct State {
    log: Rc<RefCell<Vec<Fact>>>,
    target_present: bool,
}

#[derive(Debug)]
struct RootWidget;

impl Widget<Action> for RootWidget {
    fn producer_budget(
        &self,
        phase: runenui_core::WidgetProducerPhase,
        _state: &Self::State,
    ) -> runenui_core::WidgetProducerBudget {
        match phase {
            runenui_core::WidgetProducerPhase::Activation => {
                runenui_core::WidgetProducerBudget::new(0)
            }
            runenui_core::WidgetProducerPhase::Subscriptions => {
                runenui_core::WidgetProducerBudget::new(0).with_subscriptions(0)
            }
            _ => runenui_core::WidgetProducerBudget::new(0),
        }
    }

    type State = ();

    fn create_state(&self) -> Self::State {}
}

impl ChildBearingWidget<Action> for RootWidget {}

#[derive(Debug)]
struct TargetWidget {
    log: Rc<RefCell<Vec<Fact>>>,
}

impl Widget<Action> for TargetWidget {
    fn producer_budget(
        &self,
        phase: runenui_core::WidgetProducerPhase,
        _state: &Self::State,
    ) -> runenui_core::WidgetProducerBudget {
        match phase {
            runenui_core::WidgetProducerPhase::Activation => {
                runenui_core::WidgetProducerBudget::new(0)
            }
            runenui_core::WidgetProducerPhase::Subscriptions => {
                runenui_core::WidgetProducerBudget::new(0).with_subscriptions(0)
            }
            _ => runenui_core::WidgetProducerBudget::new(0),
        }
    }

    type State = ();

    fn create_state(&self) -> Self::State {}

    fn event(
        &mut self,
        (): &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, Action>,
    ) -> WidgetEventOutput {
        if context.phase() == EventPhase::Target
            && let UiEvent::Composition(CompositionEvent::Cancel(cancel)) = event
        {
            self.log.borrow_mut().push(Fact::Cancel(cancel.reason()));
        }
        WidgetEventOutput::none()
    }

    fn text_input(&self, (): &Self::State) -> WidgetTextInput {
        WidgetTextInput::new(true, true)
    }

    fn unmount(&self, (): &mut Self::State, _: &mut runenui_core::WidgetUnmountContext) {
        self.log.borrow_mut().push(Fact::Unmounted);
    }
}

struct App;

impl UiApp for App {
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(2, 0)
            .with_reconciliation(2, 0, 2)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn action_producer_budget(
        _state: &Self::State,
        action: &Self::Action,
    ) -> runenui_core::ProducerBudget {
        match action {
            Action::Noop => runenui_core::ProducerBudget::new(1, 0),
            Action::Remove => runenui_core::ProducerBudget::new(3, 0)
                .with_reconciliation(1, 1, 1)
                .with_subscriptions(0, 0)
                .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0)),
        }
    }

    fn interaction_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(4, 1)
            .with_reconciliation(2, 2, 2)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 1))
            .with_work_residency(runenui_core::WorkResidencyBudget::new(0, 0, 0, 0, 0, 1))
    }

    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        let children = if state.target_present {
            vec![
                Element::new(TargetWidget {
                    log: Rc::clone(&state.log),
                })
                .id("target")
                .key("target")
                .focusable(true),
            ]
        } else {
            Vec::new()
        };
        container(RootWidget, children).id("root").key("root")
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            Action::Remove => state.target_present = false,
            Action::Noop => {}
        }
    }
}

fn settle(runtime: &mut AppRuntime<App>) {
    let report = runtime
        .pump(PumpBudget::new(
            usize::MAX,
            usize::MAX,
            usize::MAX,
            usize::MAX,
        ))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert!(report.is_quiescent(), "fixture did not settle: {report:?}");
}

fn mount_with_config(log: Rc<RefCell<Vec<Fact>>>, config: RuntimeConfig) -> AppRuntime<App> {
    let mut runtime = AppRuntime::<App>::mount_with_config(
        State {
            log,
            target_present: true,
        },
        config,
    )
    .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    settle(&mut runtime);
    runtime
}

fn target(runtime: &mut AppRuntime<App>) -> runenui_runtime::MountedNodeId {
    let authored =
        runenui_core::ElementId::new("target").unwrap_or_else(|_| unreachable!("valid id"));
    runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored))
        .unwrap_or_else(|| unreachable!("target remains mounted"))
        .id()
        .clone()
}

fn focus_target(runtime: &mut AppRuntime<App>) {
    let target = target(runtime);
    runtime
        .submit_command(
            target,
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("focus request is accepted"));
    settle(runtime);
}

fn find_record(
    runtime: &AppRuntime<App>,
    predicate: impl Fn(&TraceRecord) -> bool,
) -> &TraceRecord {
    runtime
        .trace()
        .records()
        .find(|record| predicate(record))
        .unwrap_or_else(|| unreachable!("required trace record is retained"))
}

fn ancestry_contains_work(
    runtime: &AppRuntime<App>,
    mut parent: Option<TraceSequence>,
    expected: WorkSequence,
) -> bool {
    for _ in 0..runtime.trace().len() {
        let Some(sequence) = parent else {
            return false;
        };
        let Some(record) = runtime
            .trace()
            .records()
            .find(|record| record.sequence() == sequence)
        else {
            return false;
        };
        if record.work_sequence() == Some(expected) {
            return true;
        }
        parent = record.causal_parent();
    }
    false
}

#[test]
fn removal_cleanup_keeps_composition_sequence_and_links_the_removing_action() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = mount_with_config(Rc::clone(&log), RuntimeConfig::default());
    focus_target(&mut runtime);
    let start = runtime
        .start_composition(None)
        .unwrap_or_else(|_| unreachable!("composition start is accepted"));
    settle(&mut runtime);
    log.borrow_mut().clear();

    let removal = runtime
        .submit_action(Action::Remove)
        .unwrap_or_else(|_| unreachable!("removal is accepted"));
    settle(&mut runtime);

    assert_eq!(
        log.borrow().as_slice(),
        [
            Fact::Cancel(CompositionCancelReason::Removal),
            Fact::Unmounted,
        ]
    );
    let cancelled = find_record(&runtime, |record| {
        matches!(
            record.kind(),
            TraceRecordKind::CompositionCancelled {
                reason: CompositionCancelReason::Removal
            }
        )
    });
    assert_eq!(cancelled.work_sequence(), Some(start.sequence()));
    assert!(ancestry_contains_work(
        &runtime,
        cancelled.causal_parent(),
        removal
    ));
    assert_eq!(runtime.status(), RuntimeStatus::Running);
}

#[test]
#[allow(clippy::assert_is_empty)]
fn cleanup_capacity_refusal_preserves_composition_for_an_exact_retry() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let config = RuntimeConfig::default().with_limits(
        RuntimeLimits::default()
            // Four lifetime credits coexist with three unary followers and
            // one free position, insufficient for the three-credit removal.
            .with_waiting_envelopes(8)
            .with_transaction_outputs(1),
    );
    let mut runtime = mount_with_config(Rc::clone(&log), config);
    focus_target(&mut runtime);
    let start = runtime
        .start_composition(None)
        .unwrap_or_else(|_| unreachable!("composition start is accepted"));
    settle(&mut runtime);
    log.borrow_mut().clear();

    for _ in 0..3 {
        runtime
            .submit_action(Action::Noop)
            .unwrap_or_else(|_| unreachable!("filler occupies cleanup capacity"));
    }
    let Err(refused) = runtime.submit_action(Action::Remove) else {
        unreachable!("removal cannot consume the live cleanup reserve");
    };
    assert_eq!(refused.kind(), SubmitActionErrorKind::Full);
    let removal = refused.into_action();
    assert_eq!(runtime.status(), RuntimeStatus::Running);
    assert!(runtime.state().target_present);
    assert!(log.borrow().is_empty());
    assert!(!runtime.trace().records().any(|record| matches!(
        record.kind(),
        TraceRecordKind::CompositionCancelled {
            reason: CompositionCancelReason::Removal
        }
    )));

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
    runtime.submit_action(removal).unwrap_or_else(|_| {
        unreachable!("the exact refused removal retries after capacity returns")
    });
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

    assert_eq!(runtime.status(), RuntimeStatus::Running);
    assert!(!runtime.state().target_present);
    assert_eq!(
        *log.borrow(),
        [
            Fact::Cancel(CompositionCancelReason::Removal),
            Fact::Unmounted,
        ]
    );
    assert!(runtime.trace().records().any(|record| matches!(
        record.kind(),
        TraceRecordKind::CompositionCancelled {
            reason: CompositionCancelReason::Removal
        }
    ) && record.work_sequence()
        == Some(start.sequence())));

    let report = runtime
        .shutdown()
        .unwrap_or_else(|_| unreachable!("shutdown observation"))
        .report()
        .to_owned();
    assert_eq!(report.unmounted_lifetimes(), 1);
    assert_eq!(runtime.status(), RuntimeStatus::Closed);
    assert_eq!(
        log.borrow()
            .iter()
            .filter(|fact| **fact == Fact::Unmounted)
            .count(),
        1
    );
}
