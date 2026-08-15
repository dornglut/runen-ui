use runenui_core::__runtime::RuntimeNamespace;

use crate::TraceRecordKind;

use super::{json, tokens, value};

pub(super) const fn name(kind: &TraceRecordKind) -> &'static str {
    match kind {
        TraceRecordKind::RuntimeMounted => "runtime_mounted",
        TraceRecordKind::ActionSubmissionAccepted => "action_submission_accepted",
        TraceRecordKind::CommandSubmissionAccepted => "command_submission_accepted",
        TraceRecordKind::SemanticActionBound { .. } => "semantic_action_bound",
        TraceRecordKind::SemanticActionProcessingRejected { .. } => {
            "semantic_action_processing_rejected"
        }
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
        TraceRecordKind::KeyboardSpaceOwnershipCleared { .. } => "keyboard_space_ownership_cleared",
        TraceRecordKind::CommittedTextSubmissionAccepted => "committed_text_submission_accepted",
        TraceRecordKind::CommittedTextSubmissionRejected => "committed_text_submission_rejected",
        TraceRecordKind::CommittedTextProcessingValidated => "committed_text_processing_validated",
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
        TraceRecordKind::PointerLogicalScrollCollected { .. } => "pointer_logical_scroll_collected",
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
        TraceRecordKind::SemanticDefaultTargetInvalidated { .. } => {
            "semantic_default_target_invalidated"
        }
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
}

pub(super) fn data(output: &mut String, runtime: &RuntimeNamespace, kind: &TraceRecordKind) {
    match kind {
        TraceRecordKind::SemanticActionBound { target, command } => {
            output.push('{');
            json::name(output, "target");
            value::semantic_action_target(output, runtime, target);
            output.push(',');
            json::name(output, "command");
            value::semantic_command(output, *command);
            output.push('}');
        }
        TraceRecordKind::SemanticActionProcessingRejected { outcome } => {
            output.push_str("{\"outcome\":");
            json::string(output, tokens::semantic_action_rejection(*outcome));
            output.push('}');
        }
        TraceRecordKind::PointerSubmissionAccepted { pointer_id, phase }
        | TraceRecordKind::PointerIngressValidated { pointer_id, phase }
        | TraceRecordKind::PointerDefaultApplied { pointer_id, phase }
        | TraceRecordKind::PointerDefaultSuppressed { pointer_id, phase } => {
            output.push_str("{\"pointer_id\":");
            json::u64_value(output, pointer_id.get());
            output.push_str(",\"phase\":");
            json::string(output, tokens::pointer_phase(*phase));
            output.push('}');
        }
        TraceRecordKind::PointerIngressRejected {
            pointer_id,
            phase,
            outcome,
        } => {
            output.push_str("{\"pointer_id\":");
            json::u64_value(output, pointer_id.get());
            output.push_str(",\"phase\":");
            json::string(output, tokens::pointer_phase(*phase));
            output.push_str(",\"outcome\":");
            json::string(output, tokens::pointer_rejection(*outcome));
            output.push('}');
        }
        TraceRecordKind::PointerContextUnavailable {
            pointer_id,
            outcome,
        } => {
            output.push_str("{\"pointer_id\":");
            json::u64_value(output, pointer_id.get());
            output.push_str(",\"outcome\":");
            json::string(output, tokens::pointer_rejection(*outcome));
            output.push('}');
        }
        TraceRecordKind::PointerStreamResolved {
            pointer_id,
            new_stream,
        } => {
            output.push_str("{\"pointer_id\":");
            json::u64_value(output, pointer_id.get());
            output.push_str(",\"new_stream\":");
            json::bool_value(output, *new_stream);
            output.push('}');
        }
        TraceRecordKind::PointerStreamRegistered {
            pointer_id,
            registration_sequence,
        } => {
            output.push_str("{\"pointer_id\":");
            json::u64_value(output, pointer_id.get());
            output.push_str(",\"registration_sequence\":");
            json::u64_value(output, *registration_sequence);
            output.push('}');
        }
        TraceRecordKind::PointerStreamObserved { pointer_id }
        | TraceRecordKind::PointerStreamClosed { pointer_id }
        | TraceRecordKind::PointerInteractionCommitted { pointer_id }
        | TraceRecordKind::PointerActivateCollected { pointer_id }
        | TraceRecordKind::PointerLogicalScrollCollected { pointer_id } => {
            output.push_str("{\"pointer_id\":");
            json::u64_value(output, pointer_id.get());
            output.push('}');
        }
        TraceRecordKind::PointerBoundaryBundlePlanned { notifications } => {
            output.push_str("{\"notifications\":");
            json::usize_value(output, *notifications);
            output.push('}');
        }
        TraceRecordKind::PointerCaptureNotificationResolved { kind } => {
            output.push_str("{\"kind\":");
            json::string(output, tokens::pointer_capture_kind(*kind));
            output.push('}');
        }
        TraceRecordKind::PointerBoundaryNotificationResolved { kind } => {
            output.push_str("{\"kind\":");
            json::string(output, tokens::pointer_boundary_kind(*kind));
            output.push('}');
        }
        TraceRecordKind::PointerCaptureRequestRejected { request, outcome } => {
            output.push_str("{\"request\":");
            json::string(output, tokens::capture_request_kind(*request));
            output.push_str(",\"outcome\":");
            json::string(output, tokens::capture_request_rejection(*outcome));
            output.push('}');
        }
        TraceRecordKind::PointerStationaryRehitQueued {
            hit_test_generation,
            coordinate_revision,
        } => {
            output.push_str("{\"hit_test_generation\":");
            json::u64_value(output, *hit_test_generation);
            output.push_str(",\"coordinate_revision\":");
            json::u64_value(output, *coordinate_revision);
            output.push('}');
        }
        TraceRecordKind::KeyboardSpaceReleaseMatched { matched } => {
            output.push_str("{\"matched\":");
            json::bool_value(output, *matched);
            output.push('}');
        }
        TraceRecordKind::KeyboardSpaceOwnershipCleared { reason } => {
            output.push_str("{\"reason\":");
            json::string(output, tokens::space_cleanup_reason(*reason));
            output.push('}');
        }
        TraceRecordKind::CompositionCancelled { reason } => {
            output.push_str("{\"reason\":");
            json::string(output, tokens::composition_cancel_reason(*reason));
            output.push('}');
        }
        TraceRecordKind::SurfaceContextAccepted { ingress } => {
            output.push_str("{\"ingress\":");
            json::string(output, tokens::surface_ingress(*ingress));
            output.push('}');
        }
        TraceRecordKind::SurfaceCommandRejected { ingress, outcome } => {
            output.push_str("{\"ingress\":");
            json::string(output, tokens::surface_ingress(*ingress));
            output.push_str(",\"outcome\":");
            json::string(output, tokens::surface_rejection(*outcome));
            output.push('}');
        }
        TraceRecordKind::CommandProcessingRejected { outcome } => {
            output.push_str("{\"outcome\":");
            json::string(output, tokens::target_rejection(*outcome));
            output.push('}');
        }
        TraceRecordKind::RouteSnapshotCreated { invocations } => {
            output.push_str("{\"invocations\":");
            json::usize_value(output, *invocations);
            output.push('}');
        }
        TraceRecordKind::EventPhaseInvoked { phase } => {
            output.push_str("{\"phase\":");
            json::string(output, tokens::event_phase(*phase));
            output.push('}');
        }
        TraceRecordKind::DelegatedCommandCollected { command }
        | TraceRecordKind::SemanticDefaultApplied { command }
        | TraceRecordKind::SemanticDefaultSuppressed { command } => {
            output.push_str("{\"command\":");
            value::semantic_command(output, *command);
            output.push('}');
        }
        TraceRecordKind::SemanticDefaultTargetInvalidated { command, outcome } => {
            output.push_str("{\"command\":");
            value::semantic_command(output, *command);
            output.push_str(",\"outcome\":");
            json::string(output, tokens::semantic_action_rejection(*outcome));
            output.push('}');
        }
        TraceRecordKind::WidgetInvalidated { invalidation } => {
            output.push_str("{\"invalidation\":");
            value::invalidation(output, *invalidation);
            output.push('}');
        }
        TraceRecordKind::RoutedIntegrityFailed { failure } => {
            output.push_str("{\"failure\":");
            json::string(output, tokens::routed_integrity_failure(*failure));
            output.push('}');
        }
        TraceRecordKind::RoutedEventAdmissionRejected { capacity } => {
            output.push_str("{\"capacity\":");
            json::string(output, tokens::routed_admission_rejection(*capacity));
            output.push('}');
        }
        TraceRecordKind::FocusCommandEvaluated {
            command,
            linear_policy,
            directional_policy,
        } => {
            output.push_str("{\"command\":");
            value::semantic_command(output, *command);
            output.push_str(",\"linear_policy\":");
            json::string(output, tokens::focus_boundary_policy(*linear_policy));
            output.push_str(",\"directional_policy\":");
            json::string(output, tokens::focus_boundary_policy(*directional_policy));
            output.push('}');
        }
        TraceRecordKind::FocusCandidateSelected { outcome } => {
            output.push_str("{\"outcome\":");
            json::string(output, tokens::focus_boundary_outcome(*outcome));
            output.push('}');
        }
        TraceRecordKind::FocusTransitionCommitted { reason } => {
            output.push_str("{\"reason\":");
            json::string(output, tokens::focus_reason(*reason));
            output.push('}');
        }
        TraceRecordKind::FocusNotificationResolved { kind } => {
            output.push_str("{\"kind\":");
            json::string(output, tokens::focus_event_kind(*kind));
            output.push('}');
        }
        TraceRecordKind::FocusWithinInvalidated { left, entered } => {
            output.push_str("{\"left\":");
            json::usize_value(output, *left);
            output.push_str(",\"entered\":");
            json::usize_value(output, *entered);
            output.push('}');
        }
        TraceRecordKind::InitialEffectsCommitted { count }
        | TraceRecordKind::UpdateEffectsCommitted { count }
        | TraceRecordKind::QueuedWorkCancelled { count } => {
            output.push_str("{\"count\":");
            json::usize_value(output, *count);
            output.push('}');
        }
        TraceRecordKind::WorkStartRefused { outcome } => {
            output.push_str("{\"outcome\":");
            json::string(output, tokens::work_start_refusal(*outcome));
            output.push('}');
        }
        TraceRecordKind::ReadinessCheckpoint {
            imported_completions,
            polled_local_work,
            promoted_timers,
        } => {
            output.push_str("{\"imported_completions\":");
            json::usize_value(output, *imported_completions);
            output.push_str(",\"polled_local_work\":");
            json::usize_value(output, *polled_local_work);
            output.push_str(",\"promoted_timers\":");
            json::usize_value(output, *promoted_timers);
            output.push('}');
        }
        TraceRecordKind::SubscriptionDiffCommitted {
            started,
            cancelled,
            duplicate_keys,
        } => {
            output.push_str("{\"started\":");
            json::usize_value(output, *started);
            output.push_str(",\"cancelled\":");
            json::usize_value(output, *cancelled);
            output.push_str(",\"duplicate_keys\":");
            json::usize_value(output, *duplicate_keys);
            output.push('}');
        }
        TraceRecordKind::TimerTerminated { outcome } => {
            output.push_str("{\"outcome\":");
            json::string(output, tokens::timer_terminal_outcome(*outcome));
            output.push('}');
        }
        TraceRecordKind::RedrawRequested { revision }
        | TraceRecordKind::RedrawTaken { revision }
        | TraceRecordKind::RedrawAcknowledged { revision } => {
            output.push_str("{\"revision\":");
            json::u64_value(output, *revision);
            output.push('}');
        }
        TraceRecordKind::RuntimeTerminal { reason } => {
            output.push_str("{\"reason\":");
            json::string(output, tokens::runtime_terminal_reason(*reason));
            output.push('}');
        }
        TraceRecordKind::RuntimeShutdown {
            cancelled_queued,
            unmounted_lifetimes,
        } => {
            output.push_str("{\"cancelled_queued\":");
            json::usize_value(output, *cancelled_queued);
            output.push_str(",\"unmounted_lifetimes\":");
            json::usize_value(output, *unmounted_lifetimes);
            output.push('}');
        }
        TraceRecordKind::RuntimeMounted
        | TraceRecordKind::ActionSubmissionAccepted
        | TraceRecordKind::CommandSubmissionAccepted
        | TraceRecordKind::KeyboardSubmissionAccepted
        | TraceRecordKind::KeyboardSubmissionRejected
        | TraceRecordKind::KeyboardProcessingValidated
        | TraceRecordKind::KeyboardDefaultPrevented
        | TraceRecordKind::KeyboardEnterActivationDerived
        | TraceRecordKind::KeyboardSpaceOwnershipEstablished
        | TraceRecordKind::KeyboardSpaceActivationDerived
        | TraceRecordKind::CommittedTextSubmissionAccepted
        | TraceRecordKind::CommittedTextSubmissionRejected
        | TraceRecordKind::CommittedTextProcessingValidated
        | TraceRecordKind::CommittedTextDefaultPrevented
        | TraceRecordKind::CompositionGenerationAllocated
        | TraceRecordKind::CompositionPendingBound
        | TraceRecordKind::CompositionActiveBound
        | TraceRecordKind::CompositionProcessingValidated
        | TraceRecordKind::CompositionUpdateSubmitted
        | TraceRecordKind::CompositionEndSubmitted
        | TraceRecordKind::CompositionCancelSubmitted
        | TraceRecordKind::CompositionRetired
        | TraceRecordKind::CompositionProcessingStaleGeneration
        | TraceRecordKind::CompositionSubmissionRejected
        | TraceRecordKind::AutomationResolutionUnique
        | TraceRecordKind::AutomationResolutionMissing
        | TraceRecordKind::AutomationResolutionAmbiguous
        | TraceRecordKind::AutomationTargetStaleAfterResolution
        | TraceRecordKind::PointerPhysicalTargetResolved
        | TraceRecordKind::PointerIntegrityCleanupCommitted
        | TraceRecordKind::SurfaceTargetBound
        | TraceRecordKind::SurfacePublished
        | TraceRecordKind::RoutedEventStarted
        | TraceRecordKind::RoutedActionCollected
        | TraceRecordKind::PropagationStopped
        | TraceRecordKind::DefaultPrevented
        | TraceRecordKind::WidgetStateMutated
        | TraceRecordKind::MountedSubscriptionInvalidated
        | TraceRecordKind::RoutedEventCommitted
        | TraceRecordKind::ActionSubmissionRejectedFull
        | TraceRecordKind::ActionSubmissionRejectedClosed
        | TraceRecordKind::ActionSubmissionRejectedTerminal
        | TraceRecordKind::ApplicationActionTransactionStarted
        | TraceRecordKind::ApplicationStateUpdated
        | TraceRecordKind::TreeReconciled
        | TraceRecordKind::FocusRetained
        | TraceRecordKind::FocusRestorationAccepted
        | TraceRecordKind::FocusRestorationRejected
        | TraceRecordKind::ModalityChanged
        | TraceRecordKind::PumpBudgetExhausted
        | TraceRecordKind::InitialApplicationTransactionStarted
        | TraceRecordKind::WorkRequested
        | TraceRecordKind::WorkGenerationCommitted
        | TraceRecordKind::WorkStartAttempted
        | TraceRecordKind::WorkStartAccepted
        | TraceRecordKind::WorkLogicallyInvalidated
        | TraceRecordKind::WorkCancellationBound
        | TraceRecordKind::WorkCleanupProcessed
        | TraceRecordKind::WorkCompletionImported
        | TraceRecordKind::WorkCompletionRejectedStale
        | TraceRecordKind::WorkCompletionMapped
        | TraceRecordKind::LocalWorkPolled
        | TraceRecordKind::LocalWorkReady
        | TraceRecordKind::TimerPromoted
        | TraceRecordKind::SubscriptionDeclared
        | TraceRecordKind::MountedSubscriptionReconciliationSuppressedStale
        | TraceRecordKind::TimerFired
        | TraceRecordKind::HostRequestExposed
        | TraceRecordKind::HostResponseAccepted
        | TraceRecordKind::HostResponseRejected
        | TraceRecordKind::WakeRequested
        | TraceRecordKind::WakeAcknowledged => output.push_str("{}"),
    }
}
