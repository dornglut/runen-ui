use runenui_core::{CommandOrigin, SemanticCommand, StyleEnvironment};
use runenui_runtime::{TraceRecordKind, TraceSurfaceIngressKind, TraceSurfaceSnapshotKind};

use crate::support::{
    activate_target, authored_center, authored_target, mounted, publication, pump_all, trace_record,
};

#[test]
fn resolved_target_ingress_uses_the_canonical_route_update_and_trace_path() {
    let mut runtime = mounted();
    let style_environment = StyleEnvironment::default();
    let published = publication(&mut runtime, &style_environment);
    let target = authored_target(&published, "surface.primary");

    activate_target(&mut runtime, published.input_context().clone(), target);

    assert_eq!(runtime.state().primary_activations, 1);
    assert!(runtime.trace().kinds().any(|kind| {
        matches!(
            kind,
            TraceRecordKind::SurfaceContextAccepted {
                ingress: TraceSurfaceIngressKind::ResolvedTarget,
            }
        )
    }));
}

#[test]
fn successful_surface_trace_has_one_causal_chain() {
    let mut runtime = mounted();
    let style_environment = StyleEnvironment::default();
    let published = publication(&mut runtime, &style_environment);
    let requested_context = published.input_context().clone();
    let point = authored_center(&published, "surface.primary");
    let generation = requested_context.hit_test_generation();
    let revision = requested_context.coordinate_revision();

    let submission = runtime
        .submit_surface_command(
            requested_context.clone(),
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
            }
        )
    });
    let target = trace_record(&runtime, sequence, |kind| {
        matches!(kind, TraceRecordKind::SurfaceTargetBound)
    });
    let accepted = trace_record(&runtime, sequence, |kind| {
        matches!(kind, TraceRecordKind::CommandSubmissionAccepted)
    });

    let surface = context
        .context()
        .surface()
        .unwrap_or_else(|| unreachable!("accepted record owns exact surface context"));
    assert_eq!(surface.surface_id(), requested_context.surface_id());
    assert_eq!(surface.hit_test_generation(), generation);
    assert_eq!(surface.coordinate_revision(), revision);
    assert_eq!(surface.snapshot(), Some(TraceSurfaceSnapshotKind::Current));
    assert_eq!(context.work_sequence(), Some(sequence));
    assert_eq!(target.work_sequence(), Some(sequence));
    assert_eq!(accepted.work_sequence(), Some(sequence));
    assert_eq!(context.instant(), target.instant());
    assert_eq!(context.instant(), accepted.instant());
    assert_eq!(target.causal_parent(), Some(context.sequence()));
    assert_eq!(accepted.causal_parent(), Some(target.sequence()));
    let accepted_sequence = accepted.sequence();

    pump_all(&mut runtime);
    let routed = trace_record(&runtime, sequence, |kind| {
        matches!(kind, TraceRecordKind::RoutedEventStarted)
    });
    assert_eq!(routed.causal_parent(), Some(accepted_sequence));
}
