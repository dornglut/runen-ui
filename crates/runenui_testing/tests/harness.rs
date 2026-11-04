use core::num::NonZeroUsize;

use runenui_core::{
    ElementId, LayoutDimension, LayoutStyle, LogicalLength, NoHostProtocol, PointerButton,
    PointerButtons, SemanticAction, SemanticRole, UiApp, View, button, children, column,
};
use runenui_runtime::{LogicalPoint, PointerDeviceKind, PointerId, PointerPhase, PumpBudget};
use runenui_testing::{
    SemanticQuery, SettleBudget, SettleOutcome, TestHarness, UniqueSemanticQueryError,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Action {
    Increment,
    Reset,
}

struct HarnessApp;

impl UiApp for HarnessApp {
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        // 3 fixed mounted owners, each with one initial subscription reconciliation.
        runenui_core::ProducerBudget::new(3, 0).with_reconciliation(3, 0, 3)
    }

    fn action_producer_budget(
        _state: &Self::State,
        _action: &Self::Action,
    ) -> runenui_core::ProducerBudget {
        // Updates retain the same fixed tree and start no app or subscription work.
        runenui_core::ProducerBudget::new(1, 0).with_reconciliation(0, 3, 0)
    }

    fn interaction_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        // Routed/default outputs and their fixed-tree action descendants.
        // Ten possible route/boundary/focus/capture callback outputs, plus
        // activation and cursor; retain both cursor outcome pairs and cleanup.
        runenui_core::ProducerBudget::new(16, 12)
            .with_reconciliation(0, 3, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 1))
            .with_work_residency(runenui_core::WorkResidencyBudget::new(0, 0, 0, 0, 0, 2))
    }

    type State = i32;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(_: &Self::State) -> impl View<Self::Action> {
        column(children![
            button("Increment")
                .id("harness.increment")
                .on_activate(|| Action::Increment)
                .with_layout(button_layout()),
            button("Reset")
                .id("harness.reset")
                .on_activate(|| Action::Reset)
                .with_layout(button_layout()),
        ])
    }

    fn update(
        state: &mut Self::State,
        action: Self::Action,
    ) -> impl runenui_core::IntoUpdateOutput<Self::Action, Self::HostProtocol> {
        match action {
            Action::Increment => *state += 1,
            Action::Reset => *state = 0,
        }
    }
}

fn button_layout() -> LayoutStyle {
    LayoutStyle::default()
        .with_width(LayoutDimension::Length(LogicalLength::from(80_u16)))
        .with_height(LayoutDimension::Length(LogicalLength::from(24_u16)))
}

fn settle_budget() -> SettleBudget {
    let iterations = NonZeroUsize::new(8).unwrap_or(NonZeroUsize::MIN);
    SettleBudget::new(iterations, PumpBudget::new(64, 64, 64, 64))
}

#[test]
fn initial_refusal_returns_nonclone_state_and_both_configurations_for_retry() {
    use runenui_core::{
        Element, ProducerBudget, Widget, WidgetProducerBudget, WidgetProducerPhase,
    };
    use runenui_runtime::{InitialAdmissionErrorKind, LogicalSize, RuntimeConfig};
    use runenui_testing::TestSurfaceConfig;
    use std::{cell::Cell, rc::Rc};

    struct OwnedState {
        marker: Box<u8>,
        callbacks: Rc<Cell<usize>>,
    }

    #[derive(Debug)]
    struct Leaf(Rc<Cell<usize>>);

    impl Widget<()> for Leaf {
        type State = ();
        fn create_state(&self) {
            self.0.set(self.0.get() + 1);
        }
        fn producer_budget(&self, phase: WidgetProducerPhase, (): &()) -> WidgetProducerBudget {
            match phase {
                WidgetProducerPhase::Subscriptions => {
                    WidgetProducerBudget::new(0).with_subscriptions(0)
                }
                _ => WidgetProducerBudget::new(0),
            }
        }
    }

    struct OwnedApp;
    impl UiApp for OwnedApp {
        type State = OwnedState;
        type Action = ();
        type HostProtocol = NoHostProtocol;
        fn root(state: &OwnedState) -> impl View<()> {
            Element::new(Leaf(Rc::clone(&state.callbacks)))
        }
        fn initial_producer_budget(_: &OwnedState) -> ProducerBudget {
            ProducerBudget::new(1, 0).with_reconciliation(1, 0, 1)
        }
        fn action_producer_budget(_: &OwnedState, (): &()) -> ProducerBudget {
            ProducerBudget::new(1, 0).with_reconciliation(0, 1, 0)
        }
        fn interaction_producer_budget(_: &OwnedState) -> ProducerBudget {
            ProducerBudget::new(1, 0).with_reconciliation(0, 1, 0)
        }
        fn initial_effects(
            state: &OwnedState,
        ) -> impl runenui_core::IntoEffects<(), NoHostProtocol> {
            state.callbacks.set(state.callbacks.get() + 1);
        }
        fn update(
            _: &mut OwnedState,
            (): (),
        ) -> impl runenui_core::IntoUpdateOutput<(), NoHostProtocol> {
        }
    }

    let callbacks = Rc::new(Cell::new(0));
    let marker = Box::new(73);
    let address = std::ptr::from_ref(marker.as_ref());
    let surface = TestSurfaceConfig::new(LogicalSize::new(
        LogicalLength::from(317_u16),
        LogicalLength::from(239_u16),
    ))
    .unwrap_or_else(|_| unreachable!("positive fixed surface extents are valid"));
    let Err(error) = TestHarness::<OwnedApp>::mount_with_config(
        OwnedState {
            marker,
            callbacks: Rc::clone(&callbacks),
        },
        RuntimeConfig::default().with_queue_capacity(0),
        surface.clone(),
    ) else {
        unreachable!("zero capacity cannot admit the one-owner initial frontier");
    };
    assert_eq!(
        error.admission_error().kind(),
        InitialAdmissionErrorKind::Capacity
    );
    assert_eq!(error.admission_error().config().queue_capacity(), 0);
    assert_eq!(error.surface_config(), &surface);
    assert_eq!(
        std::ptr::from_ref(error.admission_error().state().marker.as_ref()),
        address
    );
    assert_eq!(callbacks.get(), 0);
    let (admission, returned_surface) = error.into_parts();
    let (state, config) = admission.into_parts();
    let mut harness = TestHarness::<OwnedApp>::mount_with_config(
        state,
        config.with_queue_capacity(1),
        returned_surface,
    )
    .unwrap_or_else(|error| unreachable!("the exact one-owner frontier fits: {error}"));
    assert_eq!(std::ptr::from_ref(harness.state().marker.as_ref()), address);
    assert_eq!(harness.surface_config(), &surface);
    assert_eq!(callbacks.get(), 2);
    assert_eq!(
        harness.run_until_idle(settle_budget()).outcome(),
        SettleOutcome::Idle
    );
}

#[test]
fn semantic_query_action_and_replay_use_only_public_authority() {
    let mut harness = TestHarness::<HarnessApp>::mount(0)
        .unwrap_or_else(|error| unreachable!("fixture initial admission must succeed: {error}"));
    assert!(harness.publish().is_ok());

    let query = SemanticQuery::new()
        .with_role(SemanticRole::Button)
        .with_name("Increment")
        .with_supported_action(SemanticAction::Activate);
    let target = harness.unique_semantic_target(&query);
    assert!(target.is_ok());

    if let Ok(target) = target {
        assert!(
            harness
                .submit_semantic_action(&target, SemanticAction::Activate)
                .is_ok()
        );
        let settled = harness.run_until_idle(settle_budget());
        assert_eq!(settled.outcome(), SettleOutcome::Idle);
        assert_eq!(*harness.state(), 1);
    }

    assert!(harness.trace_replay().is_ok());
}

#[test]
fn ambiguous_semantic_queries_preserve_all_matches_without_fallback() {
    let mut harness = TestHarness::<HarnessApp>::mount(0)
        .unwrap_or_else(|error| unreachable!("fixture initial admission must succeed: {error}"));
    assert!(harness.publish().is_ok());

    let matches = harness.query_semantics(&SemanticQuery::new().with_role(SemanticRole::Button));
    assert!(matches.is_ok());
    if let Ok(matches) = matches {
        assert_eq!(matches.len(), 2);
        let unique = matches.unique();
        assert!(matches!(
            unique,
            Err(UniqueSemanticQueryError::Ambiguous { ref matches }) if matches.len() == 2
        ));
    }
}

#[test]
fn settle_is_bounded_and_publication_dirtiness_does_not_force_hidden_work() {
    let mut harness = TestHarness::<HarnessApp>::mount(0)
        .unwrap_or_else(|error| unreachable!("fixture initial admission must succeed: {error}"));
    let settled = harness.run_until_idle(settle_budget());
    assert_eq!(settled.outcome(), SettleOutcome::Idle);
    assert!(settled.last_pump().publication_dirty());
}

#[test]
fn pointer_helper_preserves_context_and_drives_public_pointer_activation() {
    let mut harness = TestHarness::<HarnessApp>::mount(0)
        .unwrap_or_else(|error| unreachable!("fixture initial admission must succeed: {error}"));
    let Some((expected_context, point, expected_target)) = (|| {
        let publication = harness.publish().ok()?;
        let authored = ElementId::new("harness.increment").ok()?;
        let node = publication
            .frame()
            .nodes()
            .iter()
            .find(|node| node.authored_id() == Some(&authored))?;
        let bounds = node.bounds();
        let point = LogicalPoint::new(bounds.x() + 1.0, bounds.y() + 1.0).ok()?;
        assert_eq!(
            publication.hit_test_scene().target_at(point),
            Some(node.id())
        );
        Some((
            publication.input_context().clone(),
            point,
            node.id().clone(),
        ))
    })() else {
        unreachable!("increment hit target must be present in the published scene");
    };

    let Some(pointer_id) = PointerId::new(1) else {
        unreachable!("test pointer identifier must be valid");
    };
    let Ok(down) = harness.pointer_event(
        pointer_id,
        PointerDeviceKind::Mouse,
        PointerPhase::Down,
        point,
    ) else {
        unreachable!("pointer-down event must be constructible");
    };
    assert_eq!(down.surface_context(), &expected_context);
    let down = down
        .with_changed_button(PointerButton::Primary)
        .with_buttons(PointerButtons::new([PointerButton::Primary]));
    let Ok(down_submission) = harness.submit_pointer(down) else {
        unreachable!("pointer-down event must be admitted");
    };
    assert_eq!(
        harness.run_until_idle(settle_budget()).outcome(),
        SettleOutcome::Idle
    );
    assert_eq!(*harness.state(), 0);

    let Ok(up) = harness.pointer_event(
        pointer_id,
        PointerDeviceKind::Mouse,
        PointerPhase::Up,
        point,
    ) else {
        unreachable!("pointer-up event must be constructible");
    };
    let up = up.with_changed_button(PointerButton::Primary);
    let Ok(up_submission) = harness.submit_pointer(up) else {
        unreachable!("pointer-up event must be admitted");
    };
    assert!(down_submission.sequence() < up_submission.sequence());
    assert_eq!(
        harness.run_until_idle(settle_budget()).outcome(),
        SettleOutcome::Idle
    );
    assert_eq!(*harness.state(), 1);

    let Some(publication) = harness.publication() else {
        unreachable!("committed harness publication must remain available");
    };
    assert!(publication.frame().node(&expected_target).is_some());
}
