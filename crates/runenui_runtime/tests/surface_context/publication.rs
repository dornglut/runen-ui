use runenui_core::{CommandOrigin, SemanticCommand, StyleTokens};
use runenui_runtime::{
    AppRuntime, RuntimeConfig, SubmitSurfaceCommandErrorKind, SurfacePublication, TraceConfig,
    TraceRecordKind, TraceSequence, TraceSurfaceIngressKind, TraceSurfaceRejection,
    TraceSurfaceSnapshotKind,
};

use crate::support::{
    SurfaceAction, SurfaceApp, activate_point, authored_center, authored_target, has_rejection,
    mounted, mounted_with, publication, pump_all, rejected,
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

fn initial_redraw_sequence(runtime: &AppRuntime<SurfaceApp>) -> TraceSequence {
    let mounted = runtime
        .trace()
        .records()
        .find(|record| matches!(record.kind(), TraceRecordKind::RuntimeMounted))
        .unwrap_or_else(|| unreachable!("mount trace is retained"));
    let initial_request = runtime
        .trace()
        .records()
        .find(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::RedrawRequested { revision: 1 }
            )
        })
        .unwrap_or_else(|| unreachable!("initial redraw request is retained"));
    assert_eq!(initial_request.causal_parent(), Some(mounted.sequence()));
    assert_eq!(initial_request.instant(), mounted.instant());
    initial_request.sequence()
}

fn publication_sequence(
    runtime: &AppRuntime<SurfaceApp>,
    publication: &SurfacePublication,
    causal_parent: TraceSequence,
    after: Option<TraceSequence>,
) -> TraceSequence {
    let published = runtime
        .trace()
        .records()
        .filter(|record| {
            matches!(record.kind(), TraceRecordKind::SurfacePublished)
                && after.is_none_or(|sequence| record.sequence() > sequence)
        })
        .last()
        .unwrap_or_else(|| unreachable!("surface publication is retained"));
    assert_eq!(published.causal_parent(), Some(causal_parent));
    let context = published
        .context()
        .publication()
        .unwrap_or_else(|| unreachable!("publication owns exact context"));
    assert_eq!(
        context.surface().surface_id(),
        publication.input_context().surface_id()
    );
    assert_eq!(
        context.surface().hit_test_generation(),
        publication.input_context().hit_test_generation()
    );
    assert_eq!(
        context.surface().coordinate_revision(),
        publication.input_context().coordinate_revision()
    );
    assert_eq!(
        context.surface().snapshot(),
        Some(TraceSurfaceSnapshotKind::Current)
    );
    assert_eq!(
        context.reconciliation_generation(),
        runtime.reconciliation_report().generation()
    );
    assert_eq!(context.node_count(), publication.frame().nodes().len());
    assert_eq!(
        context.executed_phases(),
        runtime.last_surface_phase_report().executed()
    );
    published.sequence()
}

fn update_redraw_sequence(runtime: &AppRuntime<SurfaceApp>, after: TraceSequence) -> TraceSequence {
    let reconciled = runtime
        .trace()
        .records()
        .filter(|record| matches!(record.kind(), TraceRecordKind::TreeReconciled))
        .last()
        .unwrap_or_else(|| unreachable!("update reconciliation is retained"));
    let redraw = runtime
        .trace()
        .records()
        .filter(|record| {
            matches!(record.kind(), TraceRecordKind::RedrawRequested { .. })
                && record.sequence() > after
        })
        .last()
        .unwrap_or_else(|| unreachable!("update redraw request is retained"));
    assert_eq!(redraw.causal_parent(), Some(reconciled.sequence()));
    assert_eq!(redraw.instant(), reconciled.instant());
    redraw.sequence()
}

#[test]
fn publication_retains_initial_and_update_redraw_causality() {
    let mut runtime = mounted();
    let tokens = StyleTokens::new();
    let initial_request = initial_redraw_sequence(&runtime);

    let first = publication(&mut runtime, &tokens);
    let first_published = publication_sequence(&runtime, &first, initial_request, None);

    runtime
        .submit_action(SurfaceAction::Swap)
        .unwrap_or_else(|_| unreachable!("swap action is accepted"));
    pump_all(&mut runtime);
    let update_request = update_redraw_sequence(&runtime, first_published);

    let second = publication(&mut runtime, &tokens);
    let _ = publication_sequence(&runtime, &second, update_request, Some(first_published));
}

#[test]
fn capacity_zero_does_not_block_publication_or_allocate_trace_context() {
    let config = RuntimeConfig::default().with_trace_config(TraceConfig::new(0));
    let mut runtime = mounted_with(config);
    let tokens = StyleTokens::new();

    let first = publication(&mut runtime, &tokens);
    runtime
        .submit_action(SurfaceAction::Swap)
        .unwrap_or_else(|_| unreachable!("trace-disabled update is accepted"));
    pump_all(&mut runtime);
    let second = publication(&mut runtime, &tokens);

    assert_eq!(first.input_context().hit_test_generation(), 1);
    assert_eq!(second.input_context().hit_test_generation(), 2);
    assert!(runtime.trace().is_empty());
}

#[cfg(feature = "internal-test-seams")]
#[test]
fn publication_consumes_and_replenishes_one_private_trace_reservation() {
    let mut runtime = mounted();
    let tokens = StyleTokens::new();

    assert!(runtime.__surface_publication_trace_reserved_for_test());
    assert_eq!(runtime.__routed_trace_reservations_for_test(), 0);
    let _ = publication(&mut runtime, &tokens);
    assert!(runtime.__surface_publication_trace_reserved_for_test());
    assert_eq!(runtime.__routed_trace_reservations_for_test(), 0);
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

    activate_point(&mut runtime, old_context.clone(), old_point);
    assert_eq!(runtime.state().primary_activations, 1);
    assert_eq!(runtime.state().secondary_activations, 0);
    let retained = runtime
        .trace()
        .records()
        .filter(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::SurfaceContextAccepted {
                    ingress: TraceSurfaceIngressKind::LogicalCoordinate,
                }
            ) && record.context().surface().is_some_and(|surface| {
                surface.surface_id() == old_context.surface_id()
                    && surface.hit_test_generation() == old_context.hit_test_generation()
                    && surface.coordinate_revision() == old_context.coordinate_revision()
                    && surface.snapshot() == Some(TraceSurfaceSnapshotKind::RetainedHistorical)
            })
        })
        .last()
        .unwrap_or_else(|| unreachable!("retained accepted context is traced exactly"));
    assert!(retained.instant().is_some());

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
    let requested_context = old_context.clone();
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
    let rejection = runtime
        .trace()
        .records()
        .filter(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::SurfaceCommandRejected {
                    ingress: TraceSurfaceIngressKind::LogicalCoordinate,
                    outcome: TraceSurfaceRejection::RetiredGeneration,
                }
            )
        })
        .last()
        .unwrap_or_else(|| unreachable!("retired rejection is traced"));
    let surface = rejection
        .context()
        .surface()
        .unwrap_or_else(|| unreachable!("rejection owns submitted surface context"));
    assert_eq!(surface.surface_id(), requested_context.surface_id());
    assert_eq!(
        surface.hit_test_generation(),
        requested_context.hit_test_generation()
    );
    assert_eq!(
        surface.coordinate_revision(),
        requested_context.coordinate_revision()
    );
    assert_eq!(surface.snapshot(), None);
    assert!(rejection.instant().is_some());

    pump_all(&mut runtime);
    assert_eq!(runtime.state().primary_activations, 1);
    assert_eq!(runtime.state().secondary_activations, 0);
}
