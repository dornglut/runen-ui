use runenui_core::{CommandOrigin, SemanticCommand, StyleTokens};
use runenui_runtime::{
    SubmitSurfaceCommandErrorKind, TraceRecordKind, TraceSurfaceIngressKind, TraceSurfaceRejection,
    TraceSurfaceSnapshotKind,
};

use crate::support::{
    SurfaceAction, activate_point, authored_center, authored_target, has_rejection, mounted,
    publication, pump_all, rejected,
};

#[test]
fn every_publication_issues_a_fresh_context_for_one_surface() {
    let mut runtime = mounted();
    let tokens = StyleTokens::new();
    let first = publication(&mut runtime, &tokens);
    let second = publication(&mut runtime, &tokens);

    assert_eq!(
        first.input_context().surface_id(),
        second.input_context().surface_id()
    );
    assert_ne!(first.input_context(), second.input_context());
    assert_eq!(first.input_context().hit_test_generation(), 1);
    assert_eq!(second.input_context().hit_test_generation(), 2);
    assert_eq!(first.input_context().coordinate_revision(), 1);
    assert_eq!(second.input_context().coordinate_revision(), 2);
}

#[test]
fn current_and_retained_contexts_use_their_exact_geometry() {
    let mut runtime = mounted();
    let tokens = StyleTokens::new();
    let old = publication(&mut runtime, &tokens);
    let old_context = old.input_context().clone();
    let old_point = authored_center(&old, "surface.primary");
    let primary = authored_target(&old, "surface.primary");

    runtime
        .submit_action(SurfaceAction::Swap)
        .unwrap_or_else(|_| unreachable!("swap action is accepted"));
    pump_all(&mut runtime);
    let current = publication(&mut runtime, &tokens);
    let current_context = current.input_context().clone();
    let secondary = authored_target(&current, "surface.secondary");

    assert_eq!(old.frame().hit_test_id(old_point), Some(primary));
    assert_eq!(current.frame().hit_test_id(old_point), Some(secondary));

    activate_point(&mut runtime, old_context, old_point);
    assert_eq!(runtime.state().primary_activations, 1);
    assert_eq!(runtime.state().secondary_activations, 0);
    assert!(runtime.trace().kinds().any(|kind| {
        matches!(
            kind,
            TraceRecordKind::SurfaceContextAccepted {
                ingress: TraceSurfaceIngressKind::LogicalCoordinate,
                snapshot: TraceSurfaceSnapshotKind::RetainedHistorical,
                ..
            }
        )
    }));

    activate_point(&mut runtime, current_context, old_point);
    assert_eq!(runtime.state().primary_activations, 1);
    assert_eq!(runtime.state().secondary_activations, 1);
}

#[test]
fn accepted_historical_target_stays_bound_after_context_retirement() {
    let mut runtime = mounted();
    let tokens = StyleTokens::new();
    let old = publication(&mut runtime, &tokens);
    let old_context = old.input_context().clone();
    let old_point = authored_center(&old, "surface.primary");

    runtime
        .submit_action(SurfaceAction::Swap)
        .unwrap_or_else(|_| unreachable!("swap action is accepted"));
    pump_all(&mut runtime);
    let _ = publication(&mut runtime, &tokens);

    runtime
        .submit_surface_command(
            old_context.clone(),
            old_point,
            SemanticCommand::Activate,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("retained historical context is accepted"));

    let _ = publication(&mut runtime, &tokens);
    let retired = rejected(
        runtime.submit_surface_command(
            old_context,
            old_point,
            SemanticCommand::Activate,
            CommandOrigin::programmatic(),
        ),
        "expected retired surface context",
    );
    assert_eq!(
        retired.kind(),
        SubmitSurfaceCommandErrorKind::RetiredSurfaceContext
    );
    assert!(has_rejection(
        &runtime,
        TraceSurfaceIngressKind::LogicalCoordinate,
        TraceSurfaceRejection::RetiredGeneration,
    ));

    pump_all(&mut runtime);
    assert_eq!(runtime.state().primary_activations, 1);
    assert_eq!(runtime.state().secondary_activations, 0);
}
