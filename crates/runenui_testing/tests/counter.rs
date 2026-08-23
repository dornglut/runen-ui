use core::num::NonZeroUsize;

use runenui_core::{
    ElementId, PointerButton, PointerButtons, PointerDeviceKind, PointerId, PointerPhase,
    SemanticAction, SemanticRole,
};
use runenui_runtime::{LogicalPoint, PumpBudget, SemanticUpdateResult, TraceRecordKind};
use runenui_testing::{SemanticQuery, SettleBudget, SettleOutcome, TestHarness};

#[path = "../../../examples/counter/src/app.rs"]
mod app;
#[path = "../../../examples/counter/src/ui.rs"]
mod ui;

use app::{Counter, CounterApp};

fn settle_budget() -> SettleBudget {
    SettleBudget::new(
        NonZeroUsize::new(8).unwrap_or(NonZeroUsize::MIN),
        PumpBudget::new(64, 64, 64, 64),
    )
}

#[test]
fn real_counter_uses_public_semantic_query_action_publication_and_replay() {
    let mut harness = TestHarness::<CounterApp>::mount(Counter::new());
    assert!(harness.publish().is_ok());

    let Ok(snapshot) = harness.semantic_snapshot() else {
        return;
    };
    let first_surface = snapshot.surface_id().clone();
    let first_revision = snapshot.revision();

    let increment = SemanticQuery::new()
        .with_role(SemanticRole::Button)
        .with_name("+")
        .with_supported_action(SemanticAction::Activate);
    let Ok(target) = harness.unique_semantic_target(&increment) else {
        return;
    };
    assert_eq!(target.surface_id(), &first_surface);
    assert!(
        harness
            .submit_semantic_action(&target, SemanticAction::Activate)
            .is_ok()
    );
    assert_eq!(
        harness.run_until_idle(settle_budget()).outcome(),
        SettleOutcome::Idle
    );
    assert_eq!(harness.state().count, 1);

    assert!(harness.publish().is_ok());
    let Some(publication) = harness.publication() else {
        return;
    };
    assert!(!publication.frame().nodes().is_empty());
    assert!(!publication.layout_report().nodes().is_empty());
    assert!(
        !publication
            .semantic_publication()
            .snapshot()
            .nodes()
            .is_empty()
    );

    assert!(matches!(
        harness.semantic_update_from(&first_surface, first_revision),
        Ok(SemanticUpdateResult::Delta(_))
    ));

    let mut foreign = TestHarness::<CounterApp>::mount(Counter::new());
    assert!(foreign.publish().is_ok());
    let Ok(foreign_snapshot) = foreign.semantic_snapshot() else {
        return;
    };
    assert!(matches!(
        harness.semantic_update_from(foreign_snapshot.surface_id(), first_revision),
        Ok(SemanticUpdateResult::FullResync(_))
    ));

    assert!(!harness.trace_jsonl().is_empty());
    assert!(harness.trace_replay().is_ok());
}

#[test]
fn harness_point_input_converges_on_the_latest_public_scene_and_context() {
    let mut harness = TestHarness::<CounterApp>::mount(Counter::new());
    let publication = harness
        .publish()
        .unwrap_or_else(|_| unreachable!("counter publication is admitted"))
        .clone();

    let authored = ElementId::new("counter.increment")
        .unwrap_or_else(|_| unreachable!("fixture element id is valid"));
    let button = publication
        .frame()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored))
        .unwrap_or_else(|| unreachable!("counter increment button is published"));
    let bounds = button.bounds();
    let input_point = LogicalPoint::new(
        bounds.x() + bounds.width() / 2.0,
        bounds.y() + bounds.height() / 2.0,
    )
    .unwrap_or_else(|_| unreachable!("published button center is finite"));

    let retained = harness
        .publication()
        .unwrap_or_else(|| unreachable!("harness retains the accepted publication"));
    assert_eq!(
        retained.paint_publication(),
        publication.paint_publication()
    );
    assert_eq!(retained.hit_test_scene(), publication.hit_test_scene());
    assert_eq!(
        harness
            .input_context()
            .unwrap_or_else(|_| unreachable!("publication exposes exact input context")),
        publication.input_context()
    );

    let expected_target = publication
        .hit_test_scene()
        .target_at(input_point)
        .cloned()
        .unwrap_or_else(|| unreachable!("button center resolves through the public hit scene"));
    assert_eq!(&expected_target, button.id());
    assert!(
        publication
            .hit_test_scene()
            .contains_mounted_target(&expected_target)
    );

    let pointer = harness
        .pointer_event(
            PointerId::new(1).unwrap_or_else(|| unreachable!("pointer id is non-zero")),
            PointerDeviceKind::Mouse,
            PointerPhase::Down,
            input_point,
        )
        .unwrap_or_else(|_| unreachable!("harness derives input from the latest public context"))
        .with_changed_button(PointerButton::Primary)
        .with_buttons(PointerButtons::new([PointerButton::Primary]));
    assert_eq!(pointer.surface_context(), publication.input_context());

    let sequence = harness
        .submit_pointer(pointer)
        .unwrap_or_else(|_| unreachable!("exact-context pointer is admitted"))
        .sequence();
    assert_eq!(
        harness.run_until_idle(settle_budget()).outcome(),
        SettleOutcome::Idle
    );
    let resolved = harness
        .trace()
        .records()
        .find(|record| {
            record.work_sequence() == Some(sequence)
                && matches!(
                    record.kind(),
                    TraceRecordKind::PointerPhysicalTargetResolved
                )
        })
        .unwrap_or_else(|| unreachable!("runtime traces public-scene physical target resolution"));
    assert_eq!(
        resolved
            .target()
            .map(runenui_runtime::TraceTarget::mounted_node_id),
        Some(&expected_target)
    );
}
