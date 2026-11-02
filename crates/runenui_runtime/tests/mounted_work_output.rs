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
    fn producer_budget(
        &self,
        phase: runenui_core::WidgetProducerPhase,
        _state: &Self::State,
    ) -> runenui_core::WidgetProducerBudget {
        match phase {
            runenui_core::WidgetProducerPhase::Event => runenui_core::WidgetProducerBudget::new(0)
                .with_work_starts(runenui_core::WorkStartBudget::new(1, 1, 1, 0, 0)),
            runenui_core::WidgetProducerPhase::Activation => {
                runenui_core::WidgetProducerBudget::new(2)
                    .with_work_starts(runenui_core::WorkStartBudget::new(1, 1, 1, 0, 0))
            }
            runenui_core::WidgetProducerPhase::Mount => runenui_core::WidgetProducerBudget::new(4)
                .with_work_starts(runenui_core::WorkStartBudget::new(1, 1, 1, 0, 0)),
            runenui_core::WidgetProducerPhase::Subscriptions => {
                runenui_core::WidgetProducerBudget::new(0).with_subscriptions(0)
            }
            runenui_core::WidgetProducerPhase::Update => {
                runenui_core::WidgetProducerBudget::new(usize::from(self.emit_update))
            }
            _ => runenui_core::WidgetProducerBudget::new(0),
        }
    }

    type State = ();

    fn create_state(&self) -> Self::State {}

    fn mount(&self, (): &mut Self::State, context: &mut WidgetMountContext<Action>) {
        let _ = context.try_emit(Action::Mounted);
        {
            let _ =
                context.try_keyed_local_task(key("mounted.local"), async { Some(Action::Local) });
        };
        let _ = context.try_timer(TimerEffect::once(Duration::ZERO, || Action::Timer));
        let _ = context.try_send_task_with_failure(
            async { 7_u8 },
            |_| unreachable!(),
            |_| Action::SendRefused,
        );
    }

    fn update(&self, (): &mut Self::State, context: &mut WidgetUpdateContext<Action>) {
        if self.emit_update {
            let _ = context.try_emit(Action::Updated);
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
        let _ = context.try_emit(Action::ActivationEmitted);
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
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        // Two initial actions, three starts, one dirty-owner reconciliation,
        // and one cancellation credit for each live work generation. The
        // Timer start's own Q becomes its first firing credit.
        runenui_core::ProducerBudget::new(9, 5)
            .with_reconciliation(1, 0, 1)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(1, 1, 1, 0, 0))
            .with_work_residency(runenui_core::WorkResidencyBudget::new(1, 1, 1, 0, 0, 0))
    }

    fn action_producer_budget(
        _state: &Self::State,
        action: &Self::Action,
    ) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, usize::from(matches!(action, Action::TriggerUpdate)))
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn interaction_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        // The routed command profile includes the target Event and Activation
        // phase maxima; each phase may emit two actions and start local, send,
        // and timer work. The frontier includes their cleanup/outcome credits.
        runenui_core::ProducerBudget::new(18, 8)
            .with_work_starts(runenui_core::WorkStartBudget::new(2, 2, 2, 0, 0))
    }

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
    ) -> impl runenui_core::IntoUpdateOutput<Self::Action, Self::HostProtocol> {
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
        .unwrap_or_else(|error| {
            unreachable!(
                "the exact live target is accepted: {error:?}; status={:?}, trace={:?}",
                runtime.status(),
                runtime
                    .trace()
                    .records()
                    .map(runenui_runtime::TraceRecord::kind)
                    .collect::<Vec<_>>(),
            )
        })
        .sequence()
}

#[test]
fn mounted_callbacks_emit_exact_owner_work_through_the_canonical_scheduler() {
    let mut runtime = AppRuntime::<App>::mount(State::default())
        .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    drain(&mut runtime);
    assert!(
        matches!(runtime.status(), runenui_runtime::RuntimeStatus::Running),
        "mounted output profile must fund its complete frontier; status={:?}, trace={:#?}",
        runtime.status(),
        runtime
            .trace()
            .records()
            .map(runenui_runtime::TraceRecord::kind)
            .collect::<Vec<_>>()
    );
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
    assert!(
        runtime.state().actions.contains(&Action::Updated),
        "status: {:?}; trace: {:#?}",
        runtime.status(),
        runtime
            .trace()
            .records()
            .map(runenui_runtime::TraceRecord::kind)
            .collect::<Vec<_>>()
    );

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
    fn producer_budget(
        &self,
        phase: runenui_core::WidgetProducerPhase,
        _state: &Self::State,
    ) -> runenui_core::WidgetProducerBudget {
        match phase {
            runenui_core::WidgetProducerPhase::Event | runenui_core::WidgetProducerPhase::Mount => {
                runenui_core::WidgetProducerBudget::new(0)
                    .with_work_starts(runenui_core::WorkStartBudget::new(1, 0, 0, 0, 0))
            }
            runenui_core::WidgetProducerPhase::Activation => {
                runenui_core::WidgetProducerBudget::new(4)
                    .with_work_starts(runenui_core::WorkStartBudget::new(1, 0, 0, 0, 0))
            }
            runenui_core::WidgetProducerPhase::Subscriptions => {
                runenui_core::WidgetProducerBudget::new(0).with_subscriptions(1)
            }
            _ => runenui_core::WidgetProducerBudget::new(0),
        }
    }

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
        let _ = context.try_invalidate_subscriptions();
        let _ = context.try_emit(ActivationOrderAction::Auxiliary);
        let _ = context.try_local_task(async { Some(ActivationOrderAction::Task) });
        WidgetActivationOutput::changed_with_action(ActivationOrderAction::Primary)
    }

    fn subscriptions(&self, state: &Self::State, _: &mut SubscriptionSet<ActivationOrderAction>) {
        self.declarations.borrow_mut().push(*state);
    }
}

struct ActivationOrderApp;

impl UiApp for ActivationOrderApp {
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0)
            .with_reconciliation(1, 0, 1)
            .with_subscriptions(0, 1)
            .with_work_starts(runenui_core::WorkStartBudget::new(1, 0, 0, 0, 0))
    }

    fn action_producer_budget(
        _state: &Self::State,
        _action: &Self::Action,
    ) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn interaction_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        // The target Event and Activation maxima include four activation
        // outputs and one local start per phase; reconciliation retains one
        // dirty subscription owner.
        runenui_core::ProducerBudget::new(8, 6)
            .with_reconciliation(0, 0, 1)
            .with_subscriptions(0, 1)
            .with_work_starts(runenui_core::WorkStartBudget::new(2, 0, 0, 0, 0))
            .with_work_residency(runenui_core::WorkResidencyBudget::new(1, 0, 0, 0, 0, 0))
    }

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
        AppRuntime::<ActivationOrderApp>::mount((Rc::clone(&declarations), Vec::new()))
            .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    let _ = runtime
        .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(&*declarations.borrow(), &[0]);

    let target = runtime.index().nodes()[0].id().clone();
    let command_sequence = submit_activate(&mut runtime, target);
    assert_eq!(command_sequence.get(), 2);
    let _ = runtime
        .pump(PumpBudget::new(1, usize::MAX, 0, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
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
    assert_eq!(runtime.state().1, Vec::<ActivationOrderAction>::new());

    let _ = runtime
        .pump(PumpBudget::new(1, usize::MAX, 0, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(&*declarations.borrow(), &[0, 1]);
    assert_eq!(runtime.state().1, Vec::<ActivationOrderAction>::new());

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
    fn producer_budget(
        &self,
        phase: runenui_core::WidgetProducerPhase,
        _state: &Self::State,
    ) -> runenui_core::WidgetProducerBudget {
        match phase {
            runenui_core::WidgetProducerPhase::Event | runenui_core::WidgetProducerPhase::Mount => {
                runenui_core::WidgetProducerBudget::new(0)
                    .with_work_starts(runenui_core::WorkStartBudget::new(1, 0, 1, 0, 0))
            }
            runenui_core::WidgetProducerPhase::Activation => {
                runenui_core::WidgetProducerBudget::new(4)
                    .with_work_starts(runenui_core::WorkStartBudget::new(1, 0, 1, 0, 0))
            }
            runenui_core::WidgetProducerPhase::Subscriptions => {
                runenui_core::WidgetProducerBudget::new(0).with_subscriptions(0)
            }
            _ => runenui_core::WidgetProducerBudget::new(0),
        }
    }

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
            0 => {
                let _ = context.try_emit(());
            }
            1 => {
                let _ = context.try_local_task(async { None });
            }
            2 => {
                let _ = context.try_timer(TimerEffect::once(Duration::from_secs(1), || ()));
            }
            3 => {
                let _ = context.try_invalidate_subscriptions();
            }
            4 => context.invalidate(WidgetInvalidation::PAINT),
            _ => return WidgetActivationOutput::none(),
        }
        *state += 1;
        WidgetActivationOutput::changed()
    }
}

struct RoutedOutputApp;

impl UiApp for RoutedOutputApp {
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0)
            .with_reconciliation(1, 0, 1)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(1, 0, 1, 0, 0))
    }

    fn action_producer_budget(
        _state: &Self::State,
        _action: &Self::Action,
    ) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn interaction_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        // Event and Activation phase maxima each permit four outputs, one
        // local task, and one timer, with their future obligations retained.
        runenui_core::ProducerBudget::new(16, 8)
            .with_reconciliation(0, 0, 1)
            .with_work_starts(runenui_core::WorkStartBudget::new(2, 0, 2, 0, 0))
            // Each activation takes one state-selected branch, so this root
            // retains at most one task or one timer across its descendants.
            .with_work_residency(runenui_core::WorkResidencyBudget::new(1, 0, 1, 0, 0, 0))
    }

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
    let mut runtime = AppRuntime::<RoutedOutputApp>::mount(0)
        .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
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
    let _ = runtime
        .pump(PumpBudget::new(1, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert!(runtime.take_redraw_request().is_none());
    drain(&mut runtime);
    assert_eq!(
        runtime.status(),
        runenui_runtime::RuntimeStatus::Running,
        "status after routed action child"
    );
    let action_redraw = runtime
        .take_redraw_request()
        .unwrap_or_else(|| unreachable!("the routed auxiliary action updates application state"));
    runtime
        .acknowledge_redraw(&action_redraw)
        .unwrap_or_else(|_| unreachable!("runtime-local redraw request is valid"));

    let task = submit_activate(&mut runtime, target.clone());
    assert!(task > auxiliary);
    let _ = runtime
        .pump(PumpBudget::new(1, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert!(runtime.take_redraw_request().is_none());
    drain(&mut runtime);

    submit_activate(&mut runtime, target.clone());
    drain(&mut runtime);

    submit_activate(&mut runtime, target.clone());
    drain(&mut runtime);

    submit_activate(&mut runtime, target.clone());
    let _ = runtime
        .pump(PumpBudget::new(1, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert!(runtime.take_redraw_request().is_some());
    submit_activate(&mut runtime, target);
    let _ = runtime
        .pump(PumpBudget::new(1, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
}

#[derive(Debug)]
struct CoalescedInvalidationWidget;

impl Widget<()> for CoalescedInvalidationWidget {
    fn producer_budget(
        &self,
        phase: runenui_core::WidgetProducerPhase,
        _state: &Self::State,
    ) -> runenui_core::WidgetProducerBudget {
        match phase {
            runenui_core::WidgetProducerPhase::Activation => {
                runenui_core::WidgetProducerBudget::new(1)
            }
            runenui_core::WidgetProducerPhase::Subscriptions => {
                runenui_core::WidgetProducerBudget::new(0).with_subscriptions(0)
            }
            _ => runenui_core::WidgetProducerBudget::new(0),
        }
    }

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
        let _ = context.try_invalidate_subscriptions();
        WidgetActivationOutput::none()
    }
}

struct CoalescedInvalidationApp;

impl UiApp for CoalescedInvalidationApp {
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0)
            .with_reconciliation(1, 0, 1)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn action_producer_budget(
        _state: &Self::State,
        _action: &Self::Action,
    ) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn interaction_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        // A coalesced invalidation can enqueue one mounted subscription
        // reconciliation after the activation envelope is popped.
        runenui_core::ProducerBudget::new(2, 1).with_reconciliation(0, 0, 1)
    }

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
    let mut runtime = AppRuntime::<CoalescedInvalidationApp>::mount(())
        .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    drain(&mut runtime);
    let target = runtime.index().nodes()[0].id().clone();
    submit_activate(&mut runtime, target.clone());
    submit_activate(&mut runtime, target);
    let _ = runtime
        .pump(PumpBudget::new(2, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(
        runtime
            .trace()
            .kinds()
            .filter(|kind| matches!(kind, TraceRecordKind::MountedSubscriptionInvalidated))
            .count(),
        2,
        "both callbacks report invalidation while queue reconciliation coalesces; status={:?}, trace={:?}",
        runtime.status(),
        runtime
            .trace()
            .records()
            .map(runenui_runtime::TraceRecord::kind)
            .collect::<Vec<_>>(),
    );
}
