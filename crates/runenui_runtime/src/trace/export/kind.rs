use runenui_core::__runtime::RuntimeNamespace;

use crate::{
    TraceMotionCollision, TraceMotionFact, TraceMotionInterpolation, TraceMotionLifecycle,
    TraceMotionPhase, TraceMotionPlanningRejection, TraceMotionPolicy,
    TraceMotionPreferenceDecision, TraceMotionSource, TraceRecordKind,
};

use super::{json, tokens, value};

macro_rules! trace_kind_name {
    ($kind:expr) => {
        match $kind {
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
            TraceRecordKind::KeyboardSpaceOwnershipCleared { .. } => {
                "keyboard_space_ownership_cleared"
            }
            TraceRecordKind::CommittedTextSubmissionAccepted => {
                "committed_text_submission_accepted"
            }
            TraceRecordKind::CommittedTextSubmissionRejected => {
                "committed_text_submission_rejected"
            }
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
            TraceRecordKind::PointerBoundaryBundlePlanned { .. } => {
                "pointer_boundary_bundle_planned"
            }
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
            TraceRecordKind::PointerTextSelectionStarted { .. } => "pointer_text_selection_started",
            TraceRecordKind::PointerTextSelectionUpdated { .. } => "pointer_text_selection_updated",
            TraceRecordKind::PointerTextSelectionEnded { .. } => "pointer_text_selection_ended",
            TraceRecordKind::PointerTextSelectionCancelled { .. } => {
                "pointer_text_selection_cancelled"
            }
            TraceRecordKind::TouchGestureProvisional { .. } => "touch_gesture_provisional",
            TraceRecordKind::TouchGestureWon { .. } => "touch_gesture_won",
            TraceRecordKind::TouchGestureCancelled { .. } => "touch_gesture_cancelled",
            TraceRecordKind::TouchGestureCompleted { .. } => "touch_gesture_completed",
            TraceRecordKind::LogicalScrollOwnerApplied { .. } => "logical_scroll_owner_applied",
            TraceRecordKind::LogicalScrollChainCompleted { .. } => "logical_scroll_chain_completed",
            TraceRecordKind::PointerStationaryRehitQueued { .. } => {
                "pointer_stationary_rehit_queued"
            }
            TraceRecordKind::PointerCaptureRequestRejected { .. } => {
                "pointer_capture_request_rejected"
            }
            TraceRecordKind::PointerIntegrityCleanupCommitted => {
                "pointer_integrity_cleanup_committed"
            }
            TraceRecordKind::SurfaceContextAccepted { .. } => "surface_context_accepted",
            TraceRecordKind::SurfaceTargetBound => "surface_target_bound",
            TraceRecordKind::SurfaceCommandRejected { .. } => "surface_command_rejected",
            TraceRecordKind::SurfacePublished => "surface_published",
            TraceRecordKind::Motion { .. } => "motion",
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
            TraceRecordKind::EditingDefaultUnavailable { .. } => "editing_default_unavailable",
            TraceRecordKind::SemanticDefaultTargetInvalidated { .. } => {
                "semantic_default_target_invalidated"
            }
            TraceRecordKind::RoutedEventCommitted => "routed_event_committed",
            TraceRecordKind::RoutedIntegrityFailed { .. } => "routed_integrity_failed",
            TraceRecordKind::RoutedEventAdmissionRejected { .. } => {
                "routed_event_admission_rejected"
            }
            TraceRecordKind::ActionSubmissionRejectedFull => "action_submission_rejected_full",
            TraceRecordKind::ActionSubmissionRejectedClosed => "action_submission_rejected_closed",
            TraceRecordKind::ActionSubmissionRejectedTerminal => {
                "action_submission_rejected_terminal"
            }
            TraceRecordKind::ApplicationActionTransactionStarted => {
                "application_action_transaction_started"
            }
            TraceRecordKind::ApplicationStateUpdated => "application_state_updated",
            TraceRecordKind::EditResolutionVerified { .. } => "edit_resolution_verified",
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
            TraceRecordKind::FrameworkServiceExposed => "framework_service_exposed",
            TraceRecordKind::FrameworkServiceResponseQueued => "framework_service_response_queued",
            TraceRecordKind::FrameworkServiceResponseAccepted => {
                "framework_service_response_accepted"
            }
            TraceRecordKind::FrameworkServiceResponseOutcome { .. } => {
                "framework_service_response_outcome"
            }
            TraceRecordKind::FrameworkServiceResponseRejected => {
                "framework_service_response_rejected"
            }
            TraceRecordKind::FrameworkServiceCancelled => "framework_service_cancelled",
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

pub(super) const fn name(kind: &TraceRecordKind) -> &'static str {
    trace_kind_name!(kind)
}

pub(super) fn data(output: &mut String, runtime: &RuntimeNamespace, kind: &TraceRecordKind) {
    output.push('{');
    encode_data_fields(output, runtime, kind);
    output.push('}');
}

fn encode_data_fields(output: &mut String, runtime: &RuntimeNamespace, kind: &TraceRecordKind) {
    if encode_motion_data(output, kind)
        || encode_semantic_data(output, runtime, kind)
        || encode_input_data(output, kind)
        || encode_pointer_data(output, kind)
    {
        return;
    }
    if encode_routed_focus_data(output, kind) {
        return;
    }
    let _ = encode_runtime_data(output, kind);
}

fn encode_motion_data(output: &mut String, kind: &TraceRecordKind) -> bool {
    let TraceRecordKind::Motion { target, fact } = kind else {
        return false;
    };
    field_str(output, "target", motion_target(*target));
    output.push(',');
    match fact {
        TraceMotionFact::PolicyResolved { policy } => {
            field_str(output, "fact", "policy_resolved");
            output.push(',');
            field_str(output, "policy", motion_policy(*policy));
        }
        TraceMotionFact::CollisionRejected { source, collision } => {
            field_str(output, "fact", "collision_rejected");
            output.push(',');
            encode_motion_source(output, source);
            output.push(',');
            field_str(output, "collision", motion_collision(*collision));
        }
        TraceMotionFact::Lifecycle { source, lifecycle } => {
            field_str(output, "fact", "lifecycle");
            output.push(',');
            encode_motion_source(output, source);
            output.push(',');
            field_str(output, "lifecycle", motion_lifecycle(*lifecycle));
        }
        TraceMotionFact::Sampled {
            source,
            phase,
            progress_bits,
            eased_progress_bits,
            interpolation,
            suppressed,
        } => {
            field_str(output, "fact", "sampled");
            output.push(',');
            encode_motion_source(output, source);
            output.push(',');
            field_str(output, "phase", motion_phase(*phase));
            output.push_str(",\"progress_bits\":");
            json::optional_u64(output, progress_bits.map(u64::from));
            output.push_str(",\"eased_progress_bits\":");
            json::optional_u64(output, eased_progress_bits.map(u64::from));
            output.push(',');
            field_str(
                output,
                "interpolation",
                motion_interpolation(*interpolation),
            );
            output.push(',');
            field_bool(output, "suppressed", *suppressed);
        }
        TraceMotionFact::Preference {
            source,
            reduced_motion,
            strategy,
            decision,
        } => {
            field_str(output, "fact", "preference");
            output.push(',');
            encode_motion_source(output, source);
            output.push(',');
            field_bool(output, "reduced_motion", *reduced_motion);
            output.push(',');
            field_str(output, "strategy", reduced_motion_strategy(*strategy));
            output.push(',');
            field_str(output, "decision", motion_preference(*decision));
        }
        TraceMotionFact::Effect { decision } => {
            field_str(output, "fact", "effect");
            output.push(',');
            field_bool(output, "layout", decision.layout());
            output.push(',');
            field_bool(output, "presentation", decision.presentation());
            output.push(',');
            field_bool(output, "paint", decision.paint());
            output.push(',');
            field_bool(
                output,
                "retain_node_effect_group",
                decision.retain_node_effect_group(),
            );
            output.push(',');
            field_bool(output, "effective_changed", decision.effective_changed());
        }
        TraceMotionFact::PlanningRejected { source, rejection } => {
            field_str(output, "fact", "planning_rejected");
            output.push(',');
            if let Some(source) = source {
                encode_motion_source(output, source);
            } else {
                json::name(output, "source");
                output.push_str("null");
                output.push_str(",\"animation_id\":null");
            }
            output.push(',');
            field_str(output, "rejection", motion_planning_rejection(*rejection));
        }
    }
    true
}

fn encode_motion_source(output: &mut String, source: &TraceMotionSource) {
    match source {
        TraceMotionSource::Transition => {
            field_str(output, "source", "transition");
            output.push_str(",\"animation_id\":null");
        }
        TraceMotionSource::Timeline { animation_id } => {
            field_str(output, "source", "timeline");
            output.push_str(",\"animation_id\":");
            json::string(output, animation_id.as_str());
        }
    }
}

fn motion_target(target: runenui_core::MotionTarget) -> &'static str {
    match target {
        runenui_core::MotionTarget::Foreground => "foreground",
        runenui_core::MotionTarget::Background => "background",
        runenui_core::MotionTarget::Padding => "padding",
        runenui_core::MotionTarget::Radius => "radius",
        runenui_core::MotionTarget::Typography => "typography",
        runenui_core::MotionTarget::Shadows => "shadows",
        runenui_core::MotionTarget::Opacity => "opacity",
        runenui_core::MotionTarget::Presentation => "presentation",
        runenui_core::MotionTarget::Width => "width",
        runenui_core::MotionTarget::Height => "height",
        runenui_core::MotionTarget::MinWidth => "min_width",
        runenui_core::MotionTarget::MinHeight => "min_height",
        runenui_core::MotionTarget::MaxWidth => "max_width",
        runenui_core::MotionTarget::MaxHeight => "max_height",
        runenui_core::MotionTarget::Margin => "margin",
        runenui_core::MotionTarget::Gap => "gap",
        runenui_core::MotionTarget::FlexGrow => "flex_grow",
        runenui_core::MotionTarget::FlexShrink => "flex_shrink",
        runenui_core::MotionTarget::FlexBasis => "flex_basis",
        _ => unreachable!("runtime and core motion-target vocabularies are version-locked"),
    }
}

const fn motion_policy(policy: TraceMotionPolicy) -> &'static str {
    match policy {
        TraceMotionPolicy::Absent => "absent",
        TraceMotionPolicy::Disabled => "disabled",
        TraceMotionPolicy::Enabled => "enabled",
    }
}

const fn motion_collision(collision: TraceMotionCollision) -> &'static str {
    match collision {
        TraceMotionCollision::DuplicateAnimationId => "duplicate_animation_id",
        TraceMotionCollision::DuplicateTarget => "duplicate_target",
    }
}

const fn motion_lifecycle(lifecycle: TraceMotionLifecycle) -> &'static str {
    match lifecycle {
        TraceMotionLifecycle::Started => "started",
        TraceMotionLifecycle::Replaced => "replaced",
        TraceMotionLifecycle::Cancelled => "cancelled",
        TraceMotionLifecycle::Restarted => "restarted",
        TraceMotionLifecycle::Completed => "completed",
        TraceMotionLifecycle::CompletedRetained => "completed_retained",
        TraceMotionLifecycle::HoldInitialEntered => "hold_initial_entered",
        TraceMotionLifecycle::HoldInitialReleased => "hold_initial_released",
    }
}

const fn motion_phase(phase: TraceMotionPhase) -> &'static str {
    match phase {
        TraceMotionPhase::Delayed => "delayed",
        TraceMotionPhase::Running => "running",
        TraceMotionPhase::Completed => "completed",
        TraceMotionPhase::HeldInitial => "held_initial",
    }
}

const fn motion_interpolation(interpolation: TraceMotionInterpolation) -> &'static str {
    match interpolation {
        TraceMotionInterpolation::Endpoint => "endpoint",
        TraceMotionInterpolation::Continuous => "continuous",
        TraceMotionInterpolation::Discrete => "discrete",
    }
}

const fn motion_preference(decision: TraceMotionPreferenceDecision) -> &'static str {
    match decision {
        TraceMotionPreferenceDecision::Normal => "normal",
        TraceMotionPreferenceDecision::SnapToEnd => "snap_to_end",
        TraceMotionPreferenceDecision::HoldInitial => "hold_initial",
        TraceMotionPreferenceDecision::PreserveEssential => "preserve_essential",
        TraceMotionPreferenceDecision::HighContrastSuppressed => "high_contrast_suppressed",
    }
}

const fn motion_planning_rejection(rejection: TraceMotionPlanningRejection) -> &'static str {
    match rejection {
        TraceMotionPlanningRejection::ScheduleOverflow => "schedule_overflow",
        TraceMotionPlanningRejection::Interpolation => "interpolation",
    }
}

fn reduced_motion_strategy(strategy: runenui_core::ReducedMotionStrategy) -> &'static str {
    match strategy {
        runenui_core::ReducedMotionStrategy::SnapToEnd => "snap_to_end",
        runenui_core::ReducedMotionStrategy::HoldInitial => "hold_initial",
        runenui_core::ReducedMotionStrategy::PreserveEssential => "preserve_essential",
        _ => unreachable!("runtime and core reduced-motion vocabularies are version-locked"),
    }
}

fn encode_semantic_data(
    output: &mut String,
    runtime: &RuntimeNamespace,
    kind: &TraceRecordKind,
) -> bool {
    match kind {
        TraceRecordKind::SemanticActionBound { target, command } => {
            json::name(output, "target");
            value::semantic_action_target(output, runtime, target);
            output.push(',');
            json::name(output, "command");
            value::semantic_command(output, *command);
        }
        TraceRecordKind::SemanticActionProcessingRejected { outcome } => {
            field_str(
                output,
                "outcome",
                tokens::semantic_action_rejection(*outcome),
            );
        }
        TraceRecordKind::SemanticDefaultTargetInvalidated { command, outcome } => {
            json::name(output, "command");
            value::semantic_command(output, *command);
            output.push(',');
            field_str(
                output,
                "outcome",
                tokens::semantic_action_rejection(*outcome),
            );
        }
        _ => return false,
    }
    true
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

#[allow(clippy::too_many_lines)] // One match is the exhaustive stable pointer-record encoding table.
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
        | TraceRecordKind::PointerTextSelectionStarted { pointer_id }
        | TraceRecordKind::PointerTextSelectionUpdated { pointer_id }
        | TraceRecordKind::PointerTextSelectionEnded { pointer_id }
        | TraceRecordKind::PointerTextSelectionCancelled { pointer_id }
        | TraceRecordKind::PointerInteractionCommitted { pointer_id } => {
            field_u64(output, "pointer_id", pointer_id.get());
        }
        TraceRecordKind::TouchGestureWon {
            pointer_id,
            gesture,
        }
        | TraceRecordKind::TouchGestureCancelled {
            pointer_id,
            gesture,
        }
        | TraceRecordKind::TouchGestureCompleted {
            pointer_id,
            gesture,
        } => {
            field_u64(output, "pointer_id", pointer_id.get());
            output.push(',');
            field_str(output, "gesture", tokens::touch_gesture(*gesture));
        }
        TraceRecordKind::TouchGestureProvisional {
            pointer_id,
            thresholds,
            scroll_candidates,
            selection_candidate,
        } => {
            field_u64(output, "pointer_id", pointer_id.get());
            output.push(',');
            json::name(output, "scroll_threshold");
            json::f32_value(output, thresholds.scroll_movement());
            output.push(',');
            json::name(output, "selection_threshold");
            json::f32_value(output, thresholds.selection_movement());
            output.push(',');
            field_usize(output, "scroll_candidates", *scroll_candidates);
            output.push(',');
            field_bool(output, "selection_candidate", *selection_candidate);
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

#[allow(clippy::too_many_lines)] // One match is the exhaustive stable routed-record encoding table.
fn encode_routed_focus_data(output: &mut String, kind: &TraceRecordKind) -> bool {
    match kind {
        TraceRecordKind::LogicalScrollOwnerApplied {
            evaluation_order,
            offered,
            consumed,
            remainder,
            offset,
            maximum,
        } => {
            field_usize(output, "evaluation_order", *evaluation_order);
            output.push(',');
            json::name(output, "offered");
            value::logical_delta(output, *offered);
            output.push(',');
            json::name(output, "consumed");
            value::logical_delta(output, *consumed);
            output.push(',');
            json::name(output, "remainder");
            value::logical_delta(output, *remainder);
            output.push(',');
            json::name(output, "offset");
            value::logical_delta(output, *offset);
            output.push(',');
            json::name(output, "maximum");
            value::logical_delta(output, *maximum);
        }
        TraceRecordKind::LogicalScrollChainCompleted { remainder } => {
            json::name(output, "remainder");
            value::logical_delta(output, *remainder);
        }
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
        | TraceRecordKind::SemanticDefaultSuppressed { command }
        | TraceRecordKind::EditingDefaultUnavailable { command } => {
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
        TraceRecordKind::EditResolutionVerified { outcome } => {
            field_str(
                output,
                "outcome",
                match outcome {
                    crate::TraceEditResolutionOutcome::Accepted => "accepted",
                    crate::TraceEditResolutionOutcome::Rejected => "rejected",
                    crate::TraceEditResolutionOutcome::Transformed => "transformed",
                },
            );
        }
        TraceRecordKind::InitialEffectsCommitted { count }
        | TraceRecordKind::UpdateEffectsCommitted { count }
        | TraceRecordKind::QueuedWorkCancelled { count } => {
            field_usize(output, "count", *count);
        }
        TraceRecordKind::WorkStartRefused { outcome } => {
            field_str(output, "outcome", tokens::work_start_refusal(*outcome));
        }
        TraceRecordKind::FrameworkServiceResponseOutcome { service, outcome } => {
            encode_framework_service_response_outcome(output, *service, *outcome);
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

fn encode_framework_service_response_outcome(
    output: &mut String,
    service: crate::TraceFrameworkServiceKind,
    outcome: crate::TraceFrameworkServiceOutcome,
) {
    field_str(output, "service", tokens::framework_service_kind(service));
    if let crate::TraceFrameworkServiceKind::ClipboardWriteText(purpose) = service {
        output.push(',');
        field_str(output, "purpose", tokens::clipboard_write_purpose(purpose));
    }
    output.push(',');
    match outcome {
        crate::TraceFrameworkServiceOutcome::Succeeded => {
            field_str(output, "outcome", "succeeded");
        }
        crate::TraceFrameworkServiceOutcome::Failed(failure) => {
            field_str(output, "outcome", "failed");
            output.push(',');
            field_str(
                output,
                "failure",
                tokens::framework_service_failure(failure),
            );
        }
        crate::TraceFrameworkServiceOutcome::ClipboardText {
            classification,
            bytes,
        } => {
            field_str(output, "outcome", "clipboard_text");
            output.push(',');
            field_str(
                output,
                "classification",
                match classification {
                    runenui_core::ClipboardClassification::Unclassified => "unclassified",
                    runenui_core::ClipboardClassification::Public => "public",
                    runenui_core::ClipboardClassification::Sensitive => "sensitive",
                    _ => "unknown",
                },
            );
            output.push(',');
            field_usize(output, "bytes", bytes);
        }
        crate::TraceFrameworkServiceOutcome::DragDrop {
            phase,
            payload,
            items,
            accepted,
        } => {
            field_str(output, "outcome", "drag_drop");
            output.push(',');
            field_str(output, "phase", tokens::drag_drop_phase(phase));
            output.push(',');
            field_str(
                output,
                "payload_kind",
                tokens::drag_drop_payload_kind(payload),
            );
            output.push(',');
            field_u64(output, "items", u64::from(items));
            output.push(',');
            field_bool(output, "accepted", accepted);
        }
    }
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
