#![allow(refining_impl_trait)]

use std::{
    cell::RefCell,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use runenui_core::{
    CommandOrigin, Effects, Element, IntoEffects, MountedNodeId, NoHostProtocol, SemanticCommand,
    SubscriptionSet, TimerEffect, UiApp, Widget, WidgetActivation, WidgetActivationContext,
    WidgetActivationOutput, WidgetInvalidation, WidgetMountContext, WidgetUpdateContext, WorkKey,
};
use runenui_runtime::{AppRuntime, PumpBudget, TraceRecordKind, WorkSequence};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Action {
    Mounted,
    Initial,
    Local,
    Timer,
    SendRefused,
    TriggerUpdate,
    Updated,
    ActivationReturned,
    ActivationEmitted,
}

#[derive(Debug)]
struct OutputWidget {
    emit_update: bool,
}

impl Widget<Action> for OutputWidget {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn mount(&self, (): &mut Self::State, context: &mut WidgetMountContext<Action>) {
        context.emit(Action::Mounted);
        context.keyed_local_task(key("mounted.local"), async { Some(Action::Local) });
        context.timer(TimerEffect::once(Duration::ZERO, || Action::Timer));
        context.send_task_with_failure(async { 7_u8 }, |_| unreachable!(), |_| Action::SendRefused);
    }

    fn update(&self, (): &mut Self::State, context: &mut WidgetUpdateContext<Action>) {
        if self.emit_update {
            context.emit(Action::Updated);
        }
    }

    fn activation(&self, (): &Self::State) -> WidgetActivation {
        WidgetActivation::actionable(true)
    }

    fn activate(
        &mut self,
        (): &mut Self::State,
        context: &mut WidgetActivationContext<Action>,
    ) -> WidgetActivationOutput<Action> {
        context.emit(Action::ActivationEmitted);
        WidgetActivationOutput::action(Action::ActivationReturned)
    }
}

#[derive(Debug, Default)]
struct State {
    actions: Vec<Action>,
    emit_update: bool,
}

struct App;

impl UiApp for App {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> Element<Self::Action> {
        Element::new(OutputWidget {
            emit_update: state.emit_update,
        })
        .key("output")
    }

    fn initial_effects(_: &Self::State) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        Effects::action(Action::Initial)
    }

    fn update(
        state: &mut Self::State,
        action: Self::Action,
    ) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        state.actions.push(action);
        match action {
            Action::TriggerUpdate => state.emit_update = true,
            Action::Updated => state.emit_update = false,
            _ => {}
        }
    }
}

fn key(value: &str) -> WorkKey {
    WorkKey::new(value).unwrap_or_else(|_| unreachable!())
}

fn drain<Application: UiApp>(runtime: &mut AppRuntime<Application>) {
    runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
}

fn submit_activate<Application: UiApp>(
    runtime: &mut AppRuntime<Application>,
    target: MountedNodeId,
) -> WorkSequence {
    runtime
        .submit_command(
            target,
            SemanticCommand::Activate,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("the exact live target is accepted"))
        .sequence()
}

#[test]
fn mounted_callbacks_emit_exact_owner_work_through_the_canonical_scheduler() {
    let mut runtime = AppRuntime::<App>::mount(State::default());
    drain(&mut runtime);
    assert_eq!(
        runtime.state().actions[0..2],
        [Action::Initial, Action::Mounted]
    );
    assert!(runtime.state().actions.contains(&Action::Local));
    assert!(runtime.state().actions.contains(&Action::Timer));
    assert!(runtime.state().actions.contains(&Action::SendRefused));

    runtime
        .submit_action(Action::TriggerUpdate)
        .unwrap_or_else(|_| unreachable!());
    drain(&mut runtime);
    assert!(runtime.state().actions.contains(&Action::Updated));

    let target = runtime.index().nodes()[0].id().clone();
    submit_activate(&mut runtime, target);
    drain(&mut runtime);
    let actions = &runtime.state().actions;
    let returned = actions
        .iter()
        .position(|action| *action == Action::ActivationReturned)
        .unwrap_or_else(|| unreachable!());
    let emitted = actions
        .iter()
        .position(|action| *action == Action::ActivationEmitted)
        .unwrap_or_else(|| unreachable!());
    assert!(returned < emitted);
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ActivationOrderAction {
    Primary,
    Auxiliary,
    Task,
}

#[derive(Debug)]
struct ActivationOrderWidget {
    declarations: Rc<RefCell<Vec<usize>>>,
}

impl Widget<ActivationOrderAction> for ActivationOrderWidget {
    type State = usize;

    fn create_state(&self) -> Self::State {
        0
    }

    fn activation(&self, _: &Self::State) -> WidgetActivation {
        WidgetActivation::actionable(true)
    }

    fn activate(
        &mut self,
        state: &mut Self::State,
        context: &mut WidgetActivationContext<ActivationOrderAction>,
    ) -> WidgetActivationOutput<ActivationOrderAction> {
        *state += 1;
        context.invalidate_subscriptions();
        context.emit(ActivationOrderAction::Auxiliary);
        context.local_task(async { Some(ActivationOrderAction::Task) });
        WidgetActivationOutput::changed_with_action(ActivationOrderAction::Primary)
    }

    fn subscriptions(&self, state: &Self::State, _: &mut SubscriptionSet<ActivationOrderAction>) {
        self.declarations.borrow_mut().push(*state);
    }
}

struct ActivationOrderApp;

impl UiApp for ActivationOrderApp {
    type State = (Rc<RefCell<Vec<usize>>>, Vec<ActivationOrderAction>);
    type Action = ActivationOrderAction;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> Element<Self::Action> {
        Element::new(ActivationOrderWidget {
            declarations: Rc::clone(&state.0),
        })
        .key("activation-order")
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        state.1.push(action);
    }
}

#[test]
fn routed_activation_commits_subscription_then_primary_and_auxiliary_actions() {
    let declarations = Rc::new(RefCell::new(Vec::new()));
    let mut runtime =
        AppRuntime::<ActivationOrderApp>::mount((Rc::clone(&declarations), Vec::new()));
    runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(&*declarations.borrow(), &[0]);

    let target = runtime.index().nodes()[0].id().clone();
    let command_sequence = submit_activate(&mut runtime, target);
    assert_eq!(command_sequence.get(), 2);
    runtime.pump(PumpBudget::new(1, usize::MAX, 0, usize::MAX));
    let action_sequences: Vec<_> = runtime
        .trace()
        .records()
        .filter(|record| matches!(record.kind(), TraceRecordKind::ActionSubmissionAccepted))
        .filter_map(runenui_runtime::TraceRecord::work_sequence)
        .collect();
    assert_eq!(
        action_sequences
            .iter()
            .map(|sequence| sequence.get())
            .collect::<Vec<_>>(),
        [4, 5],
        "subscription reconciliation receives sequence 3"
    );
    assert_eq!(&*declarations.borrow(), &[0]);
    assert!(runtime.state().1.is_empty());

    runtime.pump(PumpBudget::new(1, usize::MAX, 0, usize::MAX));
    assert_eq!(&*declarations.borrow(), &[0, 1]);
    assert!(runtime.state().1.is_empty());

    drain(&mut runtime);
    assert_eq!(
        runtime.state().1,
        [
            ActivationOrderAction::Primary,
            ActivationOrderAction::Auxiliary,
            ActivationOrderAction::Task,
        ]
    );
    assert_eq!(&*declarations.borrow(), &[0, 1]);
}

#[derive(Debug)]
struct RoutedOutputWidget;

impl Widget<()> for RoutedOutputWidget {
    type State = usize;

    fn create_state(&self) -> Self::State {
        0
    }

    fn activation(&self, _: &Self::State) -> WidgetActivation {
        WidgetActivation::actionable(true)
    }

    fn activate(
        &mut self,
        state: &mut Self::State,
        context: &mut WidgetActivationContext<()>,
    ) -> WidgetActivationOutput<()> {
        match *state {
            0 => context.emit(()),
            1 => context.local_task(async { None }),
            2 => context.timer(TimerEffect::once(Duration::from_secs(1), || ())),
            3 => context.invalidate_subscriptions(),
            4 => context.invalidate(WidgetInvalidation::PAINT),
            _ => return WidgetActivationOutput::none(),
        }
        *state += 1;
        WidgetActivationOutput::changed()
    }
}

struct RoutedOutputApp;

impl UiApp for RoutedOutputApp {
    type State = usize;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(_: &Self::State) -> Element<Self::Action> {
        Element::new(RoutedOutputWidget).key("routed-output")
    }

    fn update(state: &mut Self::State, (): Self::Action) {
        *state += 1;
    }
}

#[test]
fn routed_activation_separates_scheduler_wake_from_redraw() {
    let mut runtime = AppRuntime::<RoutedOutputApp>::mount(0);
    drain(&mut runtime);
    let initial_redraw = runtime
        .take_redraw_request()
        .unwrap_or_else(|| unreachable!("mount is initially publication-dirty"));
    runtime
        .acknowledge_redraw(&initial_redraw)
        .unwrap_or_else(|_| unreachable!("runtime-local redraw request is valid"));
    let wakes = Arc::new(AtomicUsize::new(0));
    let wake_probe = Arc::clone(&wakes);
    runtime.set_wake_transport(move || {
        wake_probe.fetch_add(1, Ordering::SeqCst);
    });
    let target = runtime.index().nodes()[0].id().clone();

    let auxiliary = submit_activate(&mut runtime, target.clone());
    assert_eq!(wakes.load(Ordering::SeqCst), 1);
    runtime.pump(PumpBudget::new(1, 0, 0, 0));
    assert!(runtime.take_redraw_request().is_none());
    drain(&mut runtime);
    let action_redraw = runtime
        .take_redraw_request()
        .unwrap_or_else(|| unreachable!("the routed auxiliary action updates application state"));
    runtime
        .acknowledge_redraw(&action_redraw)
        .unwrap_or_else(|_| unreachable!("runtime-local redraw request is valid"));

    let task = submit_activate(&mut runtime, target.clone());
    assert!(task > auxiliary);
    runtime.pump(PumpBudget::new(1, 0, 0, 0));
    assert!(runtime.take_redraw_request().is_none());
    drain(&mut runtime);

    submit_activate(&mut runtime, target.clone());
    drain(&mut runtime);

    submit_activate(&mut runtime, target.clone());
    drain(&mut runtime);

    submit_activate(&mut runtime, target.clone());
    runtime.pump(PumpBudget::new(1, 0, 0, 0));
    assert!(runtime.take_redraw_request().is_some());
    submit_activate(&mut runtime, target);
    runtime.pump(PumpBudget::new(1, 0, 0, 0));
}

#[derive(Debug)]
struct CoalescedInvalidationWidget;

impl Widget<()> for CoalescedInvalidationWidget {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn activation(&self, (): &Self::State) -> WidgetActivation {
        WidgetActivation::actionable(true)
    }

    fn activate(
        &mut self,
        (): &mut Self::State,
        context: &mut WidgetActivationContext<()>,
    ) -> WidgetActivationOutput<()> {
        context.invalidate_subscriptions();
        WidgetActivationOutput::none()
    }
}

struct CoalescedInvalidationApp;

impl UiApp for CoalescedInvalidationApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> Element<Self::Action> {
        Element::new(CoalescedInvalidationWidget).key("coalesced-invalidation")
    }

    fn update((): &mut Self::State, (): Self::Action) {}
}

#[test]
fn coalesced_subscription_invalidation_is_an_effect_not_no_effect() {
    let mut runtime = AppRuntime::<CoalescedInvalidationApp>::mount(());
    drain(&mut runtime);
    let target = runtime.index().nodes()[0].id().clone();
    submit_activate(&mut runtime, target.clone());
    submit_activate(&mut runtime, target);
    runtime.pump(PumpBudget::new(2, 0, 0, 0));
    assert_eq!(
        runtime
            .trace()
            .kinds()
            .filter(|kind| matches!(kind, TraceRecordKind::MountedSubscriptionInvalidated))
            .count(),
        2,
        "both callbacks report invalidation while queue reconciliation coalesces"
    );
}
