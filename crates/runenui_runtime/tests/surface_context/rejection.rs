use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use runenui_core::{CommandOrigin, SemanticCommand, StyleTokens};
use runenui_runtime::{
    LogicalPoint, RuntimeConfig, RuntimeTerminalReason, SubmitSurfaceCommandErrorKind,
    TraceSurfaceIngressKind, TraceSurfaceRejection,
};

use crate::support::{
    SurfaceAction, authored_center, authored_target, has_rejection, mounted, mounted_target,
    mounted_with, publication, pump_all, rejected,
};

const QUEUE_CAPACITY: usize = 16;

#[test]
fn rejected_no_hit_request_recovers_inputs_without_queue_state_or_wake_effect() {
    let mut runtime = mounted();
    let tokens = StyleTokens::new();
    let published = publication(&mut runtime, &tokens);
    let context = published.input_context().clone();
    let point = LogicalPoint::new(-1.0, -1.0).unwrap_or_else(|_| unreachable!());
    let origin = CommandOrigin::programmatic();
    let before_state = runtime.state().clone();
    let before_work_sequence = runtime.__routed_sequence_state_for_test().0;
    let wakes = Arc::new(AtomicUsize::new(0));
    let wake_count = Arc::clone(&wakes);
    runtime.set_wake_transport(move || {
        wake_count.fetch_add(1, Ordering::SeqCst);
    });

    let error = rejected(
        runtime.submit_surface_command(context.clone(), point, SemanticCommand::Activate, origin),
        "expected no-target rejection",
    );

    assert_eq!(error.kind(), SubmitSurfaceCommandErrorKind::NoTarget);
    assert_eq!(error.unaccepted().context(), &context);
    assert_eq!(error.unaccepted().point(), Some(point));
    assert_eq!(error.unaccepted().target(), None);
    assert_eq!(error.unaccepted().command(), SemanticCommand::Activate);
    assert_eq!(error.unaccepted().origin(), origin);
    assert_eq!(runtime.state(), &before_state);
    assert_eq!(
        runtime.__routed_sequence_state_for_test().0,
        before_work_sequence
    );
    assert_eq!(runtime.__routed_trace_reservations_for_test(), 0);
    assert_eq!(wakes.load(Ordering::SeqCst), 0);
    assert!(has_rejection(
        &runtime,
        TraceSurfaceIngressKind::LogicalCoordinate,
        TraceSurfaceRejection::NoTarget,
    ));
}

#[test]
fn target_absent_from_named_snapshot_is_not_retargeted() {
    let mut runtime = mounted();
    let tokens = StyleTokens::new();
    let old = publication(&mut runtime, &tokens);
    let old_context = old.input_context().clone();

    runtime
        .submit_action(SurfaceAction::ShowExtra)
        .unwrap_or_else(|_| unreachable!("show-extra action is accepted"));
    pump_all(&mut runtime);
    let extra = mounted_target(&mut runtime, "surface.extra");

    let error = rejected(
        runtime.submit_resolved_surface_command(
            old_context,
            extra,
            SemanticCommand::Activate,
            CommandOrigin::programmatic(),
        ),
        "expected snapshot-membership rejection",
    );

    assert_eq!(
        error.kind(),
        SubmitSurfaceCommandErrorKind::TargetNotInSnapshot
    );
    assert_eq!(runtime.state().extra_activations, 0);
    assert!(has_rejection(
        &runtime,
        TraceSurfaceIngressKind::ResolvedTarget,
        TraceSurfaceRejection::TargetNotInSnapshot,
    ));
}

#[test]
fn stale_target_is_rejected_after_snapshot_membership_succeeds() {
    let mut runtime = mounted();
    let tokens = StyleTokens::new();
    let published = publication(&mut runtime, &tokens);
    let context = published.input_context().clone();
    let primary = authored_target(&published, "surface.primary");

    runtime
        .submit_action(SurfaceAction::HidePrimary)
        .unwrap_or_else(|_| unreachable!("hide-primary action is accepted"));
    pump_all(&mut runtime);

    let error = rejected(
        runtime.submit_resolved_surface_command(
            context,
            primary,
            SemanticCommand::Activate,
            CommandOrigin::programmatic(),
        ),
        "expected stale-target rejection",
    );

    assert_eq!(error.kind(), SubmitSurfaceCommandErrorKind::StaleTarget);
    assert_eq!(runtime.state().primary_activations, 0);
    assert!(has_rejection(
        &runtime,
        TraceSurfaceIngressKind::ResolvedTarget,
        TraceSurfaceRejection::StaleTarget,
    ));
}

#[test]
fn foreign_and_missing_targets_are_classified_after_snapshot_membership() {
    let mut runtime = mounted();
    let mut foreign_runtime = mounted();
    let tokens = StyleTokens::new();

    let local = publication(&mut runtime, &tokens);
    let local_context = local.input_context().clone();
    let local_primary = authored_target(&local, "surface.primary");
    let foreign = publication(&mut foreign_runtime, &tokens);
    let foreign_primary = authored_target(&foreign, "surface.primary");
    runtime.__replace_surface_snapshot_target_for_test(
        &local_context,
        &local_primary,
        foreign_primary.clone(),
    );

    let foreign_error = rejected(
        runtime.submit_resolved_surface_command(
            local_context,
            foreign_primary,
            SemanticCommand::Activate,
            CommandOrigin::programmatic(),
        ),
        "expected foreign-target rejection",
    );
    assert_eq!(
        foreign_error.kind(),
        SubmitSurfaceCommandErrorKind::ForeignTarget
    );
    assert!(has_rejection(
        &runtime,
        TraceSurfaceIngressKind::ResolvedTarget,
        TraceSurfaceRejection::ForeignTarget,
    ));

    let mut missing_runtime = mounted();
    let missing_publication = publication(&mut missing_runtime, &tokens);
    let missing_context = missing_publication.input_context().clone();
    let original = authored_target(&missing_publication, "surface.primary");
    let missing = missing_runtime.__missing_target_for_test();
    missing_runtime.__replace_surface_snapshot_target_for_test(
        &missing_context,
        &original,
        missing.clone(),
    );
    let missing_error = rejected(
        missing_runtime.submit_resolved_surface_command(
            missing_context,
            missing,
            SemanticCommand::Activate,
            CommandOrigin::programmatic(),
        ),
        "expected missing-target rejection",
    );
    assert_eq!(
        missing_error.kind(),
        SubmitSurfaceCommandErrorKind::MissingTarget
    );
    assert!(has_rejection(
        &missing_runtime,
        TraceSurfaceIngressKind::ResolvedTarget,
        TraceSurfaceRejection::MissingTarget,
    ));
}

#[test]
fn logical_context_dimensions_have_distinct_outcomes_and_trace() {
    let mut runtime = mounted();
    let mut foreign_runtime = mounted();
    let tokens = StyleTokens::new();
    let published = publication(&mut runtime, &tokens);
    let foreign = publication(&mut foreign_runtime, &tokens);
    let point = authored_center(&published, "surface.primary");
    let current = published.input_context();
    let command = SemanticCommand::Activate;
    let origin = CommandOrigin::programmatic();

    let foreign_runtime_error = rejected(
        runtime.submit_surface_command(foreign.input_context().clone(), point, command, origin),
        "expected foreign-runtime rejection",
    );
    assert_eq!(
        foreign_runtime_error.kind(),
        SubmitSurfaceCommandErrorKind::ForeignSurfaceContext
    );
    assert!(has_rejection(
        &runtime,
        TraceSurfaceIngressKind::LogicalCoordinate,
        TraceSurfaceRejection::ForeignRuntime,
    ));

    let foreign_surface = runtime.__surface_context_for_test(
        1,
        1,
        current.coordinate_revision(),
        current.hit_test_generation(),
    );
    let foreign_surface_error = rejected(
        runtime.submit_surface_command(foreign_surface, point, command, origin),
        "expected foreign-surface rejection",
    );
    assert_eq!(
        foreign_surface_error.kind(),
        SubmitSurfaceCommandErrorKind::ForeignSurface
    );
    assert!(has_rejection(
        &runtime,
        TraceSurfaceIngressKind::LogicalCoordinate,
        TraceSurfaceRejection::ForeignSurface,
    ));

    let missing_generation = runtime.__surface_context_for_test(
        0,
        1,
        current.coordinate_revision(),
        current.hit_test_generation().saturating_add(100),
    );
    let missing_error = rejected(
        runtime.submit_surface_command(missing_generation, point, command, origin),
        "expected missing-generation rejection",
    );
    assert_eq!(
        missing_error.kind(),
        SubmitSurfaceCommandErrorKind::MissingSurfaceGeneration
    );
    assert!(has_rejection(
        &runtime,
        TraceSurfaceIngressKind::LogicalCoordinate,
        TraceSurfaceRejection::MissingGeneration,
    ));

    let mismatched_revision = runtime.__surface_context_for_test(
        0,
        1,
        current.coordinate_revision().saturating_add(1),
        current.hit_test_generation(),
    );
    let mismatch_error = rejected(
        runtime.submit_surface_command(mismatched_revision, point, command, origin),
        "expected coordinate-revision rejection",
    );
    assert_eq!(
        mismatch_error.kind(),
        SubmitSurfaceCommandErrorKind::CoordinateRevisionMismatch
    );
    assert!(has_rejection(
        &runtime,
        TraceSurfaceIngressKind::LogicalCoordinate,
        TraceSurfaceRejection::CoordinateRevisionMismatch,
    ));
    assert_eq!(runtime.state().primary_activations, 0);
}

#[test]
fn resolved_target_reuses_every_context_validation_dimension() {
    let mut runtime = mounted();
    let mut foreign_runtime = mounted();
    let tokens = StyleTokens::new();
    let current = publication(&mut runtime, &tokens);
    let foreign = publication(&mut foreign_runtime, &tokens);
    let target = authored_target(&current, "surface.primary");
    let context = current.input_context();
    let command = SemanticCommand::Activate;
    let origin = CommandOrigin::programmatic();

    let foreign_runtime_error = rejected(
        runtime.submit_resolved_surface_command(
            foreign.input_context().clone(),
            target.clone(),
            command,
            origin,
        ),
        "expected foreign-runtime rejection",
    );
    assert_eq!(
        foreign_runtime_error.kind(),
        SubmitSurfaceCommandErrorKind::ForeignSurfaceContext
    );
    assert!(has_rejection(
        &runtime,
        TraceSurfaceIngressKind::ResolvedTarget,
        TraceSurfaceRejection::ForeignRuntime,
    ));

    let foreign_surface = runtime.__surface_context_for_test(
        1,
        1,
        context.coordinate_revision(),
        context.hit_test_generation(),
    );
    let foreign_surface_error = rejected(
        runtime.submit_resolved_surface_command(foreign_surface, target.clone(), command, origin),
        "expected foreign-surface rejection",
    );
    assert_eq!(
        foreign_surface_error.kind(),
        SubmitSurfaceCommandErrorKind::ForeignSurface
    );
    assert!(has_rejection(
        &runtime,
        TraceSurfaceIngressKind::ResolvedTarget,
        TraceSurfaceRejection::ForeignSurface,
    ));

    let missing_generation = runtime.__surface_context_for_test(
        0,
        1,
        context.coordinate_revision(),
        context.hit_test_generation().saturating_add(100),
    );
    let missing_error = rejected(
        runtime.submit_resolved_surface_command(
            missing_generation,
            target.clone(),
            command,
            origin,
        ),
        "expected missing-generation rejection",
    );
    assert_eq!(
        missing_error.kind(),
        SubmitSurfaceCommandErrorKind::MissingSurfaceGeneration
    );
    assert!(has_rejection(
        &runtime,
        TraceSurfaceIngressKind::ResolvedTarget,
        TraceSurfaceRejection::MissingGeneration,
    ));

    let revision_mismatch = runtime.__surface_context_for_test(
        0,
        1,
        context.coordinate_revision().saturating_add(1),
        context.hit_test_generation(),
    );
    let revision_error = rejected(
        runtime.submit_resolved_surface_command(revision_mismatch, target, command, origin),
        "expected coordinate-revision rejection",
    );
    assert_eq!(
        revision_error.kind(),
        SubmitSurfaceCommandErrorKind::CoordinateRevisionMismatch
    );
    assert!(has_rejection(
        &runtime,
        TraceSurfaceIngressKind::ResolvedTarget,
        TraceSurfaceRejection::CoordinateRevisionMismatch,
    ));
}

#[test]
fn queue_closed_work_and_trace_capacity_failures_remain_structured() {
    let tokens = StyleTokens::new();

    let mut full = mounted_with(RuntimeConfig::default().with_queue_capacity(QUEUE_CAPACITY));
    let full_publication = publication(&mut full, &tokens);
    let full_point = authored_center(&full_publication, "surface.primary");
    for _ in 0..QUEUE_CAPACITY {
        full.submit_action(SurfaceAction::Swap)
            .unwrap_or_else(|_| unreachable!("configured queue slot is filled"));
    }
    let full_error = rejected(
        full.submit_surface_command(
            full_publication.input_context().clone(),
            full_point,
            SemanticCommand::Activate,
            CommandOrigin::programmatic(),
        ),
        "expected queue-full rejection",
    );
    assert_eq!(full_error.kind(), SubmitSurfaceCommandErrorKind::Full);
    assert!(has_rejection(
        &full,
        TraceSurfaceIngressKind::LogicalCoordinate,
        TraceSurfaceRejection::QueueFull,
    ));

    let mut closed = mounted();
    let closed_publication = publication(&mut closed, &tokens);
    let closed_context = closed_publication.input_context().clone();
    let closed_point = authored_center(&closed_publication, "surface.primary");
    let _ = closed.shutdown();
    let closed_error = rejected(
        closed.submit_surface_command(
            closed_context,
            closed_point,
            SemanticCommand::Activate,
            CommandOrigin::programmatic(),
        ),
        "expected closed-runtime rejection",
    );
    assert_eq!(closed_error.kind(), SubmitSurfaceCommandErrorKind::Closed);

    let mut exhausted_work = mounted();
    let work_publication = publication(&mut exhausted_work, &tokens);
    let work_context = work_publication.input_context().clone();
    let work_point = authored_center(&work_publication, "surface.primary");
    exhausted_work.__seed_next_work_sequence_for_test(0);
    let work_error = rejected(
        exhausted_work.submit_surface_command(
            work_context.clone(),
            work_point,
            SemanticCommand::Activate,
            CommandOrigin::programmatic(),
        ),
        "expected work-sequence rejection",
    );
    assert_eq!(
        work_error.kind(),
        SubmitSurfaceCommandErrorKind::WorkSequenceExhausted
    );
    let terminal_error = rejected(
        exhausted_work.submit_surface_command(
            work_context,
            work_point,
            SemanticCommand::Activate,
            CommandOrigin::programmatic(),
        ),
        "expected terminal-runtime rejection",
    );
    assert_eq!(
        terminal_error.kind(),
        SubmitSurfaceCommandErrorKind::Terminal(RuntimeTerminalReason::WorkSequenceExhausted)
    );

    let mut exhausted_trace = mounted();
    let trace_publication = publication(&mut exhausted_trace, &tokens);
    let trace_context = trace_publication.input_context().clone();
    let trace_point = authored_center(&trace_publication, "surface.primary");
    exhausted_trace.__seed_next_trace_sequence_for_test(u64::MAX - 2);
    let trace_error = rejected(
        exhausted_trace.submit_surface_command(
            trace_context,
            trace_point,
            SemanticCommand::Activate,
            CommandOrigin::programmatic(),
        ),
        "expected trace-sequence rejection",
    );
    assert_eq!(
        trace_error.kind(),
        SubmitSurfaceCommandErrorKind::TraceSequenceExhausted
    );
}
