use runenui_core::{CommandOrigin, SemanticCommand, StyleTokens};
use runenui_runtime::{TraceRecordKind, TraceSurfaceIngressKind, TraceSurfaceSnapshotKind};

use crate::support::{
    activate_target, authored_center, authored_target, mounted, publication, pump_all, trace_record,
};

#[test]
fn resolved_target_ingress_uses_the_canonical_route_update_and_trace_path() {
    let mut runtime = mounted();
    let tokens = StyleTokens::new();
    let published = publication(&mut runtime, &tokens);
    let target = authored_target(&published, "surface.primary");

    activate_target(&mut runtime, published.input_context().clone(), target);

    assert_eq!(runtime.state().primary_activations, 1);
    assert!(runtime.trace().kinds().any(|kind| {
        matches!(
            kind,
            TraceRecordKind::SurfaceContextAccepted {
                ingress: TraceSurfaceIngressKind::ResolvedTarget,
                ..
            }
        )
    }));
}

#[test]
fn successful_surface_trace_has_one_causal_chain() {
    let mut runtime = mounted();
    let tokens = StyleTokens::new();
    let published = publication(&mut runtime, &tokens);
    let point = authored_center(&published, "surface.primary");
    let generation = published.input_context().hit_test_generation();
    let revision = published.input_context().coordinate_revision();

    let submission = runtime
        .submit_surface_command(
            published.input_context().clone(),
            point,
            SemanticCommand::Activate,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("current surface command is accepted"));
    let sequence = submission.sequence();

    let context = trace_record(&runtime, sequence, |kind| {
        matches!(
            kind,
            TraceRecordKind::SurfaceContextAccepted {
                ingress: TraceSurfaceIngressKind::LogicalCoordinate,
                snapshot: TraceSurfaceSnapshotKind::Current,
                hit_test_generation,
                coordinate_revision,
            } if *hit_test_generation == generation && *coordinate_revision == revision
        )
    });
    let target = trace_record(&runtime, sequence, |kind| {
        matches!(
            kind,
            TraceRecordKind::SurfaceTargetBound {
                ingress: TraceSurfaceIngressKind::LogicalCoordinate,
                hit_test_generation,
            } if *hit_test_generation == generation
        )
    });
    let accepted = trace_record(&runtime, sequence, |kind| {
        matches!(kind, TraceRecordKind::CommandSubmissionAccepted)
    });

    assert_eq!(target.causal_parent(), Some(context.sequence()));
    assert_eq!(accepted.causal_parent(), Some(target.sequence()));
    let accepted_sequence = accepted.sequence();

    pump_all(&mut runtime);
    let routed = trace_record(&runtime, sequence, |kind| {
        matches!(kind, TraceRecordKind::RoutedEventStarted)
    });
    assert_eq!(routed.causal_parent(), Some(accepted_sequence));
}
