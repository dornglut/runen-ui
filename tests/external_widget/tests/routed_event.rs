#![allow(refining_impl_trait)]

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use runenui_core::{
    Axis, ChildLayout, ChildLayoutWidget, CommandDerivation, CommandOrigin, Element, EventContext,
    EventPhase, EventSource, MonotonicInstant, MountedNodeId, NoHostProtocol, SemanticCommand,
    SubscriptionSet, UiApp, UiEvent, View, Widget, WidgetActivation, WidgetActivationContext,
    WidgetActivationOutput, WidgetEventOutput, WidgetInvalidation, WorkSequence, children,
    container,
};
use runenui_runtime::{
    AppRuntime, CommandSubmission, PumpBudget, RuntimeConfig, RuntimeLimits, TraceRecordKind,
};

#[derive(Debug)]
struct ChildAction(String);

#[derive(Debug)]
enum Action {
    Child(ChildAction),
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Observation {
    phase: EventPhase,
    original: MountedNodeId,
    current: MountedNodeId,
    related: Option<MountedNodeId>,
    source: EventSource,
    derivation: CommandDerivation,
    sequence: WorkSequence,
    instant: MonotonicInstant,
    cancelable: bool,
    prevented: bool,
    stopped: bool,
    state_before: usize,
}

#[derive(Debug)]
struct EventAncestor {
    observations: Rc<RefCell<Vec<Observation>>>,
}

impl Widget<Action> for EventAncestor {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn event(
        &mut self,
        (): &mut Self::State,
        _: &UiEvent,
        context: &mut EventContext<'_, Action>,
    ) -> WidgetEventOutput {
        self.observations.borrow_mut().push(observe(context, 0));
        WidgetEventOutput::none()
    }
}

impl ChildLayoutWidget<Action> for EventAncestor {
    fn child_layout(&self, (): &Self::State) -> ChildLayout {
        ChildLayout::Linear {
            axis: Axis::Vertical,
        }
    }
}

#[derive(Debug)]
struct EventChild {
    observations: Rc<RefCell<Vec<Observation>>>,
    subscription_calls: Rc<Cell<usize>>,
}

impl Widget<ChildAction> for EventChild {
    type State = usize;

    fn create_state(&self) -> Self::State {
        0
    }

    fn event(
        &mut self,
        state: &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, ChildAction>,
    ) -> WidgetEventOutput {
        self.observations
            .borrow_mut()
            .push(observe(context, *state));
        let Some(command) = event.as_semantic_command() else {
            return WidgetEventOutput::none();
        };
        match command.command() {
            SemanticCommand::Activate
                if command.origin().derivation() == CommandDerivation::Direct =>
            {
                context.emit(ChildAction(String::from("routed-first")));
                context.emit_command(SemanticCommand::OpenMenu);
                context.emit(ChildAction(String::from("routed-second")));
                context.local_task(async { Some(ChildAction(String::from("routed-mapped-work"))) });
                context.invalidate(WidgetInvalidation::PAINT | WidgetInvalidation::SEMANTICS);
                context.invalidate_subscriptions();
                context.invalidate_subscriptions();
                WidgetEventOutput::none()
            }
            SemanticCommand::OpenMenu => {
                *state += 1;
                WidgetEventOutput::changed()
            }
            SemanticCommand::OpenContextMenu => {
                context.emit(ChildAction(format!("action-only:{state}")));
                WidgetEventOutput::none()
            }
            SemanticCommand::CancelOrBack => {
                context.emit(ChildAction(format!("routed:{state}")));
                context.local_task(async { Some(ChildAction(String::from("mapped-work"))) });
                WidgetEventOutput::none()
            }
            _ => WidgetEventOutput::none(),
        }
    }

    fn subscriptions(
        &self,
        _state: &Self::State,
        _subscriptions: &mut SubscriptionSet<ChildAction>,
    ) {
        self.subscription_calls
            .set(self.subscription_calls.get() + 1);
    }

    fn activation(&self, _state: &Self::State) -> WidgetActivation {
        WidgetActivation::actionable(true)
    }

    fn activate(
        &mut self,
        _state: &mut Self::State,
        context: &mut WidgetActivationContext<ChildAction>,
    ) -> WidgetActivationOutput<ChildAction> {
        context.local_task(async { Some(ChildAction(String::from("default-mapped-work"))) });
        WidgetActivationOutput::action(ChildAction(String::from("semantic-default")))
    }
}

fn observe<Action>(context: &EventContext<'_, Action>, state_before: usize) -> Observation {
    Observation {
        phase: context.phase(),
        original: context.original_target().clone(),
        current: context.current_target().clone(),
        related: context.related_target().cloned(),
        source: context.command_origin().source(),
        derivation: context.command_origin().derivation(),
        sequence: context.sequence(),
        instant: context.instant(),
        cancelable: context.default_is_cancelable(),
        prevented: context.default_is_prevented(),
        stopped: context.propagation_is_stopped(),
        state_before,
    }
}

#[derive(Debug)]
struct State {
    observations: Rc<RefCell<Vec<Observation>>>,
    subscription_calls: Rc<Cell<usize>>,
    actions: Vec<String>,
}

struct App;

impl UiApp for App {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        container(
            EventAncestor {
                observations: Rc::clone(&state.observations),
            },
            children![
                Element::new(EventChild {
                    observations: Rc::clone(&state.observations),
                    subscription_calls: Rc::clone(&state.subscription_calls),
                })
                .id("event.target")
                .key("event-target")
                .map_action(Action::Child),
            ],
        )
        .id("event.root")
        .key("event-root")
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            Action::Child(ChildAction(value)) => state.actions.push(value),
        }
    }
}

fn settle(runtime: &mut AppRuntime<App>) {
    runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
}

fn submit(runtime: &mut AppRuntime<App>, target: MountedNodeId, command: SemanticCommand) {
    runtime
        .submit_command(target, command, CommandOrigin::controller())
        .unwrap_or_else(|_| unreachable!("the exact live target is accepted"));
    assert_eq!(
        runtime
            .pump(PumpBudget::new(1, 0, 0, 0))
            .processed_envelopes(),
        1
    );
}

#[test]
fn downstream_event_mapping_preserves_facts_state_actions_and_work() {
    let observations = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = AppRuntime::<App>::mount(State {
        observations: Rc::clone(&observations),
        subscription_calls: Rc::new(Cell::new(0)),
        actions: Vec::new(),
    });
    settle(&mut runtime);
    observations.borrow_mut().clear();
    let root = runtime.index().nodes()[0].id().clone();
    let target = runtime.index().nodes()[1].id().clone();

    submit(&mut runtime, target.clone(), SemanticCommand::OpenMenu);
    assert!(runtime.state().actions.is_empty());
    let first = observations.borrow().clone();
    assert_eq!(first.len(), 3);
    assert_eq!(
        first.iter().map(|fact| fact.phase).collect::<Vec<_>>(),
        [EventPhase::Capture, EventPhase::Target, EventPhase::Bubble]
    );
    assert!(first.iter().all(|fact| fact.original == target));
    assert_eq!(first[0].current, root);
    assert_eq!(first[1].current, target);
    assert_eq!(first[2].current, root);
    assert!(first.iter().all(|fact| fact.related.is_none()));
    assert!(
        first
            .iter()
            .all(|fact| fact.source == EventSource::Controller)
    );
    assert!(
        first
            .iter()
            .all(|fact| fact.derivation == CommandDerivation::Direct)
    );
    assert!(first.iter().all(|fact| fact.sequence == first[0].sequence));
    assert!(first.iter().all(|fact| fact.instant == first[0].instant));
    assert!(
        first
            .iter()
            .all(|fact| fact.cancelable && !fact.prevented && !fact.stopped)
    );
    assert!(
        runtime
            .trace()
            .kinds()
            .any(|kind| matches!(kind, TraceRecordKind::WidgetStateMutated))
    );

    observations.borrow_mut().clear();
    submit(
        &mut runtime,
        target.clone(),
        SemanticCommand::OpenContextMenu,
    );
    assert_eq!(observations.borrow()[1].state_before, 1);
    settle(&mut runtime);
    assert_eq!(runtime.state().actions, ["action-only:1"]);

    submit(&mut runtime, target, SemanticCommand::CancelOrBack);
    settle(&mut runtime);
    assert_eq!(
        runtime.state().actions,
        ["action-only:1", "routed:1", "mapped-work"]
    );
}

fn assert_interleaved_acceptance_trace(
    runtime: &AppRuntime<App>,
    direct: CommandSubmission,
    target: &MountedNodeId,
) {
    let mut accepted_outputs: Vec<_> = runtime
        .trace()
        .records()
        .filter_map(|record| {
            let sequence = record.work_sequence()?;
            (sequence > direct.sequence()).then(|| {
                let label = match record.kind() {
                    TraceRecordKind::ActionSubmissionAccepted => "action",
                    TraceRecordKind::CommandSubmissionAccepted => "command",
                    _ => return None,
                };
                Some((sequence, label))
            })?
        })
        .collect();
    accepted_outputs.sort_by_key(|(sequence, _)| *sequence);
    assert_eq!(
        accepted_outputs
            .iter()
            .map(|(_, label)| *label)
            .collect::<Vec<_>>(),
        ["action", "command", "action", "action"]
    );
    let delegated = runtime
        .trace()
        .records()
        .find(|record| {
            matches!(record.kind(), TraceRecordKind::CommandSubmissionAccepted)
                && record.command_origin().is_some_and(|origin| {
                    origin.source() == EventSource::Controller
                        && origin.derivation() == CommandDerivation::Delegated
                })
        })
        .unwrap_or_else(|| unreachable!("delegated command acceptance is traced"));
    assert!(
        delegated
            .work_sequence()
            .is_some_and(|sequence| sequence > direct.sequence())
    );
    assert_eq!(delegated.original_target(), Some(target));
}

#[test]
fn downstream_commit_orders_coalesced_reconciliation_interleaved_outputs_and_later_delegation() {
    let observations = Rc::new(RefCell::new(Vec::new()));
    let subscription_calls = Rc::new(Cell::new(0));
    let mut runtime = AppRuntime::<App>::mount(State {
        observations: Rc::clone(&observations),
        subscription_calls: Rc::clone(&subscription_calls),
        actions: Vec::new(),
    });
    settle(&mut runtime);
    let subscription_baseline = subscription_calls.get();
    observations.borrow_mut().clear();
    let target = runtime.index().nodes()[1].id().clone();
    let direct = runtime
        .submit_command(
            target.clone(),
            SemanticCommand::Activate,
            CommandOrigin::controller(),
        )
        .unwrap_or_else(|_| unreachable!("the exact live target is accepted"));

    assert_eq!(
        runtime
            .pump(PumpBudget::new(1, 0, 0, 0))
            .processed_envelopes(),
        1
    );
    assert!(runtime.state().actions.is_empty());
    assert_eq!(observations.borrow().len(), 3);
    assert_eq!(subscription_calls.get(), subscription_baseline);
    assert_interleaved_acceptance_trace(&runtime, direct, &target);

    assert_eq!(
        runtime
            .pump(PumpBudget::new(1, 0, 0, 0))
            .processed_envelopes(),
        1
    );
    assert_eq!(subscription_calls.get(), subscription_baseline + 1);
    assert!(runtime.state().actions.is_empty());

    assert_eq!(
        runtime
            .pump(PumpBudget::new(1, 0, 0, 0))
            .processed_envelopes(),
        1
    );
    assert_eq!(runtime.state().actions, ["routed-first"]);

    assert_eq!(
        runtime
            .pump(PumpBudget::new(1, 0, 0, 0))
            .processed_envelopes(),
        1
    );
    assert_eq!(observations.borrow().len(), 6);
    assert_eq!(runtime.state().actions, ["routed-first"]);

    assert_eq!(
        runtime
            .pump(PumpBudget::new(1, 0, 0, 0))
            .processed_envelopes(),
        1
    );
    assert_eq!(runtime.state().actions, ["routed-first", "routed-second"]);

    assert_eq!(
        runtime
            .pump(PumpBudget::new(1, 0, 0, 0))
            .processed_envelopes(),
        1
    );
    assert_eq!(
        runtime.state().actions,
        ["routed-first", "routed-second", "semantic-default"]
    );
    settle(&mut runtime);
    assert_eq!(
        runtime.state().actions,
        [
            "routed-first",
            "routed-second",
            "semantic-default",
            "routed-mapped-work",
            "default-mapped-work",
        ]
    );
    assert!(runtime.trace().kinds().any(|kind| matches!(
        kind,
        TraceRecordKind::WidgetInvalidated { invalidation }
            if invalidation.contains(WidgetInvalidation::PAINT)
                && invalidation.contains(WidgetInvalidation::SEMANTICS)
    )));
}

#[derive(Debug)]
struct ControlAction;

#[derive(Debug)]
struct ControlWidget {
    name: &'static str,
    log: Rc<RefCell<Vec<String>>>,
    actionable: bool,
    stop_cancel: bool,
    prevent_activate: bool,
}

impl Widget<ControlAction> for ControlWidget {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn event(
        &mut self,
        _state: &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, ControlAction>,
    ) -> WidgetEventOutput {
        self.log
            .borrow_mut()
            .push(format!("{}:{:?}", self.name, context.phase()));
        let Some(command) = event.as_semantic_command() else {
            return WidgetEventOutput::none();
        };
        if self.stop_cancel
            && command.command() == SemanticCommand::CancelOrBack
            && context.phase() == EventPhase::Capture
        {
            context.stop_propagation();
        }
        if self.prevent_activate
            && command.command() == SemanticCommand::Activate
            && context.phase() == EventPhase::Target
        {
            context.prevent_default();
        }
        WidgetEventOutput::none()
    }

    fn activation(&self, _state: &Self::State) -> WidgetActivation {
        WidgetActivation::actionable(self.actionable)
    }

    fn activate(
        &mut self,
        _state: &mut Self::State,
        _context: &mut WidgetActivationContext<ControlAction>,
    ) -> WidgetActivationOutput<ControlAction> {
        self.log.borrow_mut().push(format!("{}:default", self.name));
        WidgetActivationOutput::action(ControlAction)
    }
}

impl ChildLayoutWidget<ControlAction> for ControlWidget {
    fn child_layout(&self, _state: &Self::State) -> ChildLayout {
        ChildLayout::Linear {
            axis: Axis::Vertical,
        }
    }
}

#[derive(Debug)]
struct ControlState {
    log: Rc<RefCell<Vec<String>>>,
    actions: usize,
    stop_cancel: bool,
    prevent_activate: bool,
}

struct ControlApp;

impl UiApp for ControlApp {
    type State = ControlState;
    type Action = ControlAction;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        container(
            ControlWidget {
                name: "root",
                log: Rc::clone(&state.log),
                actionable: false,
                stop_cancel: state.stop_cancel,
                prevent_activate: false,
            },
            children![
                Element::new(ControlWidget {
                    name: "target",
                    log: Rc::clone(&state.log),
                    actionable: true,
                    stop_cancel: false,
                    prevent_activate: state.prevent_activate,
                })
                .id("control.target")
                .key("control-target")
            ],
        )
        .key("control-root")
    }

    fn update(state: &mut Self::State, _action: Self::Action) {
        state.actions += 1;
    }
}

fn control_runtime(
    stop_cancel: bool,
    prevent_activate: bool,
    config: RuntimeConfig,
) -> AppRuntime<ControlApp> {
    AppRuntime::mount_with_config(
        ControlState {
            log: Rc::new(RefCell::new(Vec::new())),
            actions: 0,
            stop_cancel,
            prevent_activate,
        },
        config,
    )
}

fn control_target(runtime: &mut AppRuntime<ControlApp>) -> MountedNodeId {
    runtime.index().nodes()[1].id().clone()
}

fn submit_control(runtime: &mut AppRuntime<ControlApp>, command: SemanticCommand) {
    let target = control_target(runtime);
    runtime
        .submit_command(target, command, CommandOrigin::programmatic())
        .unwrap_or_else(|_| unreachable!("the exact live target is accepted"));
    assert_eq!(
        runtime
            .pump(PumpBudget::new(1, 0, 0, 0))
            .processed_envelopes(),
        1
    );
}

#[test]
fn downstream_controls_keep_propagation_and_default_prevention_independent() {
    let mut stopped = control_runtime(true, false, RuntimeConfig::default());
    settle_control(&mut stopped);
    stopped.state().log.borrow_mut().clear();
    submit_control(&mut stopped, SemanticCommand::CancelOrBack);
    assert_eq!(stopped.state().log.borrow().as_slice(), ["root:Capture"]);
    assert_eq!(stopped.state().actions, 0);

    let mut prevented = control_runtime(false, true, RuntimeConfig::default());
    settle_control(&mut prevented);
    prevented.state().log.borrow_mut().clear();
    submit_control(&mut prevented, SemanticCommand::Activate);
    assert_eq!(
        prevented.state().log.borrow().as_slice(),
        ["root:Capture", "target:Target", "root:Bubble"]
    );
    settle_control(&mut prevented);
    assert_eq!(prevented.state().actions, 0);

    let mut defaulted = control_runtime(false, false, RuntimeConfig::default());
    settle_control(&mut defaulted);
    defaulted.state().log.borrow_mut().clear();
    submit_control(&mut defaulted, SemanticCommand::Activate);
    assert_eq!(
        defaulted.state().log.borrow().as_slice(),
        [
            "root:Capture",
            "target:Target",
            "root:Bubble",
            "target:default",
        ]
    );
    settle_control(&mut defaulted);
    assert_eq!(defaulted.state().actions, 1);
}

#[test]
fn downstream_route_only_commands_route_once_without_implicit_default_or_output() {
    let mut runtime = control_runtime(false, false, RuntimeConfig::default());
    settle_control(&mut runtime);
    runtime.state().log.borrow_mut().clear();
    for command in [
        SemanticCommand::CancelOrBack,
        SemanticCommand::OpenMenu,
        SemanticCommand::OpenContextMenu,
    ] {
        submit_control(&mut runtime, command);
    }
    settle_control(&mut runtime);
    assert_eq!(runtime.state().log.borrow().len(), 9);
    assert!(
        runtime
            .state()
            .log
            .borrow()
            .iter()
            .all(|entry| !entry.ends_with(":default"))
    );
    assert_eq!(runtime.state().actions, 0);
}

#[test]
fn downstream_conservative_rejection_runs_no_callback_and_commits_no_partial_output() {
    let limits = RuntimeLimits::default()
        .with_transaction_outputs(1)
        .with_local_tasks(0);
    let mut runtime = control_runtime(false, false, RuntimeConfig::default().with_limits(limits));
    settle_control(&mut runtime);
    runtime.state().log.borrow_mut().clear();
    submit_control(&mut runtime, SemanticCommand::Activate);
    assert!(runtime.state().log.borrow().is_empty());
    assert_eq!(runtime.state().actions, 0);
    assert!(
        runtime
            .trace()
            .kinds()
            .any(|kind| matches!(kind, TraceRecordKind::RoutedEventAdmissionRejected { .. }))
    );
}

fn settle_control(runtime: &mut AppRuntime<ControlApp>) {
    runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
}
