#![allow(refining_impl_trait)]

use std::{
    cell::RefCell,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use runenui_core::{
    Axis, ChildLayout, ChildLayoutWidget, CommandDerivation, CommandOrigin, Element, EventContext,
    EventPhase, NoHostProtocol, SemanticCommand, UiApp, UiEvent, View, Widget, WidgetActivation,
    WidgetActivationContext, WidgetActivationOutput, WidgetEventOutput, WidgetInvalidation,
    children, container,
};
use runenui_runtime::{
    AppRuntime, PumpBudget, RuntimeConfig, RuntimeLimits, RuntimeStatus, RuntimeTerminalReason,
    SubmitCommandErrorKind, TraceConfig, TraceRecordKind, TraceRoutedAdmissionRejection,
    TraceRoutedIntegrityFailure, TraceTargetRejection,
};

#[derive(Debug)]
struct Action(&'static str);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Behavior {
    Observe,
    Emit,
    EmitAndStop(EventPhase),
    EmitAndPrevent,
    DisableAtTarget,
    DelegateAtTarget,
    EmitTwiceAtTarget,
}

#[derive(Debug)]
struct Probe {
    name: &'static str,
    behavior: Behavior,
    log: Rc<RefCell<Vec<String>>>,
    actionable: bool,
}

impl Widget<Action> for Probe {
    type State = bool;

    fn create_state(&self) -> Self::State {
        true
    }

    fn event(
        &mut self,
        state: &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, Action>,
    ) -> WidgetEventOutput {
        self.log
            .borrow_mut()
            .push(format!("{}:{:?}", self.name, context.phase()));
        match self.behavior {
            Behavior::Emit => {
                context.emit(Action(self.name));
                WidgetEventOutput::none()
            }
            Behavior::EmitAndStop(phase) if phase == context.phase() => {
                context.emit(Action(self.name));
                context.stop_propagation();
                WidgetEventOutput::none()
            }
            Behavior::EmitAndPrevent => {
                context.emit(Action(self.name));
                context.prevent_default();
                WidgetEventOutput::none()
            }
            Behavior::DisableAtTarget if context.phase() == EventPhase::Target => {
                *state = false;
                context.invalidate(WidgetInvalidation::INTERACTION);
                WidgetEventOutput::changed()
            }
            Behavior::DelegateAtTarget
                if context.phase() == EventPhase::Target
                    && event.as_semantic_command().is_some_and(|command| {
                        command.origin().derivation() == CommandDerivation::Direct
                    }) =>
            {
                context.emit_command(SemanticCommand::OpenMenu);
                WidgetEventOutput::none()
            }
            Behavior::EmitTwiceAtTarget if context.phase() == EventPhase::Target => {
                context.emit(Action("first"));
                context.emit(Action("second"));
                WidgetEventOutput::none()
            }
            Behavior::Observe
            | Behavior::EmitAndStop(_)
            | Behavior::DisableAtTarget
            | Behavior::DelegateAtTarget
            | Behavior::EmitTwiceAtTarget => WidgetEventOutput::none(),
        }
    }

    fn activation(&self, state: &Self::State) -> WidgetActivation {
        if self.actionable {
            WidgetActivation::actionable(*state)
        } else {
            WidgetActivation::NONE
        }
    }

    fn activate(
        &mut self,
        _state: &mut Self::State,
        _context: &mut WidgetActivationContext<Action>,
    ) -> WidgetActivationOutput<Action> {
        self.log.borrow_mut().push(format!("{}:default", self.name));
        WidgetActivationOutput::action(Action("default"))
    }
}

impl ChildLayoutWidget<Action> for Probe {
    fn child_layout(&self, _state: &Self::State) -> ChildLayout {
        ChildLayout::Linear {
            axis: Axis::Vertical,
        }
    }
}

#[derive(Debug)]
struct State {
    log: Rc<RefCell<Vec<String>>>,
    updates: Vec<&'static str>,
    root: Behavior,
    parent: Behavior,
    target: Behavior,
}

struct App;

impl UiApp for App {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> Element<Self::Action> {
        container(
            Probe {
                name: "root",
                behavior: state.root,
                log: Rc::clone(&state.log),
                actionable: false,
            },
            children![
                container(
                    Probe {
                        name: "parent",
                        behavior: state.parent,
                        log: Rc::clone(&state.log),
                        actionable: false,
                    },
                    children![
                        Element::new(Probe {
                            name: "target",
                            behavior: state.target,
                            log: Rc::clone(&state.log),
                            actionable: true,
                        })
                        .id("target")
                        .key("target")
                    ],
                )
                .key("parent")
            ],
        )
        .key("root")
        .into_element()
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        state.updates.push(action.0);
    }
}

#[derive(Debug)]
struct SingleState {
    log: Rc<RefCell<Vec<String>>>,
}

struct SingleApp;

impl UiApp for SingleApp {
    type State = SingleState;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> Element<Self::Action> {
        Element::new(Probe {
            name: "target",
            behavior: Behavior::Observe,
            log: Rc::clone(&state.log),
            actionable: true,
        })
        .id("target")
        .key("target")
    }

    fn update(_state: &mut Self::State, _action: Self::Action) {}
}

fn make_single(config: RuntimeConfig) -> AppRuntime<SingleApp> {
    AppRuntime::mount_with_config(
        SingleState {
            log: Rc::new(RefCell::new(Vec::new())),
        },
        config,
    )
}

fn settle_single(runtime: &mut AppRuntime<SingleApp>) {
    runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
}

fn submit_single(runtime: &mut AppRuntime<SingleApp>) {
    let target = runtime.index().nodes()[0].id().clone();
    runtime
        .submit_command(
            target,
            SemanticCommand::Activate,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("the exact live target is accepted"));
    runtime.pump(PumpBudget::new(1, 0, 0, 0));
}

fn make_runtime(root: Behavior, parent: Behavior, target: Behavior) -> AppRuntime<App> {
    make_runtime_with_config(root, parent, target, RuntimeConfig::default())
}

fn make_runtime_with_config(
    root: Behavior,
    parent: Behavior,
    target: Behavior,
    config: RuntimeConfig,
) -> AppRuntime<App> {
    AppRuntime::mount_with_config(
        State {
            log: Rc::new(RefCell::new(Vec::new())),
            updates: Vec::new(),
            root,
            parent,
            target,
        },
        config,
    )
}

fn settle(runtime: &mut AppRuntime<App>) {
    runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
}

fn target(runtime: &mut AppRuntime<App>) -> runenui_core::MountedNodeId {
    let authored_id = runenui_core::ElementId::new("target")
        .unwrap_or_else(|_| unreachable!("the test identifier is valid"));
    runtime
        .index()
        .node_by_authored_id(&authored_id)
        .unwrap_or_else(|| unreachable!("the routed target is mounted"))
        .id()
        .clone()
}

fn submit_and_route(runtime: &mut AppRuntime<App>, command: SemanticCommand) {
    let target = target(runtime);
    runtime
        .submit_command(target, command, CommandOrigin::programmatic())
        .unwrap_or_else(|_| unreachable!("the exact live target is accepted"));
    let report = runtime.pump(PumpBudget::new(1, 0, 0, 0));
    assert_eq!(report.processed_envelopes(), 1);
}

#[test]
fn immutable_route_is_capture_target_bubble_then_default() {
    let mut runtime = make_runtime(Behavior::Observe, Behavior::Observe, Behavior::Observe);
    settle(&mut runtime);
    runtime.state().log.borrow_mut().clear();
    submit_and_route(&mut runtime, SemanticCommand::Activate);
    assert_eq!(
        runtime.state().log.borrow().as_slice(),
        [
            "root:Capture",
            "parent:Capture",
            "target:Target",
            "parent:Bubble",
            "root:Bubble",
            "target:default",
        ]
    );
}

#[test]
fn stop_propagation_preserves_earlier_and_current_output_and_keeps_default_eligible() {
    let mut runtime = make_runtime(
        Behavior::Emit,
        Behavior::EmitAndStop(EventPhase::Capture),
        Behavior::Observe,
    );
    settle(&mut runtime);
    let parent = runtime.index().nodes()[1].id().clone();
    runtime.state().log.borrow_mut().clear();
    submit_and_route(&mut runtime, SemanticCommand::Activate);
    assert_eq!(
        runtime.state().log.borrow().as_slice(),
        ["root:Capture", "parent:Capture", "target:default"]
    );
    settle(&mut runtime);
    assert_eq!(runtime.state().updates, ["root", "parent", "default"]);
    let stopped = runtime
        .trace()
        .records()
        .find(|record| matches!(record.kind(), TraceRecordKind::PropagationStopped))
        .unwrap_or_else(|| unreachable!("stopped propagation is traced"));
    assert_eq!(stopped.current_target(), Some(&parent));
}

#[test]
fn prevent_default_preserves_the_complete_route_and_routed_output() {
    let mut runtime = make_runtime(
        Behavior::Observe,
        Behavior::Observe,
        Behavior::EmitAndPrevent,
    );
    settle(&mut runtime);
    runtime.state().log.borrow_mut().clear();
    submit_and_route(&mut runtime, SemanticCommand::Activate);
    assert_eq!(
        runtime.state().log.borrow().as_slice(),
        [
            "root:Capture",
            "parent:Capture",
            "target:Target",
            "parent:Bubble",
            "root:Bubble",
        ]
    );
    settle(&mut runtime);
    assert_eq!(runtime.state().updates, ["target"]);
    assert!(
        runtime
            .trace()
            .kinds()
            .any(|kind| matches!(kind, TraceRecordKind::SemanticDefaultSuppressed { .. }))
    );
}

#[test]
fn callback_invalidation_requeries_the_mutated_activation_capability() {
    let mut runtime = make_runtime(
        Behavior::Observe,
        Behavior::Observe,
        Behavior::DisableAtTarget,
    );
    settle(&mut runtime);
    runtime.state().log.borrow_mut().clear();
    submit_and_route(&mut runtime, SemanticCommand::Activate);
    assert!(
        !runtime
            .state()
            .log
            .borrow()
            .iter()
            .any(|entry| entry == "target:default")
    );
    assert!(
        runtime
            .trace()
            .kinds()
            .any(|kind| matches!(kind, TraceRecordKind::WidgetStateMutated))
    );
}

#[test]
fn routed_non_clone_actions_preserve_callback_order_before_default() {
    let mut runtime = make_runtime(Behavior::Emit, Behavior::Emit, Behavior::Emit);
    settle(&mut runtime);
    submit_and_route(&mut runtime, SemanticCommand::Activate);
    runtime.pump(PumpBudget::new(usize::MAX, 0, 0, 0));
    assert_eq!(
        runtime.state().updates,
        ["root", "parent", "target", "parent", "root", "default"]
    );
}

#[test]
fn delegated_command_targets_current_node_and_runs_later_without_recursion() {
    let mut runtime = make_runtime(
        Behavior::Observe,
        Behavior::Observe,
        Behavior::DelegateAtTarget,
    );
    settle(&mut runtime);
    let target_id = target(&mut runtime);
    let direct = runtime
        .submit_command(
            target_id.clone(),
            SemanticCommand::Activate,
            CommandOrigin::automation(),
        )
        .unwrap_or_else(|_| unreachable!("the exact live target is accepted"));
    runtime.pump(PumpBudget::new(1, 0, 0, 0));
    let accepted: Vec<_> = runtime
        .trace()
        .records()
        .filter(|record| matches!(record.kind(), TraceRecordKind::CommandSubmissionAccepted))
        .map(|record| (record.work_sequence(), record.original_target().cloned()))
        .collect();
    assert_eq!(accepted.len(), 2);
    assert!(
        accepted[1]
            .0
            .is_some_and(|sequence| sequence > direct.sequence())
    );
    assert_eq!(accepted[1].1.as_ref(), Some(&target_id));
    assert_eq!(runtime.state().log.borrow().len(), 6);

    runtime.pump(PumpBudget::new(1, 0, 0, 0));
    assert_eq!(runtime.state().log.borrow().len(), 11);
}

#[test]
fn route_only_commands_have_no_default_action() {
    for command in [
        SemanticCommand::CancelOrBack,
        SemanticCommand::OpenMenu,
        SemanticCommand::OpenContextMenu,
    ] {
        let mut runtime = make_runtime(Behavior::Observe, Behavior::Observe, Behavior::Observe);
        settle(&mut runtime);
        runtime.state().log.borrow_mut().clear();
        submit_and_route(&mut runtime, command);
        runtime.pump(PumpBudget::new(usize::MAX, 0, 0, 0));
        assert!(runtime.state().updates.is_empty());
        assert!(
            !runtime
                .state()
                .log
                .borrow()
                .iter()
                .any(|entry| entry.ends_with(":default"))
        );
    }
}

#[test]
fn all_direct_sources_converge_on_the_same_activate_transaction() {
    let mut runtime = make_runtime(Behavior::Observe, Behavior::Observe, Behavior::Observe);
    settle(&mut runtime);
    let target = target(&mut runtime);
    let origins = [
        CommandOrigin::programmatic(),
        CommandOrigin::automation(),
        CommandOrigin::accessibility(),
        CommandOrigin::controller(),
    ];
    for origin in origins {
        runtime
            .submit_command(target.clone(), SemanticCommand::Activate, origin)
            .unwrap_or_else(|_| unreachable!("the exact live target is accepted"));
    }
    settle(&mut runtime);
    assert_eq!(runtime.state().updates, ["default"; 4]);
    let routed_origins: Vec<_> = runtime
        .trace()
        .records()
        .filter(|record| matches!(record.kind(), TraceRecordKind::RoutedEventStarted))
        .map(runenui_runtime::TraceRecord::command_origin)
        .collect();
    assert_eq!(routed_origins, origins.map(Some));
}

#[test]
fn submission_recovers_exact_foreign_target_inputs() {
    let mut runtime = make_runtime(Behavior::Observe, Behavior::Observe, Behavior::Observe);
    let mut foreign = make_runtime(Behavior::Observe, Behavior::Observe, Behavior::Observe);
    settle(&mut runtime);
    settle(&mut foreign);
    let foreign_target = target(&mut foreign);
    let Err(error) = runtime.submit_command(
        foreign_target.clone(),
        SemanticCommand::OpenMenu,
        CommandOrigin::controller(),
    ) else {
        unreachable!("a target from another runtime is foreign")
    };
    assert_eq!(error.kind(), SubmitCommandErrorKind::ForeignTarget);
    let (recovered, command, origin) = error.into_unaccepted().into_parts();
    assert_eq!(recovered, foreign_target);
    assert_eq!(command, SemanticCommand::OpenMenu);
    assert_eq!(origin, CommandOrigin::controller());
    assert!(!runtime.trace().kinds().any(|kind| matches!(
        kind,
        TraceRecordKind::CommandSubmissionAccepted
            | TraceRecordKind::CommandProcessingRejected { .. }
    )));
}

#[cfg(feature = "internal-test-seams")]
fn assert_ordinary_submission_rejection_is_inert(
    runtime: &mut AppRuntime<App>,
    rejected_target: &runenui_core::MountedNodeId,
    expected: SubmitCommandErrorKind,
) {
    let wake_calls = Arc::new(AtomicUsize::new(0));
    let wake_calls_for_transport = Arc::clone(&wake_calls);
    runtime.set_wake_transport(move || {
        wake_calls_for_transport.fetch_add(1, Ordering::SeqCst);
    });
    let wake_before = wake_calls.load(Ordering::SeqCst);
    let sequence_before = runtime.__routed_sequence_state_for_test();
    let trace_len_before = runtime.trace().len();
    let status_before = runtime.status();
    let reservations_before = runtime.__routed_trace_reservations_for_test();
    let callback_len_before = runtime.state().log.borrow().len();
    let updates_before = runtime.state().updates.len();
    let command = SemanticCommand::OpenContextMenu;
    let origin = CommandOrigin::controller();

    let Err(error) = runtime.submit_command(rejected_target.clone(), command, origin) else {
        unreachable!("the selected submission class must reject")
    };
    assert_eq!(error.kind(), expected);
    let (recovered_target, recovered_command, recovered_origin) =
        error.into_unaccepted().into_parts();
    assert_eq!(&recovered_target, rejected_target);
    assert_eq!(recovered_command, command);
    assert_eq!(recovered_origin, origin);
    assert_eq!(runtime.__routed_sequence_state_for_test(), sequence_before);
    assert_eq!(runtime.trace().len(), trace_len_before);
    assert_eq!(runtime.status(), status_before);
    assert_eq!(
        runtime.__routed_trace_reservations_for_test(),
        reservations_before
    );
    assert_eq!(runtime.state().log.borrow().len(), callback_len_before);
    assert_eq!(runtime.state().updates.len(), updates_before);
    assert_eq!(wake_calls.load(Ordering::SeqCst), wake_before);
}

#[cfg(feature = "internal-test-seams")]
#[test]
fn six_ordinary_submission_rejections_recover_inputs_and_consume_no_authority() {
    let mut full = make_runtime_with_config(
        Behavior::Observe,
        Behavior::Observe,
        Behavior::Observe,
        RuntimeConfig::default()
            .with_queue_capacity(4)
            .with_trace_config(TraceConfig::new(4096)),
    );
    settle(&mut full);
    let full_target = target(&mut full);
    for _ in 0..4 {
        full.submit_command(
            full_target.clone(),
            SemanticCommand::OpenMenu,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("the configured queue slot is available"));
    }
    assert_ordinary_submission_rejection_is_inert(
        &mut full,
        &full_target,
        SubmitCommandErrorKind::Full,
    );

    let mut closed = make_runtime(Behavior::Observe, Behavior::Observe, Behavior::Observe);
    settle(&mut closed);
    let closed_target = target(&mut closed);
    closed.shutdown();
    assert_ordinary_submission_rejection_is_inert(
        &mut closed,
        &closed_target,
        SubmitCommandErrorKind::Closed,
    );

    let mut terminal = make_runtime(Behavior::Observe, Behavior::Observe, Behavior::Observe);
    settle(&mut terminal);
    let terminal_target = target(&mut terminal);
    terminal.__seed_next_work_sequence_for_test(0);
    let _ = terminal.submit_action(Action("terminal"));
    assert_ordinary_submission_rejection_is_inert(
        &mut terminal,
        &terminal_target,
        SubmitCommandErrorKind::Terminal(RuntimeTerminalReason::WorkSequenceExhausted),
    );

    let mut foreign = make_runtime(Behavior::Observe, Behavior::Observe, Behavior::Observe);
    let mut foreign_owner = make_runtime(Behavior::Observe, Behavior::Observe, Behavior::Observe);
    settle(&mut foreign);
    settle(&mut foreign_owner);
    let foreign_target = target(&mut foreign_owner);
    assert_ordinary_submission_rejection_is_inert(
        &mut foreign,
        &foreign_target,
        SubmitCommandErrorKind::ForeignTarget,
    );

    let mut stale = make_runtime(Behavior::Observe, Behavior::Observe, Behavior::Observe);
    settle(&mut stale);
    let live = target(&mut stale);
    let stale_target = stale.__stale_target_for_test(&live);
    assert_ordinary_submission_rejection_is_inert(
        &mut stale,
        &stale_target,
        SubmitCommandErrorKind::StaleTarget,
    );

    let mut missing = make_runtime(Behavior::Observe, Behavior::Observe, Behavior::Observe);
    settle(&mut missing);
    let missing_target = missing.__missing_target_for_test();
    assert_ordinary_submission_rejection_is_inert(
        &mut missing,
        &missing_target,
        SubmitCommandErrorKind::MissingTarget,
    );
}

#[cfg(feature = "internal-test-seams")]
fn assert_exhausted_submission_is_terminal(
    runtime: &mut AppRuntime<App>,
    target: &runenui_core::MountedNodeId,
    expected: SubmitCommandErrorKind,
    reason: RuntimeTerminalReason,
) {
    let callback_len_before = runtime.state().log.borrow().len();
    let updates_before = runtime.state().updates.len();
    let command = SemanticCommand::OpenContextMenu;
    let origin = CommandOrigin::controller();
    let Err(error) = runtime.submit_command(target.clone(), command, origin) else {
        unreachable!("exhausted authority must reject the command")
    };
    assert_eq!(error.kind(), expected);
    let (recovered_target, recovered_command, recovered_origin) =
        error.into_unaccepted().into_parts();
    assert_eq!(&recovered_target, target);
    assert_eq!(recovered_command, command);
    assert_eq!(recovered_origin, origin);
    assert_eq!(runtime.status(), RuntimeStatus::Terminal(reason));
    assert_eq!(runtime.__routed_trace_reservations_for_test(), 0);
    assert_eq!(runtime.state().log.borrow().len(), callback_len_before);
    assert_eq!(runtime.state().updates.len(), updates_before);
}

#[cfg(feature = "internal-test-seams")]
#[test]
fn work_and_trace_sequence_exhaustion_recover_inputs_and_enter_terminal_state() {
    let mut work_exhausted = make_runtime(Behavior::Observe, Behavior::Observe, Behavior::Observe);
    settle(&mut work_exhausted);
    let work_target = target(&mut work_exhausted);
    work_exhausted.__seed_next_work_sequence_for_test(0);
    assert_exhausted_submission_is_terminal(
        &mut work_exhausted,
        &work_target,
        SubmitCommandErrorKind::WorkSequenceExhausted,
        RuntimeTerminalReason::WorkSequenceExhausted,
    );

    let mut trace_exhausted = make_runtime(Behavior::Observe, Behavior::Observe, Behavior::Observe);
    settle(&mut trace_exhausted);
    let trace_target = target(&mut trace_exhausted);
    trace_exhausted.__seed_next_trace_sequence_for_test(0);
    assert_exhausted_submission_is_terminal(
        &mut trace_exhausted,
        &trace_target,
        SubmitCommandErrorKind::TraceSequenceExhausted,
        RuntimeTerminalReason::TraceSequenceExhausted,
    );
}

#[cfg(feature = "internal-test-seams")]
#[test]
fn ordinary_rejection_preserves_the_final_two_sequences_for_a_valid_command() {
    let mut runtime = make_runtime(Behavior::Observe, Behavior::Observe, Behavior::Observe);
    let mut foreign_owner = make_runtime(Behavior::Observe, Behavior::Observe, Behavior::Observe);
    settle(&mut runtime);
    settle(&mut foreign_owner);
    let valid_target = target(&mut runtime);
    let foreign_target = target(&mut foreign_owner);
    runtime.__seed_next_trace_sequence_for_test(u64::MAX - 1);

    assert_ordinary_submission_rejection_is_inert(
        &mut runtime,
        &foreign_target,
        SubmitCommandErrorKind::ForeignTarget,
    );

    let submission = runtime
        .submit_command(
            valid_target,
            SemanticCommand::OpenMenu,
            CommandOrigin::automation(),
        )
        .unwrap_or_else(|_| {
            unreachable!("ordinary rejection preserved acceptance and outcome authority")
        });
    let acceptance = runtime
        .trace()
        .records()
        .find(|record| {
            matches!(record.kind(), TraceRecordKind::CommandSubmissionAccepted)
                && record.work_sequence() == Some(submission.sequence())
        })
        .unwrap_or_else(|| unreachable!("the valid command has one acceptance record"));
    assert_eq!(acceptance.sequence().get(), u64::MAX - 1);
    assert_eq!(runtime.__routed_sequence_state_for_test().1, Some(u64::MAX));
    assert_eq!(runtime.__routed_trace_reservations_for_test(), 1);
    runtime.shutdown();
    assert_eq!(runtime.__routed_trace_reservations_for_test(), 0);
}

#[cfg(feature = "internal-test-seams")]
#[test]
fn final_trace_sequence_cannot_accept_a_command_without_outcome_authority() {
    let mut runtime = make_runtime(Behavior::Observe, Behavior::Observe, Behavior::Observe);
    settle(&mut runtime);
    let target = target(&mut runtime);
    runtime.__seed_next_trace_sequence_for_test(u64::MAX);
    assert_exhausted_submission_is_terminal(
        &mut runtime,
        &target,
        SubmitCommandErrorKind::TraceSequenceExhausted,
        RuntimeTerminalReason::TraceSequenceExhausted,
    );
    assert!(
        !runtime
            .trace()
            .kinds()
            .any(|kind| matches!(kind, TraceRecordKind::CommandSubmissionAccepted))
    );
}

#[cfg(feature = "internal-test-seams")]
#[test]
fn accepted_command_consumes_its_reserved_final_sequence_for_admission_rejection() {
    let mut runtime = make_runtime(Behavior::Observe, Behavior::Observe, Behavior::Observe);
    settle(&mut runtime);
    runtime.state().log.borrow_mut().clear();
    let target = target(&mut runtime);
    runtime.__seed_next_trace_sequence_for_test(u64::MAX - 1);
    let submission = runtime
        .submit_command(
            target,
            SemanticCommand::OpenMenu,
            CommandOrigin::automation(),
        )
        .unwrap_or_else(|_| unreachable!("two trace sequences admit one command"));
    assert_eq!(runtime.__routed_trace_reservations_for_test(), 1);
    runtime.pump(PumpBudget::new(1, 0, 0, 0));
    assert!(runtime.state().log.borrow().is_empty());
    assert_eq!(runtime.__routed_trace_reservations_for_test(), 0);
    assert_eq!(
        runtime.status(),
        RuntimeStatus::Terminal(RuntimeTerminalReason::TraceSequenceExhausted)
    );
    let acceptance = runtime
        .trace()
        .records()
        .find(|record| {
            matches!(record.kind(), TraceRecordKind::CommandSubmissionAccepted)
                && record.work_sequence() == Some(submission.sequence())
        })
        .unwrap_or_else(|| unreachable!("command acceptance is retained"));
    let rejection = runtime
        .trace()
        .records()
        .find(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::RoutedEventAdmissionRejected {
                    capacity: TraceRoutedAdmissionRejection::TraceSequenceExhausted
                }
            )
        })
        .unwrap_or_else(|| unreachable!("reserved processing rejection is retained"));
    assert_eq!(acceptance.sequence().get(), u64::MAX - 1);
    assert_eq!(rejection.sequence().get(), u64::MAX);
    assert_eq!(rejection.causal_parent(), Some(acceptance.sequence()));
}

#[cfg(feature = "internal-test-seams")]
#[test]
fn same_runtime_missing_target_is_distinct_and_exactly_recoverable() {
    let mut runtime = make_runtime(Behavior::Observe, Behavior::Observe, Behavior::Observe);
    settle(&mut runtime);
    let missing = runtime.__missing_target_for_test();
    let Err(error) = runtime.submit_command(
        missing.clone(),
        SemanticCommand::OpenContextMenu,
        CommandOrigin::programmatic(),
    ) else {
        unreachable!("the internal missing-target seam is rejected")
    };
    assert_eq!(error.kind(), SubmitCommandErrorKind::MissingTarget);
    assert_eq!(error.into_unaccepted().into_parts().0, missing);
}

#[cfg(feature = "internal-test-seams")]
#[test]
fn route_wide_bridge_mismatch_invokes_no_callback() {
    let mut runtime = make_runtime(Behavior::Observe, Behavior::Observe, Behavior::Observe);
    settle(&mut runtime);
    let target = target(&mut runtime);
    runtime.__corrupt_widget_state_for_test(&target);
    runtime
        .submit_command(
            target.clone(),
            SemanticCommand::Activate,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("the exact live target is accepted"));
    runtime.pump(PumpBudget::new(1, 0, 0, 0));
    assert!(runtime.state().log.borrow().is_empty());
    assert_eq!(
        runtime.status(),
        RuntimeStatus::Terminal(RuntimeTerminalReason::Poisoned)
    );
    assert_routed_integrity_failure(
        &runtime,
        &target,
        TraceRoutedIntegrityFailure::EventBridgeMismatch,
        None,
    );
}

#[cfg(feature = "internal-test-seams")]
fn assert_routed_integrity_failure(
    runtime: &AppRuntime<App>,
    target: &runenui_core::MountedNodeId,
    expected: TraceRoutedIntegrityFailure,
    expected_current: Option<&runenui_core::MountedNodeId>,
) {
    let failure = runtime
        .trace()
        .records()
        .find(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::RoutedIntegrityFailed { failure } if *failure == expected
            )
        })
        .unwrap_or_else(|| unreachable!("the exact routed integrity failure is traced"));
    assert!(failure.work_sequence().is_some());
    assert!(failure.causal_parent().is_some());
    assert!(failure.instant().is_some());
    assert_eq!(failure.original_target(), Some(target));
    assert_eq!(failure.current_target(), expected_current);
    assert_eq!(
        failure.command_origin(),
        Some(CommandOrigin::programmatic())
    );
}

#[cfg(feature = "internal-test-seams")]
#[test]
fn routed_integrity_trace_distinguishes_broken_topology() {
    let mut runtime = make_runtime(Behavior::Observe, Behavior::Observe, Behavior::Observe);
    settle(&mut runtime);
    let target = target(&mut runtime);
    runtime.__break_routed_topology_for_test(&target);
    runtime
        .submit_command(
            target.clone(),
            SemanticCommand::Activate,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("the target is live at submission"));
    runtime.pump(PumpBudget::new(1, 0, 0, 0));
    assert!(runtime.state().log.borrow().is_empty());
    assert_routed_integrity_failure(
        &runtime,
        &target,
        TraceRoutedIntegrityFailure::BrokenTopology,
        None,
    );
}

#[cfg(feature = "internal-test-seams")]
#[test]
fn routed_integrity_trace_distinguishes_callback_bridge_failure() {
    let mut runtime = make_runtime(Behavior::Observe, Behavior::Observe, Behavior::Observe);
    settle(&mut runtime);
    let root = runtime.index().nodes()[0].id().clone();
    let target = target(&mut runtime);
    runtime.__fail_routed_callback_bridge_for_test();
    submit_and_route(&mut runtime, SemanticCommand::Activate);
    assert_routed_integrity_failure(
        &runtime,
        &target,
        TraceRoutedIntegrityFailure::CallbackBridgeFailure,
        Some(&root),
    );
}

#[cfg(feature = "internal-test-seams")]
#[test]
fn routed_integrity_trace_distinguishes_output_allowance_exceeded() {
    let limits = RuntimeLimits::default().with_transaction_outputs(1);
    let mut runtime = make_runtime_with_config(
        Behavior::Observe,
        Behavior::Observe,
        Behavior::EmitTwiceAtTarget,
        RuntimeConfig::default().with_limits(limits),
    );
    settle(&mut runtime);
    let target = target(&mut runtime);
    submit_and_route(&mut runtime, SemanticCommand::Activate);
    assert_routed_integrity_failure(
        &runtime,
        &target,
        TraceRoutedIntegrityFailure::OutputAllowanceExceeded,
        Some(&target),
    );
}

#[cfg(feature = "internal-test-seams")]
#[test]
fn routed_integrity_trace_distinguishes_semantic_default_failure() {
    let mut runtime = make_runtime(Behavior::Observe, Behavior::Observe, Behavior::Observe);
    settle(&mut runtime);
    let target = target(&mut runtime);
    runtime.state().log.borrow_mut().clear();
    runtime.__fail_routed_semantic_default_for_test();
    submit_and_route(&mut runtime, SemanticCommand::Activate);
    assert_eq!(
        runtime.state().log.borrow().as_slice(),
        [
            "root:Capture",
            "parent:Capture",
            "target:Target",
            "parent:Bubble",
            "root:Bubble",
        ]
    );
    assert!(runtime.state().updates.is_empty());
    assert_routed_integrity_failure(
        &runtime,
        &target,
        TraceRoutedIntegrityFailure::SemanticDefaultFailure,
        Some(&target),
    );
}

#[cfg(feature = "internal-test-seams")]
#[test]
fn routed_integrity_trace_distinguishes_commit_invariant_failure_without_partial_output() {
    let mut runtime = make_runtime(Behavior::Observe, Behavior::Observe, Behavior::Emit);
    settle(&mut runtime);
    let target = target(&mut runtime);
    runtime.__fail_routed_commit_for_test();
    submit_and_route(&mut runtime, SemanticCommand::Activate);
    assert!(runtime.state().updates.is_empty());
    assert_routed_integrity_failure(
        &runtime,
        &target,
        TraceRoutedIntegrityFailure::CommitInvariantFailure,
        None,
    );
}

#[test]
fn routed_queue_admission_has_an_exact_required_boundary() {
    let base = RuntimeLimits::default().with_transaction_outputs(1);
    let mut exact =
        make_single(RuntimeConfig::default().with_limits(base.with_waiting_envelopes(2)));
    settle_single(&mut exact);
    exact.state().log.borrow_mut().clear();
    submit_single(&mut exact);
    assert_eq!(exact.state().log.borrow().len(), 2);

    let mut short =
        make_single(RuntimeConfig::default().with_limits(base.with_waiting_envelopes(1)));
    settle_single(&mut short);
    short.state().log.borrow_mut().clear();
    submit_single(&mut short);
    assert!(short.state().log.borrow().is_empty());
    assert!(short.trace().kinds().any(|kind| matches!(
        kind,
        TraceRecordKind::RoutedEventAdmissionRejected {
            capacity: TraceRoutedAdmissionRejection::WaitingEnvelopes
        }
    )));
}

#[test]
fn routed_output_allowance_rejects_zero_before_the_first_callback() {
    let limits = RuntimeLimits::default().with_transaction_outputs(0);
    let mut runtime = make_single(RuntimeConfig::default().with_limits(limits));
    settle_single(&mut runtime);
    runtime.state().log.borrow_mut().clear();
    submit_single(&mut runtime);
    assert!(runtime.state().log.borrow().is_empty());
    assert_eq!(runtime.status(), RuntimeStatus::Running);
    assert!(runtime.trace().kinds().any(|kind| matches!(
        kind,
        TraceRecordKind::RoutedEventAdmissionRejected {
            capacity: TraceRoutedAdmissionRejection::TransactionOutputs
        }
    )));
}

#[test]
fn routed_admission_rejects_checked_arithmetic_overflow_before_the_first_callback() {
    let limits = RuntimeLimits::default().with_transaction_outputs(usize::MAX);
    let mut runtime = make_single(RuntimeConfig::default().with_limits(limits));
    settle_single(&mut runtime);
    runtime.state().log.borrow_mut().clear();
    submit_single(&mut runtime);
    assert!(runtime.state().log.borrow().is_empty());
    assert_eq!(
        runtime.status(),
        RuntimeStatus::Terminal(RuntimeTerminalReason::Poisoned)
    );
    assert!(runtime.trace().kinds().any(|kind| matches!(
        kind,
        TraceRecordKind::RoutedEventAdmissionRejected {
            capacity: TraceRoutedAdmissionRejection::CheckedArithmeticOverflow
        }
    )));
}

#[test]
fn trace_capacity_zero_preserves_routed_behavior() {
    let mut traced = make_single(RuntimeConfig::default());
    let mut untraced = make_single(RuntimeConfig::default().with_trace_config(TraceConfig::new(0)));
    settle_single(&mut traced);
    settle_single(&mut untraced);
    traced.state().log.borrow_mut().clear();
    untraced.state().log.borrow_mut().clear();
    submit_single(&mut traced);
    submit_single(&mut untraced);
    assert_eq!(
        traced.state().log.borrow().as_slice(),
        untraced.state().log.borrow().as_slice()
    );
    assert_eq!(traced.status(), untraced.status());
    assert_eq!(untraced.trace().records().count(), 0);
}

#[cfg(feature = "internal-test-seams")]
#[test]
fn every_routed_bounded_authority_rejects_before_the_first_callback() {
    let base = RuntimeLimits::default()
        .with_waiting_envelopes(2)
        .with_transaction_outputs(1);
    for limits in [
        base.with_local_tasks(0),
        base.with_send_tasks(0),
        base.with_timers(0),
    ] {
        let mut runtime = make_single(RuntimeConfig::default().with_limits(limits));
        settle_single(&mut runtime);
        runtime.state().log.borrow_mut().clear();
        submit_single(&mut runtime);
        assert!(runtime.state().log.borrow().is_empty());
    }

    let mut generation = make_single(RuntimeConfig::default().with_limits(base));
    settle_single(&mut generation);
    generation.state().log.borrow_mut().clear();
    generation.__seed_next_work_generation_for_test(0);
    submit_single(&mut generation);
    assert!(generation.state().log.borrow().is_empty());
    assert_eq!(
        generation.status(),
        RuntimeStatus::Terminal(RuntimeTerminalReason::WorkGenerationExhausted)
    );

    let mut reconciliation = make_single(RuntimeConfig::default().with_limits(base));
    settle_single(&mut reconciliation);
    reconciliation.state().log.borrow_mut().clear();
    reconciliation.__seed_reconciliation_generation_for_test(u64::MAX);
    submit_single(&mut reconciliation);
    assert!(reconciliation.state().log.borrow().is_empty());
    assert_eq!(
        reconciliation.status(),
        RuntimeStatus::Terminal(RuntimeTerminalReason::ReconciliationGenerationExhausted)
    );

    let mut trace = make_single(RuntimeConfig::default().with_limits(base));
    settle_single(&mut trace);
    trace.state().log.borrow_mut().clear();
    trace.__seed_next_trace_sequence_for_test(u64::MAX - 1);
    submit_single(&mut trace);
    assert!(trace.state().log.borrow().is_empty());
    assert_eq!(
        trace.status(),
        RuntimeStatus::Terminal(RuntimeTerminalReason::TraceSequenceExhausted)
    );

    let mut sequence = make_single(RuntimeConfig::default().with_limits(base));
    settle_single(&mut sequence);
    sequence.__seed_next_work_sequence_for_test(u64::MAX);
    sequence.state().log.borrow_mut().clear();
    submit_single(&mut sequence);
    assert!(sequence.state().log.borrow().is_empty());
    assert_eq!(
        sequence.status(),
        RuntimeStatus::Terminal(RuntimeTerminalReason::WorkSequenceExhausted)
    );
}

#[test]
fn no_output_callback_is_rejected_when_conservative_family_reservation_is_unavailable() {
    let limits = RuntimeLimits::default()
        .with_waiting_envelopes(2)
        .with_transaction_outputs(1)
        .with_local_tasks(0);
    let mut runtime = make_single(RuntimeConfig::default().with_limits(limits));
    settle_single(&mut runtime);
    runtime.state().log.borrow_mut().clear();
    submit_single(&mut runtime);
    assert!(runtime.state().log.borrow().is_empty());
    assert!(runtime.trace().kinds().any(|kind| matches!(
        kind,
        TraceRecordKind::RoutedEventAdmissionRejected {
            capacity: TraceRoutedAdmissionRejection::LocalTasks
        }
    )));
}

#[test]
fn routed_trace_links_acceptance_route_default_commit_and_later_action() {
    let mut runtime = make_single(RuntimeConfig::default());
    settle_single(&mut runtime);
    let target = runtime.index().nodes()[0].id().clone();
    let submission = runtime
        .submit_command(
            target,
            SemanticCommand::Activate,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("the exact live target is accepted"));
    runtime.pump(PumpBudget::new(2, 0, 0, 0));

    let records: Vec<_> = runtime.trace().records().collect();
    let acceptance = records
        .iter()
        .find(|record| {
            matches!(record.kind(), TraceRecordKind::CommandSubmissionAccepted)
                && record.work_sequence() == Some(submission.sequence())
        })
        .unwrap_or_else(|| unreachable!("command acceptance is traced"));
    let mut parent = acceptance.sequence();
    let mut collection = None;
    for expected in [
        "start",
        "snapshot",
        "phase",
        "modality",
        "default",
        "collected",
        "commit",
    ] {
        let record = records
            .iter()
            .find(|record| {
                record.work_sequence() == Some(submission.sequence())
                    && matches!(
                        (expected, record.kind()),
                        ("start", TraceRecordKind::RoutedEventStarted)
                            | ("snapshot", TraceRecordKind::RouteSnapshotCreated { .. })
                            | ("phase", TraceRecordKind::EventPhaseInvoked { .. })
                            | ("modality", TraceRecordKind::ModalityChanged { .. })
                            | ("default", TraceRecordKind::SemanticDefaultApplied { .. })
                            | ("collected", TraceRecordKind::RoutedActionCollected)
                            | ("commit", TraceRecordKind::RoutedEventCommitted)
                    )
            })
            .unwrap_or_else(|| unreachable!("every routed causal stage is traced"));
        assert_eq!(record.causal_parent(), Some(parent));
        parent = record.sequence();
        if expected == "collected" {
            collection = Some(parent);
        }
    }
    let collection = collection.unwrap_or_else(|| unreachable!("action collection is traced"));
    let action_acceptance = records
        .iter()
        .find(|record| {
            matches!(record.kind(), TraceRecordKind::ActionSubmissionAccepted)
                && record.causal_parent() == Some(collection)
        })
        .unwrap_or_else(|| unreachable!("the routed action acceptance links to collection"));
    let action_sequence = action_acceptance
        .work_sequence()
        .unwrap_or_else(|| unreachable!("accepted action has a work sequence"));
    let application = records
        .iter()
        .find(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::ApplicationActionTransactionStarted
            ) && record.work_sequence() == Some(action_sequence)
        })
        .unwrap_or_else(|| unreachable!("the later application transaction is traced"));
    assert_eq!(
        application.causal_parent(),
        Some(action_acceptance.sequence())
    );
}

#[test]
fn accepted_command_can_become_stale_before_processing() {
    #[derive(Debug)]
    enum ReplaceAction {
        Replace,
    }
    #[derive(Debug)]
    struct ReplaceState(bool);
    struct ReplaceApp;
    impl UiApp for ReplaceApp {
        type State = ReplaceState;
        type Action = ReplaceAction;
        type HostProtocol = NoHostProtocol;
        fn root(state: &Self::State) -> Element<Self::Action> {
            if state.0 {
                Element::new(Probe {
                    name: "replacement",
                    behavior: Behavior::Observe,
                    log: Rc::new(RefCell::new(Vec::new())),
                    actionable: false,
                })
                .map_action(|_| ReplaceAction::Replace)
                .key("replacement")
            } else {
                Element::new(Probe {
                    name: "old",
                    behavior: Behavior::Observe,
                    log: Rc::new(RefCell::new(Vec::new())),
                    actionable: false,
                })
                .map_action(|_| ReplaceAction::Replace)
                .key("old")
            }
        }
        fn update(state: &mut Self::State, _: Self::Action) {
            state.0 = true;
        }
    }
    let mut runtime = AppRuntime::<ReplaceApp>::mount(ReplaceState(false));
    runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
    let old = runtime.index().nodes()[0].id().clone();
    runtime
        .submit_action(ReplaceAction::Replace)
        .unwrap_or_else(|_| unreachable!("the replacement action is accepted"));
    let submission = runtime
        .submit_command(
            old,
            SemanticCommand::OpenMenu,
            CommandOrigin::accessibility(),
        )
        .unwrap_or_else(|_| unreachable!("the target is live at submission time"));
    runtime.pump(PumpBudget::new(2, 0, 0, 0));
    let rejected = runtime
        .trace()
        .records()
        .find_map(|record| match record.kind() {
            TraceRecordKind::CommandProcessingRejected { outcome } => Some(*outcome),
            _ => None,
        });
    assert_eq!(rejected, Some(TraceTargetRejection::Stale));
    let acceptance = runtime
        .trace()
        .records()
        .find(|record| {
            matches!(record.kind(), TraceRecordKind::CommandSubmissionAccepted)
                && record.work_sequence() == Some(submission.sequence())
        })
        .unwrap_or_else(|| unreachable!("accepted command is traced"));
    let processing = runtime
        .trace()
        .records()
        .find(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::CommandProcessingRejected { .. }
            ) && record.work_sequence() == Some(submission.sequence())
        })
        .unwrap_or_else(|| unreachable!("processing rejection is traced"));
    assert_eq!(processing.causal_parent(), Some(acceptance.sequence()));
}
