#![allow(refining_impl_trait)]

use std::{cell::RefCell, rc::Rc};

use runenui_core::{
    ChildLayout, ChildLayoutWidget, CommandOrigin, CompositionCancelReason, CompositionEvent,
    Element, EventContext, EventPhase, NoHostProtocol, SemanticCommand, UiApp, UiEvent, View,
    Widget, WidgetEventOutput, WidgetTextInput, container,
};
use runenui_runtime::{
    AppRuntime, PumpBudget, RuntimeConfig, RuntimeLimits, RuntimeStatus, RuntimeTerminalReason,
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
    type State = ();

    fn create_state(&self) -> Self::State {}
}

impl ChildLayoutWidget<Action> for RootWidget {
    fn child_layout(&self, (): &Self::State) -> ChildLayout {
        ChildLayout::Linear {
            axis: runenui_core::Axis::Vertical,
        }
    }
}

#[derive(Debug)]
struct TargetWidget {
    log: Rc<RefCell<Vec<Fact>>>,
}

impl Widget<Action> for TargetWidget {
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
    let report = runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
    assert!(report.is_quiescent(), "fixture did not settle: {report:?}");
}

fn mount_with_config(log: Rc<RefCell<Vec<Fact>>>, config: RuntimeConfig) -> AppRuntime<App> {
    let mut runtime = AppRuntime::<App>::mount_with_config(
        State {
            log,
            target_present: true,
        },
        config,
    );
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

fn ancestry_contains_sequence(
    runtime: &AppRuntime<App>,
    mut parent: Option<TraceSequence>,
    expected: TraceSequence,
) -> bool {
    for _ in 0..runtime.trace().len() {
        let Some(sequence) = parent else {
            return false;
        };
        if sequence == expected {
            return true;
        }
        let Some(record) = runtime
            .trace()
            .records()
            .find(|record| record.sequence() == sequence)
        else {
            return false;
        };
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
fn failed_cleanup_retires_without_false_delivery_and_shutdown_unmounts_once() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let config = RuntimeConfig::default().with_limits(
        RuntimeLimits::default()
            .with_waiting_envelopes(3)
            .with_transaction_outputs(1),
    );
    let mut runtime = mount_with_config(Rc::clone(&log), config);
    focus_target(&mut runtime);
    let start = runtime
        .start_composition(None)
        .unwrap_or_else(|_| unreachable!("composition start is accepted"));
    settle(&mut runtime);
    log.borrow_mut().clear();

    let removal = runtime
        .submit_action(Action::Remove)
        .unwrap_or_else(|_| unreachable!("removal occupies the FIFO head"));
    for _ in 0..2 {
        runtime
            .submit_action(Action::Noop)
            .unwrap_or_else(|_| unreachable!("filler occupies cleanup capacity"));
    }
    let _ = runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));

    assert_eq!(
        runtime.status(),
        RuntimeStatus::Terminal(RuntimeTerminalReason::Poisoned)
    );
    assert!(log.borrow().is_empty());
    assert!(!runtime.trace().records().any(|record| matches!(
        record.kind(),
        TraceRecordKind::CompositionCancelled {
            reason: CompositionCancelReason::Removal
        }
    )));

    let retired = find_record(&runtime, |record| {
        matches!(record.kind(), TraceRecordKind::CompositionRetired)
            && record.work_sequence() == Some(start.sequence())
            && ancestry_contains_work(&runtime, record.causal_parent(), removal)
    });
    let terminal = find_record(&runtime, |record| {
        matches!(
            record.kind(),
            TraceRecordKind::RuntimeTerminal {
                reason: RuntimeTerminalReason::Poisoned
            }
        )
    });
    assert!(ancestry_contains_sequence(
        &runtime,
        terminal.causal_parent(),
        retired.sequence()
    ));
    let terminal_sequence = terminal.sequence();

    let report = runtime.shutdown();
    assert_eq!(report.unmounted_lifetimes(), 2);
    assert_eq!(runtime.status(), RuntimeStatus::Closed);
    assert_eq!(
        log.borrow()
            .iter()
            .filter(|fact| **fact == Fact::Unmounted)
            .count(),
        1
    );
    assert!(
        !log.borrow()
            .iter()
            .any(|fact| matches!(fact, Fact::Cancel(_)))
    );

    let shutdown = find_record(&runtime, |record| {
        matches!(record.kind(), TraceRecordKind::RuntimeShutdown { .. })
    });
    assert!(ancestry_contains_sequence(
        &runtime,
        shutdown.causal_parent(),
        terminal_sequence
    ));
}
