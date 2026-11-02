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
    Effects, NoHostProtocol, ProducerBudget, SubscriptionSet, UiApp, View, WorkResidencyBudget,
    WorkStartBudget, text,
};
use runenui_runtime::{
    AppRuntime, InitialAdmissionErrorKind, PumpBudget, RuntimeConfig, RuntimeStatus,
};

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
