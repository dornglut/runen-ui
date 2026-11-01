//! #475: APP-05/08, ROUTE-05/11/12, WORK-01/28, M13 finality.
#![allow(clippy::unwrap_used, clippy::panic)]
use runenui_core::*;
use runenui_runtime::{
    AppRuntime, InitialAdmissionErrorKind, PumpBudget, RuntimeConfig, RuntimeStatus,
    RuntimeTerminalReason, TraceConfig,
};
use std::{
    cell::Cell,
    future::Future,
    pin::Pin,
    rc::Rc,
    task::{Context, Poll},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Mount,
    Update,
    AggregateUpdate,
    ValidAggregateUpdate,
    Activation,
    ValidActivation,
    Subscription,
    EffectsFamily,
    EffectsResidency,
    EffectsOutputs,
    InitialEffects,
    InitialFamily,
    InitialResidency,
    NewApplicationSource,
    InitialSubscriptionResidency,
    StableApplicationSubscription,
    DuplicateApplicationSubscription,
}
struct Payload(Rc<Cell<usize>>);
impl Drop for Payload {
    fn drop(&mut self) {
        self.0.set(self.0.get() + 1);
    }
}
enum Action {
    Turn,
    Payload(Payload),
}
#[derive(Clone, Debug)]
struct Counters {
    roots: Rc<Cell<usize>>,
    maps: Rc<Cell<usize>>,
    drops: Rc<Cell<usize>>,
    polls: Rc<Cell<usize>>,
    applied: Rc<Cell<usize>>,
}
impl Counters {
    fn new() -> Self {
        Self {
            roots: Rc::default(),
            maps: Rc::default(),
            drops: Rc::default(),
            polls: Rc::default(),
            applied: Rc::default(),
        }
    }
}
struct State {
    mode: Mode,
    turn: bool,
    counts: Counters,
}
struct Pending(Counters);
impl Drop for Pending {
    fn drop(&mut self) {
        self.0.drops.set(self.0.drops.get() + 1);
    }
}
impl Future for Pending {
    type Output = Option<Action>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        self.0.polls.set(self.0.polls.get() + 1);
        Poll::Pending
    }
}
impl LocalSubscriptionSource<Action> for Pending {
    fn poll_next(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Option<Action>> {
        self.0.polls.set(self.0.polls.get() + 1);
        Poll::Pending
    }
}
#[derive(Debug)]
struct Probe {
    mode: Mode,
    counts: Counters,
}
impl Widget<Action> for Probe {
    type State = ();
    fn create_state(&self) {}
    fn producer_budget(&self, phase: WidgetProducerPhase, (): &()) -> WidgetProducerBudget {
        let outputs = match (self.mode, phase) {
            (Mode::Mount, WidgetProducerPhase::Mount)
            | (Mode::Update, WidgetProducerPhase::Update)
            | (Mode::Activation, WidgetProducerPhase::Activation) => 1,
            (Mode::ValidActivation, WidgetProducerPhase::Activation)
            | (Mode::AggregateUpdate | Mode::ValidAggregateUpdate, WidgetProducerPhase::Update) => {
                2
            }
            _ => 0,
        };
        WidgetProducerBudget::new(outputs)
            .with_subscriptions(usize::from(self.mode == Mode::Subscription))
    }
    fn mount(&self, (): &mut (), ctx: &mut WidgetMountContext<Action>) {
        if self.mode == Mode::Mount {
            let _ = ctx.try_emit(Action::Payload(Payload(self.counts.drops.clone())));
            let (attempted, cause) = ctx
                .try_emit(Action::Payload(Payload(self.counts.drops.clone())))
                .err()
                .unwrap()
                .into_parts();
            assert_eq!(
                cause.phase,
                ProducerPhase::Widget(WidgetProducerPhase::Mount)
            );
            drop(attempted);
        }
    }
    fn update(&self, (): &mut (), ctx: &mut WidgetUpdateContext<Action>) {
        if matches!(
            self.mode,
            Mode::Update | Mode::AggregateUpdate | Mode::ValidAggregateUpdate
        ) {
            let _ = ctx.try_emit(Action::Payload(Payload(self.counts.drops.clone())));
            let _ = ctx.try_emit(Action::Payload(Payload(self.counts.drops.clone())));
        }
    }
    fn activation(&self, (): &()) -> WidgetActivation {
        WidgetActivation::actionable(true)
    }
    fn activate(
        &mut self,
        (): &mut (),
        ctx: &mut WidgetActivationContext<Action>,
    ) -> WidgetActivationOutput<Action> {
        let _ = ctx.try_emit(Action::Payload(Payload(self.counts.drops.clone())));
        WidgetActivationOutput::action(Action::Payload(Payload(self.counts.drops.clone())))
    }
    fn subscriptions(&self, (): &(), set: &mut SubscriptionSet<Action>) {
        if self.mode == Mode::Subscription {
            let (_, _, source) = set
                .try_local(
                    WorkKey::new("owned").unwrap(),
                    0,
                    Pending(self.counts.clone()),
                )
                .err()
                .unwrap()
                .into_parts()
                .0;
            drop(source);
        }
    }
}
struct App;
const fn budget() -> ProducerBudget {
    ProducerBudget::new(64, 16)
        .with_reconciliation(1, 1, 1)
        .with_subscriptions(0, 1)
        .with_work_starts(WorkStartBudget::new(2, 0, 0, 0, 0))
        .with_work_residency(WorkResidencyBudget::new(2, 0, 0, 1, 0, 0))
}
impl UiApp for App {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;
    fn initial_producer_budget(s: &State) -> ProducerBudget {
        if s.mode == Mode::InitialEffects {
            budget().with_application_outputs(0)
        } else if s.mode == Mode::InitialFamily {
            budget().with_work_starts(WorkStartBudget::default())
        } else if s.mode == Mode::InitialResidency {
            budget().with_work_residency(WorkResidencyBudget::default())
        } else if s.mode == Mode::NewApplicationSource {
            budget().with_subscriptions(1, 0)
        } else if s.mode == Mode::InitialSubscriptionResidency {
            budget()
                .with_subscriptions(1, 0)
                .with_work_residency(WorkResidencyBudget::default())
        } else if matches!(
            s.mode,
            Mode::StableApplicationSubscription | Mode::DuplicateApplicationSubscription
        ) {
            budget().with_subscriptions(2, 0)
        } else {
            budget()
        }
    }
    fn interaction_producer_budget(_: &State) -> ProducerBudget {
        budget()
    }
    fn action_producer_budget(s: &State, _: &Action) -> ProducerBudget {
        match s.mode {
            Mode::ValidAggregateUpdate => budget().with_application_outputs(2),
            Mode::NewApplicationSource => budget()
                .with_subscriptions(1, 0)
                .with_work_starts(WorkStartBudget::default()),
            Mode::StableApplicationSubscription | Mode::DuplicateApplicationSubscription => {
                budget()
                    .with_subscriptions(2, 0)
                    .with_work_starts(WorkStartBudget::default())
            }
            Mode::EffectsFamily => budget().with_work_starts(WorkStartBudget::default()),
            Mode::EffectsResidency => {
                budget().with_work_residency(WorkResidencyBudget::new(1, 0, 0, 1, 0, 0))
            }
            Mode::EffectsOutputs | Mode::AggregateUpdate => budget().with_application_outputs(1),
            _ => budget(),
        }
    }
    fn root(s: &State) -> impl View<Action> {
        s.counts.roots.set(s.counts.roots.get() + 1);
        let first = s.counts.maps.clone();
        let second = first.clone();
        Element::new(Probe {
            mode: s.mode,
            counts: s.counts.clone(),
        })
        .map_action(move |a| {
            first.set(first.get() + 1);
            a
        })
        .map_action(move |a| {
            second.set(second.get() + 1);
            a
        })
    }
    fn subscriptions(s: &State, set: &mut SubscriptionSet<Action>) {
        if (s.mode == Mode::NewApplicationSource && s.turn)
            || matches!(
                s.mode,
                Mode::InitialSubscriptionResidency
                    | Mode::StableApplicationSubscription
                    | Mode::DuplicateApplicationSubscription
            )
        {
            let _ = set.try_local(
                WorkKey::new("app.source").unwrap(),
                0,
                Pending(s.counts.clone()),
            );
            if s.mode == Mode::DuplicateApplicationSubscription && s.turn {
                let _ = set.try_local(
                    WorkKey::new("app.source").unwrap(),
                    0,
                    Pending(s.counts.clone()),
                );
            }
        }
    }
    fn initial_effects(s: &State) -> impl IntoEffects<Action, NoHostProtocol> {
        if matches!(
            s.mode,
            Mode::InitialEffects | Mode::InitialFamily | Mode::InitialResidency
        ) {
            Effects::local_task(Pending(s.counts.clone()))
        } else {
            Effects::none()
        }
    }
    fn update(s: &mut State, a: Action) -> impl IntoUpdateOutput<Action, NoHostProtocol> {
        if let Action::Payload(value) = a {
            s.counts.applied.set(s.counts.applied.get() + 1);
            drop(value);
            return Effects::none();
        }
        s.turn = true;
        match s.mode {
            Mode::EffectsFamily => {
                Effects::action(Action::Payload(Payload(s.counts.drops.clone())))
                    .then(Effects::local_task(Pending(s.counts.clone())))
            }
            Mode::EffectsResidency => Effects::local_task(Pending(s.counts.clone()))
                .then(Effects::local_task(Pending(s.counts.clone()))),
            Mode::EffectsOutputs => {
                Effects::action(Action::Payload(Payload(s.counts.drops.clone()))).then(
                    Effects::action(Action::Payload(Payload(s.counts.drops.clone()))),
                )
            }
            _ => Effects::none(),
        }
    }
}
const FULL: PumpBudget = PumpBudget::new(128, 128, 128, 128);
fn mount(
    mode: Mode,
    trace: usize,
    counts: &Counters,
) -> Result<AppRuntime<App>, runenui_runtime::InitialAdmissionError<State>> {
    AppRuntime::<App>::mount_with_config(
        State {
            mode,
            turn: false,
            counts: counts.clone(),
        },
        RuntimeConfig::default().with_trace_config(TraceConfig::new(trace)),
    )
}
fn cause(runtime: &AppRuntime<App>) -> ProducerContractViolation {
    let RuntimeStatus::Terminal(RuntimeTerminalReason::ProducerContractViolation(cause)) =
        runtime.status()
    else {
        panic!("missing typed failure: {:?}", runtime.status())
    };
    cause
}
#[test]
fn mapped_mount_and_initial_effects_return_typed_owned_constructor_errors() {
    for trace in [0, 512] {
        for (mode, phase, component, declared, attempted, drops) in [
            (
                Mode::Mount,
                ProducerPhase::Widget(WidgetProducerPhase::Mount),
                BudgetComponent::Outputs,
                1,
                2,
                2,
            ),
            (
                Mode::InitialEffects,
                ProducerPhase::InitialEffects,
                BudgetComponent::ApplicationOutputs,
                0,
                1,
                1,
            ),
            (
                Mode::InitialFamily,
                ProducerPhase::InitialEffects,
                BudgetComponent::WorkStarts(WorkFamily::LocalTask),
                0,
                1,
                1,
            ),
            (
                Mode::InitialResidency,
                ProducerPhase::InitialEffects,
                BudgetComponent::WorkResidency(WorkFamily::LocalTask),
                0,
                1,
                1,
            ),
            (
                Mode::InitialSubscriptionResidency,
                ProducerPhase::ApplicationSubscriptions,
                BudgetComponent::SubscriptionResidency,
                0,
                1,
                1,
            ),
        ] {
            let counts = Counters::new();
            let error = mount(mode, trace, &counts).err().unwrap();
            let InitialAdmissionErrorKind::ProducerContractViolation(cause) = error.kind() else {
                panic!("{:?}", error.kind())
            };
            assert_eq!(cause.phase, phase);
            assert_eq!(cause.component, component);
            assert_eq!(cause.declared, declared);
            assert_eq!(cause.attempted, Some(attempted));
            assert_eq!(counts.maps.get(), 0);
            assert_eq!(counts.drops.get(), drops);
            assert_eq!(counts.polls.get(), 0);
            let (state, _) = error.into_parts();
            assert!(Rc::ptr_eq(&state.counts.roots, &counts.roots));
        }
    }
}
#[test]
fn mapped_update_disposes_all_provisional_values_without_mapping() {
    for trace in [0, 512] {
        let counts = Counters::new();
        let mut runtime = mount(Mode::Update, trace, &counts).unwrap();
        runtime.pump(FULL).unwrap();
        runtime.submit_action(Action::Turn).unwrap();
        runtime.pump(FULL).unwrap();
        assert_eq!(
            cause(&runtime).phase,
            ProducerPhase::Widget(WidgetProducerPhase::Update)
        );
        assert_eq!(counts.maps.get(), 0);
        assert_eq!(counts.drops.get(), 2);
        assert_eq!(counts.applied.get(), 0);
    }
}
#[test]
fn mapped_activation_primary_and_secondary_share_one_pool_before_mapping() {
    for trace in [0, 512] {
        for valid in [false, true] {
            let counts = Counters::new();
            let mut runtime = mount(
                if valid {
                    Mode::ValidActivation
                } else {
                    Mode::Activation
                },
                trace,
                &counts,
            )
            .unwrap();
            runtime.pump(FULL).unwrap();
            let target = runtime.index().nodes()[0].id().clone();
            runtime
                .submit_command(
                    target,
                    SemanticCommand::Activate,
                    CommandOrigin::programmatic(),
                )
                .unwrap();
            runtime.pump(FULL).unwrap();
            if valid {
                assert_eq!(runtime.status(), RuntimeStatus::Running);
                assert_eq!(counts.maps.get(), 4);
                assert_eq!(counts.applied.get(), 2);
            } else {
                assert_eq!(
                    cause(&runtime).phase,
                    ProducerPhase::Widget(WidgetProducerPhase::Activation)
                );
                assert_eq!(counts.maps.get(), 0);
                assert_eq!(counts.applied.get(), 0);
            }
            assert_eq!(counts.drops.get(), 2);
        }
    }
}
#[test]
fn mapped_subscription_family_violation_returns_source_without_poll_or_partial_diff() {
    for trace in [0, 512] {
        let counts = Counters::new();
        let mut runtime = mount(Mode::Subscription, trace, &counts).unwrap();
        runtime.pump(FULL).unwrap();
        let cause = cause(&runtime);
        assert_eq!(
            cause.phase,
            ProducerPhase::Widget(WidgetProducerPhase::Subscriptions)
        );
        assert_eq!(
            cause.component,
            BudgetComponent::WorkStarts(WorkFamily::LocalTask)
        );
        assert_eq!(counts.polls.get(), 0);
        assert_eq!(counts.drops.get(), 1);
    }
}
#[test]
fn whole_returned_batch_fails_before_root_or_any_provisional_start() {
    for trace in [0, 512] {
        for (mode, component) in [
            (
                Mode::EffectsFamily,
                BudgetComponent::WorkStarts(WorkFamily::LocalTask),
            ),
            (
                Mode::EffectsResidency,
                BudgetComponent::WorkResidency(WorkFamily::LocalTask),
            ),
            (Mode::EffectsOutputs, BudgetComponent::ApplicationOutputs),
        ] {
            let counts = Counters::new();
            let mut runtime = mount(mode, trace, &counts).unwrap();
            runtime.pump(FULL).unwrap();
            let roots = counts.roots.get();
            runtime.submit_action(Action::Turn).unwrap();
            runtime.pump(FULL).unwrap();
            assert!(runtime.state().turn);
            assert_eq!(cause(&runtime).component, component);
            assert_eq!(counts.roots.get(), roots);
            assert_eq!(counts.polls.get(), 0);
            assert_eq!(counts.applied.get(), 0);
            assert_eq!(counts.drops.get(), 2);
        }
    }
}

#[test]
fn retained_and_duplicate_application_declarations_do_not_spend_new_start_credits() {
    for trace in [0, 512] {
        for mode in [
            Mode::StableApplicationSubscription,
            Mode::DuplicateApplicationSubscription,
        ] {
            let counts = Counters::new();
            let mut runtime = mount(mode, trace, &counts).unwrap();
            runtime.pump(FULL).unwrap();
            let before = runtime.trace().len();
            runtime.submit_action(Action::Turn).unwrap();
            runtime.pump(FULL).unwrap();
            assert_eq!(runtime.status(), RuntimeStatus::Running);
            assert_eq!(
                counts.drops.get(),
                if mode == Mode::StableApplicationSubscription {
                    1
                } else {
                    3
                }
            );
            if trace != 0 {
                assert!(runtime.trace().records().skip(before).any(|record|matches!(record.kind(),runenui_runtime::TraceRecordKind::SubscriptionDiffCommitted {started:0,duplicate_keys,..} if *duplicate_keys==usize::from(mode==Mode::DuplicateApplicationSubscription))));
            }
        }
    }
}

#[derive(Debug)]
struct LifecycleStep {
    name: &'static str,
    fail: bool,
    calls: Rc<std::cell::RefCell<Vec<&'static str>>>,
}
impl Widget<()> for LifecycleStep {
    type State = ();
    fn create_state(&self) {}
    fn producer_budget(&self, phase: WidgetProducerPhase, (): &()) -> WidgetProducerBudget {
        WidgetProducerBudget::new(usize::from(
            self.fail && phase == WidgetProducerPhase::Update,
        ))
    }
    fn update(&self, (): &mut (), context: &mut WidgetUpdateContext<()>) {
        self.calls.borrow_mut().push(self.name);
        if self.fail {
            let _ = context.try_emit(());
            let _ = context.try_emit(());
        }
    }
}
struct LifecycleApp;
struct LifecycleState {
    fail: bool,
    calls: Rc<std::cell::RefCell<Vec<&'static str>>>,
}
impl UiApp for LifecycleApp {
    type State = LifecycleState;
    type Action = ();
    type HostProtocol = NoHostProtocol;
    fn initial_producer_budget(_: &LifecycleState) -> ProducerBudget {
        budget().with_reconciliation(3, 3, 3)
    }
    fn action_producer_budget(state: &LifecycleState, (): &()) -> ProducerBudget {
        Self::initial_producer_budget(state)
    }
    fn interaction_producer_budget(state: &LifecycleState) -> ProducerBudget {
        Self::initial_producer_budget(state)
    }
    fn root(state: &LifecycleState) -> impl View<()> {
        column(vec![
            Element::new(LifecycleStep {
                name: "first",
                fail: state.fail,
                calls: state.calls.clone(),
            })
            .key("first"),
            Element::new(LifecycleStep {
                name: "second",
                fail: false,
                calls: state.calls.clone(),
            })
            .key("second"),
        ])
    }
    fn update(_: &mut LifecycleState, (): ()) -> impl IntoUpdateOutput<(), NoHostProtocol> {}
}

#[test]
fn first_lifecycle_failure_prevents_later_mutable_callbacks() {
    for trace in [0, 512] {
        for fail in [false, true] {
            let calls = Rc::new(std::cell::RefCell::new(Vec::new()));
            let mut runtime = AppRuntime::<LifecycleApp>::mount_with_config(
                LifecycleState {
                    fail,
                    calls: calls.clone(),
                },
                RuntimeConfig::default().with_trace_config(TraceConfig::new(trace)),
            )
            .unwrap();
            runtime.pump(FULL).unwrap();
            runtime.submit_action(()).unwrap();
            runtime.pump(FULL).unwrap();
            let expected: &[&str] = if fail {
                &["first"]
            } else {
                &["first", "second"]
            };
            assert_eq!(calls.borrow().as_slice(), expected);
            if fail {
                assert!(
                    matches!(runtime.status(), RuntimeStatus::Terminal(RuntimeTerminalReason::ProducerContractViolation(cause)) if cause.phase == ProducerPhase::Widget(WidgetProducerPhase::Update))
                );
            } else {
                assert_eq!(runtime.status(), RuntimeStatus::Running);
            }
        }
    }
}

struct InitialWorkApp;
impl UiApp for InitialWorkApp {
    type State = Counters;
    type Action = Action;
    type HostProtocol = NoHostProtocol;
    fn initial_producer_budget(_: &Counters) -> ProducerBudget {
        budget()
            .with_subscriptions(0, 0)
            .with_work_starts(WorkStartBudget::new(1, 0, 0, 0, 0))
            .with_work_residency(WorkResidencyBudget::new(1, 0, 0, 0, 0, 0))
    }
    fn action_producer_budget(state: &Counters, _: &Action) -> ProducerBudget {
        Self::initial_producer_budget(state)
    }
    fn interaction_producer_budget(state: &Counters) -> ProducerBudget {
        Self::initial_producer_budget(state)
    }
    fn root(_: &Counters) -> impl View<Action> {
        text("initial work")
    }
    fn initial_effects(state: &Counters) -> impl IntoEffects<Action, NoHostProtocol> {
        Effects::local_task(Pending(state.clone()))
    }
    fn update(_: &mut Counters, _: Action) -> impl IntoUpdateOutput<Action, NoHostProtocol> {}
}

#[test]
fn initial_returned_work_uses_its_reserved_residency_once_at_exact_capacity() {
    for trace in [0, 512] {
        let counts = Counters::new();
        let config = RuntimeConfig::default()
            .with_limits(runenui_runtime::RuntimeLimits::default().with_local_tasks(1))
            .with_trace_config(TraceConfig::new(trace));
        let mut runtime =
            AppRuntime::<InitialWorkApp>::mount_with_config(counts.clone(), config).unwrap();
        runtime.pump(FULL).unwrap();
        assert_eq!(runtime.status(), RuntimeStatus::Running);
        assert!(counts.polls.get() > 0);
        runtime.shutdown().unwrap();
        assert_eq!(counts.drops.get(), 1);
    }
}

#[test]
fn a_new_application_source_is_checked_after_the_complete_desired_set_diff() {
    for trace in [0, 512] {
        let counts = Counters::new();
        let mut runtime = mount(Mode::NewApplicationSource, trace, &counts).unwrap();
        runtime.pump(FULL).unwrap();
        runtime.submit_action(Action::Turn).unwrap();
        runtime.pump(FULL).unwrap();
        let failure = cause(&runtime);
        assert_eq!(failure.phase, ProducerPhase::ApplicationSubscriptions);
        assert_eq!(
            failure.component,
            BudgetComponent::WorkStarts(WorkFamily::LocalTask)
        );
        assert_eq!(failure.declared, 0);
        assert_eq!(failure.attempted, Some(1));
        assert_eq!(counts.polls.get(), 0);
        assert_eq!(counts.drops.get(), 1);
    }
}

#[test]
fn aggregate_lifecycle_outputs_obey_the_application_group_limit() {
    for trace in [0, 512] {
        for valid in [false, true] {
            let counts = Counters::new();
            let mut runtime = mount(
                if valid {
                    Mode::ValidAggregateUpdate
                } else {
                    Mode::AggregateUpdate
                },
                trace,
                &counts,
            )
            .unwrap();
            runtime.pump(FULL).unwrap();
            runtime.submit_action(Action::Turn).unwrap();
            runtime.pump(PumpBudget::new(1, 0, 0, 0)).unwrap();
            if valid {
                assert_eq!(runtime.status(), RuntimeStatus::Running);
                assert_eq!(counts.drops.get(), 0);
            } else {
                let failure = cause(&runtime);
                assert_eq!(failure.phase, ProducerPhase::ApplicationUpdate);
                assert_eq!(failure.component, BudgetComponent::ApplicationOutputs);
                assert_eq!(failure.declared, 1);
                assert_eq!(failure.attempted, Some(2));
                assert_eq!(counts.drops.get(), 2);
            }
            assert_eq!(counts.applied.get(), 0);
            runtime.shutdown().unwrap();
            assert_eq!(counts.drops.get(), 2);
        }
    }
}
