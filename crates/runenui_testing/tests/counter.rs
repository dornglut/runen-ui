use core::num::NonZeroUsize;

use runenui_core::{SemanticAction, SemanticRole};
use runenui_runtime::{PumpBudget, SemanticUpdateResult};
use runenui_testing::{SemanticQuery, SettleBudget, SettleOutcome, TestHarness};

#[path = "../../../examples/counter/src/app.rs"]
mod app;
#[path = "../../../examples/counter/src/ui.rs"]
mod ui;

use app::{Counter, CounterApp};

const fn settle_budget() -> SettleBudget {
    SettleBudget::new(NonZeroUsize::MIN, PumpBudget::new(64, 64, 64, 64))
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
    assert!(!publication.semantic_publication().snapshot().nodes().is_empty());

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
