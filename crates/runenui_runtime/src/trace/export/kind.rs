use crate::TraceRecordKind;

use super::{json, tokens, value};

macro_rules! trace_kind_name {
    ($kind:expr) => {
        match $kind {
            TraceRecordKind::RuntimeMounted => "runtime_mounted",
            TraceRecordKind::ActionSubmissionAccepted => "action_submission_accepted",
            TraceRecordKind::CommandSubmissionAccepted => "command_submission_accepted",
            TraceRecordKind::PointerSubmissionAccepted { .. } => "pointer_submission_accepted",
            TraceRecordKind::KeyboardSubmissionAccepted => "keyboard_submission_accepted",
            TraceRecordKind::KeyboardSubmissionRejected => "keyboard_submission_rejected",
            TraceRecordKind::KeyboardProcessingValidated => "keyboard_processing_validated",
            TraceRecordKind::KeyboardDefaultPrevented => "keyboard_default_prevented",
            TraceRecordKind::KeyboardEnterActivationDerived => "keyboard_enter_activation_derived",
            TraceRecordKind::KeyboardSpaceOwnershipEstablished => {
                "keyboard_space_ownership_established"
            }
            TraceRecordKind::KeyboardSpaceReleaseMatched { .. } => "keyboard_space_release_matched",
            TraceRecordKind::KeyboardSpaceActivationDerived => "keyboard_space_activation_derived",
            TraceRecordKind::KeyboardSpaceOwnershipCleared { .. } => {
                "keyboard_space_ownership_cleared"
            }
            TraceRecordKind::CommittedTextSubmissionAccepted => "committed_text_submission_accepted",
            TraceRecordKind::CommittedTextSubmissionRejected => "committed_text_submission_rejected",
            TraceRecordKind::CommittedTextProcessingValidated => {
                "committed_text_processing_validated"
            }
            TraceRecordKind::CommittedTextDefaultPrevented => "committed_text_default_prevented",
            TraceRecordKind::CompositionGenerationAllocated => "composition_generation_allocated",
            TraceRecordKind::CompositionPendingBound => "composition_pending_bound",
            TraceRecordKind::CompositionActiveBound => "composition_active_bound",
            TraceRecordKind::CompositionProcessingValidated => "composition_processing_validated",
            TraceRecordKind::CompositionUpdateSubmitted => "composition_update_submitted",
            TraceRecordKind::CompositionEndSubmitted => "composition_end_submitted",
            TraceRecordKind::CompositionCancelSubmitted => "composition_cancel_submitted",
            TraceRecordKind::CompositionCancelled { .. } => "composition_cancelled",
            TraceRecordKind::CompositionRetired => "composition_retired",
            TraceRecordKind::CompositionProcessingStaleGeneration => {
                "composition_processing_stale_generation"
            }
            TraceRecordKind::CompositionSubmissionRejected => "composition_submission_rejected",
            TraceRecordKind::AutomationResolutionUnique => "automation_resolution_unique",
            TraceRecordKind::AutomationResolutionMissing => "automation_resolution_missing",
            TraceRecordKind::AutomationResolutionAmbiguous => "automation_resolution_ambiguous",
            TraceRecordKind::AutomationTargetStaleAfterResolution => {
                "automation_target_stale_after_resolution"
            }
            TraceRecordKind::PointerIngressRejected { .. } => "pointer_ingress_rejected",
            TraceRecordKind::PointerIngressValidated { .. } => "pointer_ingress_validated",
            TraceRecordKind::PointerContextUnavailable { .. } => "pointer_context_unavailable",
            TraceRecordKind::PointerStreamResolved { .. } => "pointer_stream_resolved",
            TraceRecordKind::PointerStreamRegistered { .. } => "pointer_stream_registered",
            TraceRecordKind::PointerStreamObserved { .. } => "pointer_stream_observed",
            TraceRecordKind::PointerStreamClosed { .. } => "pointer_stream_closed",
            TraceRecordKind::PointerPhysicalTargetResolved => "pointer_physical_target_resolved",
            TraceRecordKind::PointerBoundaryBundlePlanned { .. } => "pointer_boundary_bundle_planned",
            TraceRecordKind::PointerDefaultApplied { .. } => "pointer_default_applied",
            TraceRecordKind::PointerDefaultSuppressed { .. } => "pointer_default_suppressed",
            TraceRecordKind::PointerInteractionCommitted { .. } => "pointer_interaction_committed",
            TraceRecordKind::PointerCaptureNotificationResolved { .. } => {
                "pointer_capture_notification_resolved"
            }
            TraceRecordKind::PointerBoundaryNotificationResolved { .. } => {
                "pointer_boundary_notification_resolved"
            }
            TraceRecordKind::PointerActivateCollected { .. } => "pointer_activate_collected",
            TraceRecordKind::PointerLogicalScrollCollected { .. } => {
                "pointer_logical_scroll_collected"
            }
            TraceRecordKind::PointerStationaryRehitQueued { .. } => "pointer_stationary_rehit_queued",
            TraceRecordKind::PointerCaptureRequestRejected { .. } => "pointer_capture_request_rejected",
            TraceRecordKind::PointerIntegrityCleanupCommitted => "pointer_integrity_cleanup_committed",
            TraceRecordKind::SurfaceContextAccepted { .. } => "surface_context_accepted",
            TraceRecordKind::SurfaceTargetBound => "surface_target_bound",
            TraceRecordKind::SurfaceCommandRejected { .. } => "surface_command_rejected",
            TraceRecordKind::SurfacePublished => "surface_published",
            TraceRecordKind::CommandProcessingRejected { .. } => "command_processing_rejected",
            TraceRecordKind::RoutedEventStarted => "routed_event_started",
            TraceRecordKind::RouteSnapshotCreated { .. } => "route_snapshot_created",
            TraceRecordKind::EventPhaseInvoked { .. } => "event_phase_invoked",
            TraceRecordKind::RoutedActionCollected => "routed_action_collected",
            TraceRecordKind::DelegatedCommandCollected { .. } => "delegated_command_collected",
            TraceRecordKind::PropagationStopped => "propagation_stopped",
            TraceRecordKind::DefaultPrevented => "default_prevented",
            TraceRecordKind::WidgetStateMutated => "widget_state_mutated",
            TraceRecordKind::WidgetInvalidated { .. } => "widget_invalidated",
            TraceRecordKind::MountedSubscriptionInvalidated => "mounted_subscription_invalidated",
            TraceRecordKind::SemanticDefaultApplied { .. } => "semantic_default_applied",
            TraceRecordKind::SemanticDefaultSuppressed { .. } => "semantic_default_suppressed",
            TraceRecordKind::RoutedEventCommitted => "routed_event_committed",
            TraceRecordKind::RoutedIntegrityFailed { .. } => "routed_integrity_failed",
            TraceRecordKind::RoutedEventAdmissionRejected { .. } => "routed_event_admission_rejected",
            TraceRecordKind::ActionSubmissionRejectedFull => "action_submission_rejected_full",
            TraceRecordKind::ActionSubmissionRejectedClosed => "action_submission_rejected_closed",
            TraceRecordKind::ActionSubmissionRejectedTerminal => "action_submission_rejected_terminal",
            TraceRecordKind::ApplicationActionTransactionStarted => {
                "application_action_transaction_started"
            }
            TraceRecordKind::ApplicationStateUpdated => "application_state_updated",
            TraceRecordKind::TreeReconciled => "tree_reconciled",
            TraceRecordKind::FocusRetained => "focus_retained",
            TraceRecordKind::FocusCommandEvaluated { .. } => "focus_command_evaluated",
            TraceRecordKind::FocusCandidateSelected { .. } => "focus_candidate_selected",
            TraceRecordKind::FocusRestorationAccepted => "focus_restoration_accepted",
            TraceRecordKind::FocusRestorationRejected => "focus_restoration_rejected",
            TraceRecordKind::FocusTransitionCommitted { .. } => "focus_transition_committed",
            TraceRecordKind::FocusNotificationResolved { .. } => "focus_notification_resolved",
            TraceRecordKind::FocusWithinInvalidated { .. } => "focus_within_invalidated",
            TraceRecordKind::ModalityChanged => "modality_changed",
            TraceRecordKind::PumpBudgetExhausted => "pump_budget_exhausted",
            TraceRecordKind::InitialEffectsCommitted { .. } => "initial_effects_committed",
            TraceRecordKind::InitialApplicationTransactionStarted => {
                "initial_application_transaction_started"
            }
            TraceRecordKind::UpdateEffectsCommitted { .. } => "update_effects_committed",
            TraceRecordKind::WorkRequested => "work_requested",
            TraceRecordKind::WorkGenerationCommitted => "work_generation_committed",
            TraceRecordKind::WorkStartAttempted => "work_start_attempted",
            TraceRecordKind::WorkStartAccepted => "work_start_accepted",
            TraceRecordKind::WorkStartRefused { .. } => "work_start_refused",
            TraceRecordKind::WorkLogicallyInvalidated => "work_logically_invalidated",
            TraceRecordKind::WorkCancellationBound => "work_cancellation_bound",
            TraceRecordKind::WorkCleanupProcessed => "work_cleanup_processed",
            TraceRecordKind::WorkCompletionImported => "work_completion_imported",
            TraceRecordKind::WorkCompletionRejectedStale => "work_completion_rejected_stale",
            TraceRecordKind::WorkCompletionMapped => "work_completion_mapped",
            TraceRecordKind::LocalWorkPolled => "local_work_polled",
            TraceRecordKind::LocalWorkReady => "local_work_ready",
            TraceRecordKind::TimerPromoted => "timer_promoted",
            TraceRecordKind::ReadinessCheckpoint { .. } => "readiness_checkpoint",
            TraceRecordKind::SubscriptionDeclared => "subscription_declared",
            TraceRecordKind::SubscriptionDiffCommitted { .. } => "subscription_diff_committed",
            TraceRecordKind::MountedSubscriptionReconciliationSuppressedStale => {
                "mounted_subscription_reconciliation_suppressed_stale"
            }
            TraceRecordKind::TimerFired => "timer_fired",
            TraceRecordKind::TimerTerminated { .. } => "timer_terminated",
            TraceRecordKind::HostRequestExposed => "host_request_exposed",
            TraceRecordKind::HostResponseAccepted => "host_response_accepted",
            TraceRecordKind::HostResponseRejected => "host_response_rejected",
            TraceRecordKind::WakeRequested => "wake_requested",
            TraceRecordKind::WakeAcknowledged => "wake_acknowledged",
            TraceRecordKind::RedrawRequested { .. } => "redraw_requested",
            TraceRecordKind::RedrawTaken { .. } => "redraw_taken",
            TraceRecordKind::RedrawAcknowledged { .. } => "redraw_acknowledged",
            TraceRecordKind::QueuedWorkCancelled { .. } => "queued_work_cancelled",
            TraceRecordKind::RuntimeTerminal { .. } => "runtime_terminal",
            TraceRecordKind::RuntimeShutdown { .. } => "runtime_shutdown",
        }
    };
}

pub(super) fn encode(output: &mut String, kind: &TraceRecordKind) {
    output.push('{');
    json::name(output, "name");
    json::string(output, name(kind));
    output.push(',');
    json::name(output, "data");
    output.push('{');
    data(output, kind);
    output.push_str("}}");
}

const fn name(kind: &TraceRecordKind) -> &'static str {
    trace_kind_name!(kind)
}

fn data(output: &mut String, kind: &TraceRecordKind) {
    if encode_input_data(output, kind) || encode_pointer_data(output, kind) {
        return;
    }
    if encode_routed_focus_data(output, kind) {
        return;
    }
    let _ = encode_runtime_data(output, kind);
}

fn encode_input_data(output: &mut String, kind: &TraceRecordKind) -> bool {
    match kind {
        TraceRecordKind::KeyboardSpaceReleaseMatched { matched } => {
            field_bool(output, "matched", *matched);
        }
        TraceRecordKind::KeyboardSpaceOwnershipCleared { reason } => {
            field_str(output, "reason", tokens::space_cleanup_reason(*reason));
        }
        TraceRecordKind::CompositionCancelled { reason } => {
            field_str(output, "reason", tokens::composition_cancel_reason(*reason));
        }
        _ => return false,
    }
    true
}

fn encode_pointer_data(output: &mut String, kind: &TraceRecordKind) -> bool {
    match kind {
        TraceRecordKind::PointerSubmissionAccepted { pointer_id, phase }
        | TraceRecordKind::PointerIngressValidated { pointer_id, phase }
        | TraceRecordKind::PointerDefaultApplied { pointer_id, phase }
        | TraceRecordKind::PointerDefaultSuppressed { pointer_id, phase } => {
            pointer_phase(output, *pointer_id, *phase);
        }
        TraceRecordKind::PointerIngressRejected {
            pointer_id,
            phase,
            outcome,
        } => {
            pointer_phase(output, *pointer_id, *phase);
            output.push(',');
            field_str(output, "outcome", tokens::pointer_rejection(*outcome));
        }
        TraceRecordKind::PointerContextUnavailable {
            pointer_id,
            outcome,
        } => {
            field_u64(output, "pointer_id", pointer_id.get());
            output.push(',');
            field_str(output, "outcome", tokens::pointer_rejection(*outcome));
        }
        TraceRecordKind::PointerStreamResolved {
            pointer_id,
            new_stream,
        } => {
            field_u64(output, "pointer_id", pointer_id.get());
            output.push(',');
            field_bool(output, "new_stream", *new_stream);
        }
        TraceRecordKind::PointerStreamRegistered {
            pointer_id,
            registration_sequence,
        } => {
            field_u64(output, "pointer_id", pointer_id.get());
            output.push(',');
            field_u64(output, "registration_sequence", *registration_sequence);
        }
        TraceRecordKind::PointerStreamObserved { pointer_id }
        | TraceRecordKind::PointerStreamClosed { pointer_id }
        | TraceRecordKind::PointerActivateCollected { pointer_id }
        | TraceRecordKind::PointerLogicalScrollCollected { pointer_id }
        | TraceRecordKind::PointerInteractionCommitted { pointer_id } => {
            field_u64(output, "pointer_id", pointer_id.get());
        }
        TraceRecordKind::PointerBoundaryBundlePlanned { notifications } => {
            field_usize(output, "notifications", *notifications);
        }
        TraceRecordKind::PointerCaptureNotificationResolved { kind } => {
            field_str(output, "kind", tokens::pointer_capture_kind(*kind));
        }
        TraceRecordKind::PointerBoundaryNotificationResolved { kind } => {
            field_str(output, "kind", tokens::pointer_boundary_kind(*kind));
        }
        TraceRecordKind::PointerStationaryRehitQueued {
            hit_test_generation,
            coordinate_revision,
        } => {
            field_u64(output, "hit_test_generation", *hit_test_generation);
            output.push(',');
            field_u64(output, "coordinate_revision", *coordinate_revision);
        }
        TraceRecordKind::PointerCaptureRequestRejected { request, outcome } => {
            field_str(output, "request", tokens::capture_request_kind(*request));
            output.push(',');
            field_str(
                output,
                "outcome",
                tokens::capture_request_rejection(*outcome),
            );
        }
        _ => return false,
    }
    true
}

fn encode_routed_focus_data(output: &mut String, kind: &TraceRecordKind) -> bool {
    match kind {
        TraceRecordKind::SurfaceContextAccepted { ingress } => {
            field_str(output, "ingress", tokens::surface_ingress(*ingress));
        }
        TraceRecordKind::SurfaceCommandRejected { ingress, outcome } => {
            field_str(output, "ingress", tokens::surface_ingress(*ingress));
            output.push(',');
            field_str(output, "outcome", tokens::surface_rejection(*outcome));
        }
        TraceRecordKind::CommandProcessingRejected { outcome } => {
            field_str(output, "outcome", tokens::target_rejection(*outcome));
        }
        TraceRecordKind::RouteSnapshotCreated { invocations } => {
            field_usize(output, "invocations", *invocations);
        }
        TraceRecordKind::EventPhaseInvoked { phase } => {
            field_str(output, "phase", tokens::event_phase(*phase));
        }
        TraceRecordKind::DelegatedCommandCollected { command }
        | TraceRecordKind::SemanticDefaultApplied { command }
        | TraceRecordKind::SemanticDefaultSuppressed { command } => {
            json::name(output, "command");
            value::semantic_command(output, *command);
        }
        TraceRecordKind::WidgetInvalidated { invalidation } => {
            json::name(output, "invalidation");
            value::invalidation(output, *invalidation);
        }
        TraceRecordKind::RoutedIntegrityFailed { failure } => {
            field_str(
                output,
                "failure",
                tokens::routed_integrity_failure(*failure),
            );
        }
        TraceRecordKind::RoutedEventAdmissionRejected { capacity } => {
            field_str(
                output,
                "capacity",
                tokens::routed_admission_rejection(*capacity),
            );
        }
        TraceRecordKind::FocusCommandEvaluated {
            command,
            linear_policy,
            directional_policy,
        } => {
            json::name(output, "command");
            value::semantic_command(output, *command);
            output.push(',');
            field_str(
                output,
                "linear_policy",
                tokens::focus_boundary_policy(*linear_policy),
            );
            output.push(',');
            field_str(
                output,
                "directional_policy",
                tokens::focus_boundary_policy(*directional_policy),
            );
        }
        TraceRecordKind::FocusCandidateSelected { outcome } => {
            field_str(output, "outcome", tokens::focus_boundary_outcome(*outcome));
        }
        TraceRecordKind::FocusTransitionCommitted { reason } => {
            field_str(output, "reason", tokens::focus_reason(*reason));
        }
        TraceRecordKind::FocusNotificationResolved { kind } => {
            field_str(output, "kind", tokens::focus_event_kind(*kind));
        }
        TraceRecordKind::FocusWithinInvalidated { left, entered } => {
            field_usize(output, "left", *left);
            output.push(',');
            field_usize(output, "entered", *entered);
        }
        _ => return false,
    }
    true
}

fn encode_runtime_data(output: &mut String, kind: &TraceRecordKind) -> bool {
    match kind {
        TraceRecordKind::InitialEffectsCommitted { count }
        | TraceRecordKind::UpdateEffectsCommitted { count }
        | TraceRecordKind::QueuedWorkCancelled { count } => {
            field_usize(output, "count", *count);
        }
        TraceRecordKind::WorkStartRefused { outcome } => {
            field_str(output, "outcome", tokens::work_start_refusal(*outcome));
        }
        TraceRecordKind::ReadinessCheckpoint {
            imported_completions,
            polled_local_work,
            promoted_timers,
        } => {
            field_usize(output, "imported_completions", *imported_completions);
            output.push(',');
            field_usize(output, "polled_local_work", *polled_local_work);
            output.push(',');
            field_usize(output, "promoted_timers", *promoted_timers);
        }
        TraceRecordKind::SubscriptionDiffCommitted {
            started,
            cancelled,
            duplicate_keys,
        } => {
            field_usize(output, "started", *started);
            output.push(',');
            field_usize(output, "cancelled", *cancelled);
            output.push(',');
            field_usize(output, "duplicate_keys", *duplicate_keys);
        }
        TraceRecordKind::TimerTerminated { outcome } => {
            field_str(output, "outcome", tokens::timer_terminal_outcome(*outcome));
        }
        TraceRecordKind::RedrawRequested { revision }
        | TraceRecordKind::RedrawTaken { revision }
        | TraceRecordKind::RedrawAcknowledged { revision } => {
            field_u64(output, "revision", *revision);
        }
        TraceRecordKind::RuntimeTerminal { reason } => {
            field_str(output, "reason", tokens::runtime_terminal_reason(*reason));
        }
        TraceRecordKind::RuntimeShutdown {
            cancelled_queued,
            unmounted_lifetimes,
        } => {
            field_usize(output, "cancelled_queued", *cancelled_queued);
            output.push(',');
            field_usize(output, "unmounted_lifetimes", *unmounted_lifetimes);
        }
        _ => return false,
    }
    true
}

fn pointer_phase(
    output: &mut String,
    pointer_id: runenui_core::PointerId,
    phase: runenui_core::PointerPhase,
) {
    field_u64(output, "pointer_id", pointer_id.get());
    output.push(',');
    field_str(output, "phase", tokens::pointer_phase(phase));
}

fn field_str(output: &mut String, key: &str, value: &str) {
    json::name(output, key);
    json::string(output, value);
}

fn field_u64(output: &mut String, key: &str, value: u64) {
    json::name(output, key);
    json::u64_value(output, value);
}

fn field_usize(output: &mut String, key: &str, value: usize) {
    json::name(output, key);
    json::usize_value(output, value);
}

fn field_bool(output: &mut String, key: &str, value: bool) {
    json::name(output, key);
    json::bool_value(output, value);
}
