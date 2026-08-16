use core::num::NonZeroUsize;

use runenui_core::{
    IntoEffects, NoHostProtocol, SemanticAction, SemanticRole, UiApp, View, button, children,
    column,
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
    type State = i32;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(_: &Self::State) -> impl View<Self::Action> {
        column(children![
            button("Increment").on_activate(|| Action::Increment),
            button("Reset").on_activate(|| Action::Reset),
        ])
    }

    fn update(
        state: &mut Self::State,
        action: Self::Action,
    ) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        match action {
            Action::Increment => *state += 1,
            Action::Reset => *state = 0,
        }
    }
}

const fn settle_budget() -> SettleBudget {
    let iterations = NonZeroUsize::new(8).map_or(NonZeroUsize::MIN, |iterations| iterations);
    SettleBudget::new(iterations, PumpBudget::new(64, 64, 64, 64))
}

#[test]
fn semantic_query_action_and_replay_use_only_public_authority() {
    let mut harness = TestHarness::<HarnessApp>::mount(0);
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
    let mut harness = TestHarness::<HarnessApp>::mount(0);
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
    let mut harness = TestHarness::<HarnessApp>::mount(0);
    let settled = harness.run_until_idle(settle_budget());
    assert_eq!(settled.outcome(), SettleOutcome::Idle);
    assert!(settled.last_pump().publication_dirty());
}

#[test]
fn pointer_helper_uses_the_exact_current_public_surface_context() {
    let mut harness = TestHarness::<HarnessApp>::mount(0);
    let publication = harness.publish();
    assert!(publication.is_ok());

    let Some(pointer_id) = PointerId::new(1) else {
        return;
    };
    let Ok(point) = LogicalPoint::new(4.0, 4.0) else {
        return;
    };
    let event = harness.pointer_event(
        pointer_id,
        PointerDeviceKind::Mouse,
        PointerPhase::Move,
        point,
    );
    assert!(event.is_ok());
    if let (Ok(publication), Ok(event)) = (publication, event) {
        assert_eq!(
            event.surface_context(),
            publication.input_context(),
            "pointer helper must preserve the exact public displayed context"
        );
    }
}
