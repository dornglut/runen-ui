use core::num::NonZeroUsize;

use runenui_core::{
    ElementId, IntoEffects, NoHostProtocol, PointerButton, PointerButtons, SemanticAction,
    SemanticRole, UiApp, View, button, children, column,
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
            button("Increment")
                .id("harness.increment")
                .on_activate(|| Action::Increment),
            button("Reset")
                .id("harness.reset")
                .on_activate(|| Action::Reset),
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

fn settle_budget() -> SettleBudget {
    let iterations = NonZeroUsize::new(8).unwrap_or(NonZeroUsize::MIN);
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
fn pointer_helper_preserves_context_and_drives_public_pointer_activation() {
    let mut harness = TestHarness::<HarnessApp>::mount(0);
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
        return;
    };

    let Some(pointer_id) = PointerId::new(1) else {
        return;
    };
    let Ok(down) = harness.pointer_event(
        pointer_id,
        PointerDeviceKind::Mouse,
        PointerPhase::Down,
        point,
    ) else {
        return;
    };
    assert_eq!(down.surface_context(), &expected_context);
    let down = down
        .with_changed_button(PointerButton::Primary)
        .with_buttons(PointerButtons::new([PointerButton::Primary]));
    let Ok(down_submission) = harness.submit_pointer(down) else {
        return;
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
        return;
    };
    let up = up.with_changed_button(PointerButton::Primary);
    let Ok(up_submission) = harness.submit_pointer(up) else {
        return;
    };
    assert!(down_submission.sequence() < up_submission.sequence());
    assert_eq!(
        harness.run_until_idle(settle_budget()).outcome(),
        SettleOutcome::Idle
    );
    assert_eq!(*harness.state(), 1);

    let Some(publication) = harness.publication() else {
        return;
    };
    assert!(publication.frame().node(&expected_target).is_some());
}
