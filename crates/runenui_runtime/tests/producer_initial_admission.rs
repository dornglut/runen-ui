#![allow(refining_impl_trait)]

#[derive(Clone)]
struct OrderState {
    log: std::rc::Rc<std::cell::RefCell<Vec<&'static str>>>,
    reject_effects: bool,
}
#[derive(Debug)]
struct OrderWidget(std::rc::Rc<std::cell::RefCell<Vec<&'static str>>>);
impl runenui_core::Widget<()> for OrderWidget {
    type State = ();
    fn producer_budget(
        &self,
        _: runenui_core::WidgetProducerPhase,
        (): &(),
    ) -> runenui_core::WidgetProducerBudget {
        runenui_core::WidgetProducerBudget::new(0)
    }
    fn create_state(&self) {}
    fn mount(&self, (): &mut (), _: &mut runenui_core::WidgetMountContext<()>) {
        self.0.borrow_mut().push("mount");
    }
    fn subscriptions(&self, (): &(), _: &mut runenui_core::SubscriptionSet<()>) {
        self.0.borrow_mut().push("widget subscriptions");
    }
    fn unmount(&self, (): &mut (), _: &mut runenui_core::WidgetUnmountContext) {
        self.0.borrow_mut().push("unmount");
    }
}
struct OrderApp;
impl UiApp for OrderApp {
    type State = OrderState;
    type Action = ();
    type HostProtocol = NoHostProtocol;
    fn initial_producer_budget(_: &OrderState) -> ProducerBudget {
        ProducerBudget::new(1, 0).with_reconciliation(1, 0, 1)
    }
    fn action_producer_budget(_: &OrderState, (): &()) -> ProducerBudget {
        ProducerBudget::new(1, 0).with_reconciliation(0, 1, 0)
    }
    fn interaction_producer_budget(_: &OrderState) -> ProducerBudget {
        ProducerBudget::new(1, 0).with_reconciliation(0, 1, 0)
    }
    fn root(state: &OrderState) -> impl View<()> {
        state.log.borrow_mut().push("root");
        runenui_core::Element::new(OrderWidget(std::rc::Rc::clone(&state.log)))
    }
    fn initial_effects(state: &OrderState) -> runenui_core::Effects<(), NoHostProtocol> {
        state.log.borrow_mut().push("initial effects");
        if state.reject_effects {
            runenui_core::Effects::action(())
        } else {
            runenui_core::Effects::none()
        }
    }
    fn subscriptions(state: &OrderState, _: &mut runenui_core::SubscriptionSet<()>) {
        state.log.borrow_mut().push("app subscriptions");
    }
    fn update(_: &mut OrderState, (): ()) {}
}

#[test]
fn initial_lifecycle_precedes_effects_and_refusal_unmounts_once() {
    for reject_effects in [false, true] {
        let log = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let result = AppRuntime::<OrderApp>::mount(OrderState {
            log: std::rc::Rc::clone(&log),
            reject_effects,
        });
        if reject_effects {
            let error = result
                .err()
                .unwrap_or_else(|| unreachable!("underdeclared output is rejected"));
            assert!(matches!(
                error.kind(),
                InitialAdmissionErrorKind::ProducerContractViolation(_)
            ));
            assert_eq!(
                &*log.borrow(),
                &["root", "mount", "initial effects", "unmount"]
            );
            drop(error);
            assert_eq!(
                log.borrow()
                    .iter()
                    .filter(|entry| **entry == "unmount")
                    .count(),
                1
            );
        } else {
            let mut app = result.unwrap_or_else(|_| unreachable!("no effects"));
            assert_eq!(
                &*log.borrow(),
                &["root", "mount", "initial effects", "app subscriptions"]
            );
            app.pump(PumpBudget::new(16, 16, 16, 16))
                .unwrap_or_else(|_| unreachable!());
            assert_eq!(log.borrow().last(), Some(&"widget subscriptions"));
            app.shutdown()
                .unwrap_or_else(|_| unreachable!("shutdown observation"));
            drop(app);
            assert_eq!(
                log.borrow()
                    .iter()
                    .filter(|entry| **entry == "unmount")
                    .count(),
                1
            );
        }
    }
}

use std::{cell::Cell, rc::Rc};

use runenui_core::{
    Effects, Element, NoHostProtocol, ProducerBudget, SubscriptionSet, UiApp, View, Widget,
    WidgetMountContext, WorkResidencyBudget, WorkStartBudget, text,
};
use runenui_runtime::{
    AppRuntime, InitialAdmissionErrorKind, PumpBudget, RuntimeConfig, RuntimeStatus, TraceConfig,
};

struct RootedMountState {
    queue_credits: usize,
    mount_calls: Rc<Cell<usize>>,
    accepted_tasks: Rc<Cell<usize>>,
    task_polls: Rc<Cell<usize>>,
    unmount_calls: Rc<Cell<usize>>,
}

#[derive(Debug)]
struct RootedMountTask {
    mount_calls: Rc<Cell<usize>>,
    accepted_tasks: Rc<Cell<usize>>,
    task_polls: Rc<Cell<usize>>,
    unmount_calls: Rc<Cell<usize>>,
}

impl Widget<()> for RootedMountTask {
    type State = ();

    fn create_state(&self) {}

    fn producer_budget(
        &self,
        phase: runenui_core::WidgetProducerPhase,
        (): &(),
    ) -> runenui_core::WidgetProducerBudget {
        if phase == runenui_core::WidgetProducerPhase::Mount {
            runenui_core::WidgetProducerBudget::new(1)
                .with_work_starts(WorkStartBudget::new(1, 0, 0, 0, 0))
        } else {
            runenui_core::WidgetProducerBudget::new(0)
        }
    }

    fn mount(&self, (): &mut (), context: &mut WidgetMountContext<()>) {
        self.mount_calls.set(self.mount_calls.get() + 1);
        let task_polls = Rc::clone(&self.task_polls);
        if context
            .try_local_task(async move {
                task_polls.set(task_polls.get() + 1);
                None
            })
            .is_ok()
        {
            self.accepted_tasks.set(self.accepted_tasks.get() + 1);
        }
    }

    fn unmount(&self, (): &mut (), _: &mut runenui_core::WidgetUnmountContext) {
        self.unmount_calls.set(self.unmount_calls.get() + 1);
    }
}

struct RootedMountApp;

impl UiApp for RootedMountApp {
    type State = RootedMountState;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn initial_producer_budget(state: &Self::State) -> ProducerBudget {
        ProducerBudget::new(state.queue_credits, 1)
            .with_reconciliation(1, 0, 1)
            .with_subscriptions(0, 0)
            .with_work_starts(WorkStartBudget::new(1, 0, 0, 0, 0))
            .with_work_residency(WorkResidencyBudget::new(1, 0, 0, 0, 0, 0))
    }

    fn action_producer_budget(_: &Self::State, (): &()) -> ProducerBudget {
        ProducerBudget::new(1, 0).with_subscriptions(0, 0)
    }

    fn interaction_producer_budget(_: &Self::State) -> ProducerBudget {
        ProducerBudget::new(1, 0)
    }

    fn root(state: &Self::State) -> impl View<Self::Action> {
        Element::new(RootedMountTask {
            mount_calls: Rc::clone(&state.mount_calls),
            accepted_tasks: Rc::clone(&state.accepted_tasks),
            task_polls: Rc::clone(&state.task_polls),
            unmount_calls: Rc::clone(&state.unmount_calls),
        })
    }

    fn update(_: &mut Self::State, (): Self::Action) {}
}

fn rooted_mount_state(queue_credits: usize) -> RootedMountState {
    RootedMountState {
        queue_credits,
        mount_calls: Rc::new(Cell::new(0)),
        accepted_tasks: Rc::new(Cell::new(0)),
        task_polls: Rc::new(Cell::new(0)),
        unmount_calls: Rc::new(Cell::new(0)),
    }
}

#[test]
fn mounted_work_frontier_one_short_is_an_owned_typed_initial_refusal() {
    for trace_capacity in [0, 256] {
        let state = rooted_mount_state(2);
        let mount_calls = Rc::clone(&state.mount_calls);
        let accepted_tasks = Rc::clone(&state.accepted_tasks);
        let task_polls = Rc::clone(&state.task_polls);
        let unmount_calls = Rc::clone(&state.unmount_calls);
        let config = RuntimeConfig::default().with_trace_config(TraceConfig::new(trace_capacity));
        let result = AppRuntime::<RootedMountApp>::mount_with_config(state, config);
        let error = match result {
            Err(error) => error,
            Ok(runtime) => unreachable!(
                "one-short initial root frontier was not refused: status={:?}, mount_calls={}, accepted_tasks={}",
                runtime.status(),
                mount_calls.get(),
                accepted_tasks.get()
            ),
        };
        assert!(matches!(
            error.kind(),
            InitialAdmissionErrorKind::ProducerContractViolation(
                runenui_core::ProducerContractViolation {
                    kind: runenui_core::ProducerViolationKind::DeclarationExceeded,
                    phase: runenui_core::ProducerPhase::InitialEffects,
                    component: runenui_core::BudgetComponent::QueueFrontier,
                    declared: 2,
                    attempted: Some(3),
                    ..
                }
            )
        ));
        assert_eq!(error.state().queue_credits, 2);
        assert_eq!(mount_calls.get(), 1);
        assert_eq!(accepted_tasks.get(), 1);
        assert_eq!(task_polls.get(), 0);
        assert_eq!(unmount_calls.get(), 1);
        let (state, returned_config) = error.into_parts();
        assert_eq!(state.queue_credits, 2);
        assert!(Rc::ptr_eq(&state.mount_calls, &mount_calls));
        assert_eq!(returned_config, config);
        drop(state);
        assert_eq!(unmount_calls.get(), 1);
    }
}

#[test]
fn mounted_work_frontier_exact_fit_admits_and_runs_task() {
    for trace_capacity in [0, 256] {
        let state = rooted_mount_state(3);
        let task_polls = Rc::clone(&state.task_polls);
        let config = RuntimeConfig::default().with_trace_config(TraceConfig::new(trace_capacity));
        let mut runtime = AppRuntime::<RootedMountApp>::mount_with_config(state, config)
            .unwrap_or_else(|_| unreachable!("exact initial root frontier fits"));
        assert_eq!(runtime.status(), RuntimeStatus::Running);
        runtime
            .pump(PumpBudget::new(16, 16, 1, 16))
            .unwrap_or_else(|_| unreachable!("mounted local task polls"));
        assert_eq!(task_polls.get(), 1);
        assert_eq!(runtime.status(), RuntimeStatus::Running);
    }
}

#[derive(Debug)]
struct MountAction;

impl Widget<()> for MountAction {
    type State = ();

    fn create_state(&self) {}

    fn producer_budget(
        &self,
        phase: runenui_core::WidgetProducerPhase,
        (): &(),
    ) -> runenui_core::WidgetProducerBudget {
        if phase == runenui_core::WidgetProducerPhase::Mount {
            runenui_core::WidgetProducerBudget::new(1)
        } else {
            runenui_core::WidgetProducerBudget::new(0)
        }
    }

    fn mount(&self, (): &mut (), context: &mut WidgetMountContext<()>) {
        assert!(context.try_emit(()).is_ok());
    }
}

struct MountActionApp;

impl UiApp for MountActionApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn initial_producer_budget((): &()) -> ProducerBudget {
        ProducerBudget::new(2, 1)
            .with_reconciliation(1, 0, 1)
            .with_subscriptions(0, 0)
    }

    fn action_producer_budget((): &(), (): &()) -> ProducerBudget {
        ProducerBudget::new(3, 1).with_subscriptions(0, 0)
    }

    fn interaction_producer_budget((): &()) -> ProducerBudget {
        ProducerBudget::new(1, 0)
    }

    fn root((): &()) -> impl View<()> {
        Element::new(MountAction)
    }

    fn update((): &mut (), (): ()) {}
}

#[test]
fn mounted_initial_action_cannot_exceed_its_causal_root_profile() {
    for trace_capacity in [0, 256] {
        let config = RuntimeConfig::default().with_trace_config(TraceConfig::new(trace_capacity));
        let error = AppRuntime::<MountActionApp>::mount_with_config((), config)
            .err()
            .unwrap_or_else(|| unreachable!("the mounted action's child profile exceeds its root"));
        assert!(matches!(
            error.kind(),
            InitialAdmissionErrorKind::ProducerContractViolation(
                runenui_core::ProducerContractViolation {
                    kind: runenui_core::ProducerViolationKind::DescendantProfileNotCovered,
                    phase: runenui_core::ProducerPhase::InitialEffects,
                    component: runenui_core::BudgetComponent::QueueFrontier,
                    declared: 2,
                    attempted: Some(3),
                }
            )
        ));
    }
}

#[derive(Clone, Copy)]
enum PhaseAction {
    Trigger,
    Child,
}

struct PhaseState {
    emit_child: bool,
    child_updates: Rc<Cell<usize>>,
}

#[derive(Debug)]
struct PhaseWidget(bool);

impl Widget<PhaseAction> for PhaseWidget {
    type State = ();

    fn create_state(&self) {}

    fn producer_budget(
        &self,
        phase: runenui_core::WidgetProducerPhase,
        (): &(),
    ) -> runenui_core::WidgetProducerBudget {
        if phase == runenui_core::WidgetProducerPhase::Update {
            runenui_core::WidgetProducerBudget::new(1)
        } else {
            runenui_core::WidgetProducerBudget::new(0)
        }
    }

    fn update(&self, (): &mut (), context: &mut runenui_core::WidgetUpdateContext<PhaseAction>) {
        if self.0 {
            let _ = context.try_emit(PhaseAction::Child);
        }
    }
}

struct PhaseApp;

impl UiApp for PhaseApp {
    type State = PhaseState;
    type Action = PhaseAction;
    type HostProtocol = NoHostProtocol;

    fn initial_producer_budget(_: &PhaseState) -> ProducerBudget {
        ProducerBudget::new(1, 0).with_reconciliation(1, 0, 1)
    }

    fn action_producer_budget(_: &PhaseState, action: &PhaseAction) -> ProducerBudget {
        match action {
            PhaseAction::Trigger => ProducerBudget::new(2, 1).with_reconciliation(0, 1, 0),
            PhaseAction::Child => ProducerBudget::new(3, 0).with_reconciliation(0, 1, 0),
        }
    }

    fn interaction_producer_budget(_: &PhaseState) -> ProducerBudget {
        ProducerBudget::new(1, 0)
    }

    fn root(state: &PhaseState) -> impl View<PhaseAction> {
        Element::new(PhaseWidget(state.emit_child)).key("root")
    }

    fn update(state: &mut PhaseState, action: PhaseAction) {
        match action {
            PhaseAction::Trigger => state.emit_child = true,
            PhaseAction::Child => state.child_updates.set(state.child_updates.get() + 1),
        }
    }
}

#[test]
fn mounted_update_child_profile_reports_update_phase() {
    for trace_capacity in [0, 256] {
        let child_updates = Rc::new(Cell::new(0));
        let config = RuntimeConfig::default().with_trace_config(TraceConfig::new(trace_capacity));
        let mut runtime = AppRuntime::<PhaseApp>::mount_with_config(
            PhaseState {
                emit_child: false,
                child_updates: Rc::clone(&child_updates),
            },
            config,
        )
        .unwrap_or_else(|_| unreachable!("initial profile fits"));
        runtime
            .submit_action(PhaseAction::Trigger)
            .unwrap_or_else(|_| unreachable!("the causal root is admitted"));
        runtime
            .pump(PumpBudget::new(16, 16, 16, 16))
            .unwrap_or_else(|_| unreachable!("the root update is processed"));
        let RuntimeStatus::Terminal(
            runenui_runtime::RuntimeTerminalReason::ProducerContractViolation(cause),
        ) = runtime.status()
        else {
            unreachable!("undercovered mounted child terminalizes with a typed violation");
        };
        assert_eq!(
            cause.kind,
            runenui_core::ProducerViolationKind::DescendantProfileNotCovered
        );
        assert_eq!(cause.phase, runenui_core::ProducerPhase::ApplicationUpdate);
        assert_eq!(
            cause.component,
            runenui_core::BudgetComponent::QueueFrontier
        );
        assert_eq!(cause.declared, 2);
        assert_eq!(cause.attempted, Some(3));
        assert_eq!(
            child_updates.get(),
            0,
            "the uncovered child is never accepted"
        );
    }
}

struct State {
    identity: Box<u64>,
    callbacks: Rc<Cell<usize>>,
}

struct App;

impl UiApp for App {
    type State = State;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn initial_producer_budget(_: &State) -> ProducerBudget {
        // One mounted declaration envelope and two local tasks, each with
        // start, outcome and cancellation capacity. Neither task emits actions.
        ProducerBudget::new(7, 2)
            .with_reconciliation(1, 0, 1)
            .with_subscriptions(0, 0)
            .with_work_starts(WorkStartBudget::new(2, 0, 0, 0, 0))
            .with_work_residency(WorkResidencyBudget::new(2, 0, 0, 0, 0, 0))
    }

    fn action_producer_budget(_: &State, (): &()) -> ProducerBudget {
        ProducerBudget::new(1, 0).with_reconciliation(0, 1, 0)
    }

    fn interaction_producer_budget(_: &State) -> ProducerBudget {
        ProducerBudget::new(1, 0)
    }

    fn initial_effects(state: &State) -> Effects<(), NoHostProtocol> {
        state.callbacks.set(state.callbacks.get() + 1);
        Effects::local_task(async { None }).then(Effects::local_task(async { None }))
    }

    fn subscriptions(state: &State, _: &mut SubscriptionSet<()>) {
        state.callbacks.set(state.callbacks.get() + 1);
    }

    fn root(state: &State) -> impl View<()> {
        state.callbacks.set(state.callbacks.get() + 1);
        text("initial generation admission")
    }

    fn update(_: &mut State, (): ()) {}
}

#[cfg(feature = "internal-test-seams")]
#[test]
fn exhausted_initial_generations_refuse_before_any_producer_and_preserve_owned_retry() {
    let callbacks = Rc::new(Cell::new(0));
    let state = State {
        identity: Box::new(475),
        callbacks: Rc::clone(&callbacks),
    };
    let allocation = std::ptr::from_ref(state.identity.as_ref());
    let config = RuntimeConfig::default().__with_initial_next_work_generation_for_test(u64::MAX);
    let Err(refused) = AppRuntime::<App>::mount_with_config(state, config) else {
        unreachable!("two initial generations cannot be reserved at the last generation");
    };
    // Check the temporal obligation even if classification is wrong.
    assert_eq!(
        callbacks.get(),
        0,
        "refusal must precede effects, subscriptions and root"
    );
    assert_eq!(
        refused.kind(),
        InitialAdmissionErrorKind::WorkGenerationExhausted
    );
    let (state, returned_config) = refused.into_parts();
    assert_eq!(std::ptr::from_ref(state.identity.as_ref()), allocation);
    assert_eq!(*state.identity, 475);
    assert_eq!(returned_config.limits(), config.limits());
    let mut runtime = AppRuntime::<App>::mount_with_config(
        state,
        returned_config.__with_initial_next_work_generation_for_test(u64::MAX - 1),
    )
    .unwrap_or_else(|error| unreachable!("the final two generations fit: {error}"));
    assert_eq!(runtime.status(), RuntimeStatus::Running);
    assert_eq!(callbacks.get(), 3);
    let report = runtime
        .pump(PumpBudget::new(
            usize::MAX,
            usize::MAX,
            usize::MAX,
            usize::MAX,
        ))
        .unwrap_or_else(|_| unreachable!());
    assert!(report.report().is_quiescent());
    assert_eq!(runtime.status(), RuntimeStatus::Running);
}

#[cfg(feature = "internal-test-seams")]
#[test]
fn initial_resource_refusals_preserve_exact_state_and_config_before_producers() {
    for (config, expected) in [
        (
            RuntimeConfig::default().with_queue_capacity(0),
            InitialAdmissionErrorKind::Capacity,
        ),
        (
            RuntimeConfig::default().__with_initial_next_work_sequence_for_test(u64::MAX),
            InitialAdmissionErrorKind::SequenceExhausted,
        ),
        (
            RuntimeConfig::default()
                .with_limits(runenui_runtime::RuntimeLimits::default().with_local_tasks(1)),
            InitialAdmissionErrorKind::WorkCapacity(runenui_runtime::WorkCapacityFamily::LocalTask),
        ),
        (
            RuntimeConfig::default().__with_mounted_public_slot_limit_for_test(0),
            InitialAdmissionErrorKind::MountedIdentityExhausted,
        ),
    ] {
        let callbacks = Rc::new(Cell::new(0));
        let state = State {
            identity: Box::new(475),
            callbacks: Rc::clone(&callbacks),
        };
        let allocation = std::ptr::from_ref(state.identity.as_ref());
        let Err(refused) = AppRuntime::<App>::mount_with_config(state, config) else {
            unreachable!("the declared initial resources cannot fit");
        };
        assert_eq!(refused.kind(), expected);
        assert_eq!(
            callbacks.get(),
            0,
            "{expected:?} must precede every initial producer"
        );
        let (state, returned_config) = refused.into_parts();
        assert_eq!(std::ptr::from_ref(state.identity.as_ref()), allocation);
        assert_eq!(returned_config, config);
    }
}
