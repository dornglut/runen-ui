use runenui_core::{
    CommandDerivation, CompositionCancelReason, EventPhase, EventSource, FocusBoundaryPolicy,
    FocusDirection, FocusEventKind, FocusReason, InputModality, PointerBoundaryKind,
    PointerCaptureKind, PointerDeviceKind, PointerPhase,
};

use crate::{
    RuntimeTerminalReason, SurfacePhase, TraceActionCategory, TraceAutomationRecordRole,
    TraceDeliveryOutcome, TraceEventFamily, TraceFocusBoundaryOutcome, TraceFocusRecordRole,
    TraceInputRecordRole, TracePointerCaptureRequestKind, TracePointerCaptureRequestRejection,
    TracePointerRecordRole, TracePointerRejection, TraceRoutedAdmissionRejection,
    TraceRoutedIntegrityFailure, TraceSinkDeliveryOutcome, TraceSpaceCleanupReason,
    TraceSurfaceIngressKind, TraceSurfaceRejection, TraceSurfaceSnapshotKind, TraceTargetRejection,
    TraceTimerTerminalOutcome, TraceWorkFamily, TraceWorkStartRefusal,
};

pub(super) const fn event_phase(value: EventPhase) -> &'static str {
    match value {
        EventPhase::Capture => "capture",
        EventPhase::Target => "target",
        EventPhase::Bubble => "bubble",
        _ => "unknown",
    }
}

pub(super) const fn event_source(value: EventSource) -> &'static str {
    match value {
        EventSource::Programmatic => "programmatic",
        EventSource::Automation => "automation",
        EventSource::Accessibility => "accessibility",
        EventSource::Controller => "controller",
        EventSource::Pointer => "pointer",
        EventSource::Keyboard => "keyboard",
        _ => "unknown",
    }
}

pub(super) const fn command_derivation(value: CommandDerivation) -> &'static str {
    match value {
        CommandDerivation::Direct => "direct",
        CommandDerivation::Delegated => "delegated",
        CommandDerivation::SemanticDefault => "semantic_default",
        _ => "unknown",
    }
}

pub(super) const fn focus_boundary_policy(value: FocusBoundaryPolicy) -> &'static str {
    match value {
        FocusBoundaryPolicy::Delegate => "delegate",
        FocusBoundaryPolicy::Trap => "trap",
        FocusBoundaryPolicy::Stop => "stop",
        FocusBoundaryPolicy::Wrap => "wrap",
        FocusBoundaryPolicy::LogicalScroll => "logical_scroll",
        _ => "unknown",
    }
}

pub(super) const fn focus_direction(value: FocusDirection) -> &'static str {
    match value {
        FocusDirection::Left => "left",
        FocusDirection::Right => "right",
        FocusDirection::Up => "up",
        FocusDirection::Down => "down",
        _ => "unknown",
    }
}

pub(super) const fn focus_reason(value: FocusReason) -> &'static str {
    match value {
        FocusReason::Pointer => "pointer",
        FocusReason::LinearNavigation => "linear_navigation",
        FocusReason::DirectionalNavigation => "directional_navigation",
        FocusReason::ProgrammaticRequest => "programmatic_request",
        FocusReason::Removal => "removal",
        FocusReason::Disablement => "disablement",
        FocusReason::RememberedRestoration => "remembered_restoration",
        FocusReason::Shutdown => "shutdown",
        _ => "unknown",
    }
}

pub(super) const fn focus_event_kind(value: FocusEventKind) -> &'static str {
    match value {
        FocusEventKind::Out => "out",
        FocusEventKind::In => "in",
        _ => "unknown",
    }
}

pub(super) const fn input_modality(value: InputModality) -> &'static str {
    match value {
        InputModality::Pointer => "pointer",
        InputModality::Keyboard => "keyboard",
        InputModality::Controller => "controller",
        InputModality::Accessibility => "accessibility",
        InputModality::Automation => "automation",
        InputModality::Programmatic => "programmatic",
        _ => "unknown",
    }
}

pub(super) const fn pointer_phase(value: PointerPhase) -> &'static str {
    match value {
        PointerPhase::Down => "down",
        PointerPhase::Move => "move",
        PointerPhase::Up => "up",
        PointerPhase::Cancel => "cancel",
        PointerPhase::Wheel => "wheel",
        _ => "unknown",
    }
}

pub(super) const fn pointer_device_kind(value: PointerDeviceKind) -> &'static str {
    match value {
        PointerDeviceKind::Mouse => "mouse",
        PointerDeviceKind::Touch => "touch",
        PointerDeviceKind::Pen => "pen",
        PointerDeviceKind::Other => "other",
        _ => "unknown",
    }
}

pub(super) const fn pointer_boundary_kind(value: PointerBoundaryKind) -> &'static str {
    match value {
        PointerBoundaryKind::Enter => "enter",
        PointerBoundaryKind::Leave => "leave",
        _ => "unknown",
    }
}

pub(super) const fn pointer_capture_kind(value: PointerCaptureKind) -> &'static str {
    match value {
        PointerCaptureKind::Gained => "gained",
        PointerCaptureKind::Lost => "lost",
        _ => "unknown",
    }
}

pub(super) const fn composition_cancel_reason(value: CompositionCancelReason) -> &'static str {
    match value {
        CompositionCancelReason::FocusTransfer => "focus_transfer",
        CompositionCancelReason::Removal => "removal",
        CompositionCancelReason::Replacement => "replacement",
        CompositionCancelReason::Disablement => "disablement",
        CompositionCancelReason::Explicit => "explicit",
        CompositionCancelReason::Shutdown => "shutdown",
        _ => "unknown",
    }
}

pub(super) const fn runtime_terminal_reason(value: RuntimeTerminalReason) -> &'static str {
    match value {
        RuntimeTerminalReason::WorkSequenceExhausted => "work_sequence_exhausted",
        RuntimeTerminalReason::WorkGenerationExhausted => "work_generation_exhausted",
        RuntimeTerminalReason::ReconciliationGenerationExhausted => {
            "reconciliation_generation_exhausted"
        }
        RuntimeTerminalReason::MountedIdentityExhausted => "mounted_identity_exhausted",
        RuntimeTerminalReason::TraceSequenceExhausted => "trace_sequence_exhausted",
        RuntimeTerminalReason::Poisoned => "poisoned",
    }
}

pub(super) const fn event_family(value: TraceEventFamily) -> &'static str {
    match value {
        TraceEventFamily::SemanticCommand => "semantic_command",
        TraceEventFamily::Pointer => "pointer",
        TraceEventFamily::PointerBoundary => "pointer_boundary",
        TraceEventFamily::PointerCapture => "pointer_capture",
        TraceEventFamily::Focus => "focus",
        TraceEventFamily::Keyboard => "keyboard",
        TraceEventFamily::CommittedText => "committed_text",
        TraceEventFamily::Composition => "composition",
    }
}

pub(super) const fn pointer_record_role(value: TracePointerRecordRole) -> &'static str {
    match value {
        TracePointerRecordRole::Observation => "observation",
        TracePointerRecordRole::BoundaryPlan => "boundary_plan",
        TracePointerRecordRole::BoundaryNotification => "boundary_notification",
        TracePointerRecordRole::CaptureNotification => "capture_notification",
        TracePointerRecordRole::CaptureRequestRejection => "capture_request_rejection",
        TracePointerRecordRole::Cleanup => "cleanup",
    }
}

pub(super) const fn focus_record_role(value: TraceFocusRecordRole) -> &'static str {
    match value {
        TraceFocusRecordRole::Transition => "transition",
        TraceFocusRecordRole::Notification => "notification",
        TraceFocusRecordRole::ModalityChange => "modality_change",
    }
}

pub(super) const fn input_record_role(value: TraceInputRecordRole) -> &'static str {
    match value {
        TraceInputRecordRole::Keyboard => "keyboard",
        TraceInputRecordRole::CommittedText => "committed_text",
        TraceInputRecordRole::CompositionIdentity => "composition_identity",
        TraceInputRecordRole::CompositionUpdate => "composition_update",
        TraceInputRecordRole::CompositionCleanup => "composition_cleanup",
    }
}

pub(super) const fn automation_record_role(value: TraceAutomationRecordRole) -> &'static str {
    match value {
        TraceAutomationRecordRole::Unique => "unique",
        TraceAutomationRecordRole::Missing => "missing",
        TraceAutomationRecordRole::Ambiguous => "ambiguous",
    }
}

pub(super) const fn action_category(value: TraceActionCategory) -> &'static str {
    match value {
        TraceActionCategory::DirectSubmission => "direct_submission",
        TraceActionCategory::RoutedCommand => "routed_command",
        TraceActionCategory::ApplicationEffect => "application_effect",
    }
}

pub(super) const fn delivery_outcome(value: TraceDeliveryOutcome) -> &'static str {
    match value {
        TraceDeliveryOutcome::Delivered => "delivered",
        TraceDeliveryOutcome::Suppressed => "suppressed",
    }
}

pub(super) const fn sink_delivery(value: TraceSinkDeliveryOutcome) -> &'static str {
    match value {
        TraceSinkDeliveryOutcome::Delivered => "delivered",
        TraceSinkDeliveryOutcome::Full => "full",
        TraceSinkDeliveryOutcome::Closed => "closed",
    }
}

pub(super) const fn surface_snapshot(value: TraceSurfaceSnapshotKind) -> &'static str {
    match value {
        TraceSurfaceSnapshotKind::Current => "current",
        TraceSurfaceSnapshotKind::RetainedHistorical => "retained_historical",
    }
}

pub(super) const fn surface_ingress(value: TraceSurfaceIngressKind) -> &'static str {
    match value {
        TraceSurfaceIngressKind::LogicalCoordinate => "logical_coordinate",
        TraceSurfaceIngressKind::ResolvedTarget => "resolved_target",
    }
}

pub(super) const fn target_rejection(value: TraceTargetRejection) -> &'static str {
    match value {
        TraceTargetRejection::Foreign => "foreign",
        TraceTargetRejection::Stale => "stale",
        TraceTargetRejection::Missing => "missing",
    }
}

pub(super) const fn surface_rejection(value: TraceSurfaceRejection) -> &'static str {
    match value {
        TraceSurfaceRejection::ForeignRuntime => "foreign_runtime",
        TraceSurfaceRejection::ForeignSurface => "foreign_surface",
        TraceSurfaceRejection::RetiredGeneration => "retired_generation",
        TraceSurfaceRejection::MissingGeneration => "missing_generation",
        TraceSurfaceRejection::CoordinateRevisionMismatch => "coordinate_revision_mismatch",
        TraceSurfaceRejection::NoTarget => "no_target",
        TraceSurfaceRejection::TargetNotInSnapshot => "target_not_in_snapshot",
        TraceSurfaceRejection::ForeignTarget => "foreign_target",
        TraceSurfaceRejection::StaleTarget => "stale_target",
        TraceSurfaceRejection::MissingTarget => "missing_target",
        TraceSurfaceRejection::QueueFull => "queue_full",
        TraceSurfaceRejection::RuntimeClosed => "runtime_closed",
        TraceSurfaceRejection::RuntimeTerminal => "runtime_terminal",
        TraceSurfaceRejection::WorkSequenceExhausted => "work_sequence_exhausted",
        TraceSurfaceRejection::TraceSequenceExhausted => "trace_sequence_exhausted",
    }
}

pub(super) const fn pointer_rejection(value: TracePointerRejection) -> &'static str {
    match value {
        TracePointerRejection::ForeignRuntime => "foreign_runtime",
        TracePointerRejection::ForeignSurface => "foreign_surface",
        TracePointerRejection::RetiredGeneration => "retired_generation",
        TracePointerRejection::MissingGeneration => "missing_generation",
        TracePointerRejection::CoordinateRevisionMismatch => "coordinate_revision_mismatch",
        TracePointerRejection::NoTarget => "no_target",
        TracePointerRejection::DuplicateStream => "duplicate_stream",
        TracePointerRejection::MissingStream => "missing_stream",
        TracePointerRejection::RegistryFull => "registry_full",
        TracePointerRejection::RegistrationSequenceExhausted => "registration_sequence_exhausted",
        TracePointerRejection::DeviceMismatch => "device_mismatch",
        TracePointerRejection::DeviceKindMismatch => "device_kind_mismatch",
        TracePointerRejection::ForeignStreamSurface => "foreign_stream_surface",
    }
}

pub(super) const fn capture_request_kind(value: TracePointerCaptureRequestKind) -> &'static str {
    match value {
        TracePointerCaptureRequestKind::Capture => "capture",
        TracePointerCaptureRequestKind::Release => "release",
    }
}

pub(super) const fn capture_request_rejection(
    value: TracePointerCaptureRequestRejection,
) -> &'static str {
    match value {
        TracePointerCaptureRequestRejection::PointerMismatch => "pointer_mismatch",
        TracePointerCaptureRequestRejection::TargetNotInTransaction => "target_not_in_transaction",
        TracePointerCaptureRequestRejection::TargetUnavailable => "target_unavailable",
        TracePointerCaptureRequestRejection::ReleaseNotOwner => "release_not_owner",
    }
}

pub(super) const fn routed_integrity_failure(value: TraceRoutedIntegrityFailure) -> &'static str {
    match value {
        TraceRoutedIntegrityFailure::BrokenTopology => "broken_topology",
        TraceRoutedIntegrityFailure::EventBridgeMismatch => "event_bridge_mismatch",
        TraceRoutedIntegrityFailure::CallbackBridgeFailure => "callback_bridge_failure",
        TraceRoutedIntegrityFailure::OutputAllowanceExceeded => "output_allowance_exceeded",
        TraceRoutedIntegrityFailure::SemanticDefaultFailure => "semantic_default_failure",
        TraceRoutedIntegrityFailure::CommitInvariantFailure => "commit_invariant_failure",
    }
}

pub(super) const fn routed_admission_rejection(
    value: TraceRoutedAdmissionRejection,
) -> &'static str {
    match value {
        TraceRoutedAdmissionRejection::TransactionOutputs => "transaction_outputs",
        TraceRoutedAdmissionRejection::WaitingEnvelopes => "waiting_envelopes",
        TraceRoutedAdmissionRejection::LocalTasks => "local_tasks",
        TraceRoutedAdmissionRejection::SendTasks => "send_tasks",
        TraceRoutedAdmissionRejection::Timers => "timers",
        TraceRoutedAdmissionRejection::WorkSequenceExhausted => "work_sequence_exhausted",
        TraceRoutedAdmissionRejection::WorkGenerationExhausted => "work_generation_exhausted",
        TraceRoutedAdmissionRejection::ReconciliationGenerationExhausted => {
            "reconciliation_generation_exhausted"
        }
        TraceRoutedAdmissionRejection::TraceSequenceExhausted => "trace_sequence_exhausted",
        TraceRoutedAdmissionRejection::CheckedArithmeticOverflow => "checked_arithmetic_overflow",
    }
}

pub(super) const fn focus_boundary_outcome(value: TraceFocusBoundaryOutcome) -> &'static str {
    match value {
        TraceFocusBoundaryOutcome::Candidate => "candidate",
        TraceFocusBoundaryOutcome::Delegated => "delegated",
        TraceFocusBoundaryOutcome::Trapped => "trapped",
        TraceFocusBoundaryOutcome::Stopped => "stopped",
        TraceFocusBoundaryOutcome::Wrapped => "wrapped",
        TraceFocusBoundaryOutcome::LogicalScroll => "logical_scroll",
        TraceFocusBoundaryOutcome::Empty => "empty",
    }
}

pub(super) const fn space_cleanup_reason(value: TraceSpaceCleanupReason) -> &'static str {
    match value {
        TraceSpaceCleanupReason::KeyboardCancel => "keyboard_cancel",
        TraceSpaceCleanupReason::FocusTransfer => "focus_transfer",
        TraceSpaceCleanupReason::Removal => "removal",
        TraceSpaceCleanupReason::Replacement => "replacement",
        TraceSpaceCleanupReason::Disablement => "disablement",
        TraceSpaceCleanupReason::CapabilityLoss => "capability_loss",
        TraceSpaceCleanupReason::Terminal => "terminal",
        TraceSpaceCleanupReason::Shutdown => "shutdown",
        TraceSpaceCleanupReason::Drop => "drop",
        TraceSpaceCleanupReason::Release => "release",
    }
}

pub(super) const fn work_family(value: TraceWorkFamily) -> &'static str {
    match value {
        TraceWorkFamily::LocalTask => "local_task",
        TraceWorkFamily::SendTask => "send_task",
        TraceWorkFamily::Timer => "timer",
        TraceWorkFamily::Subscription => "subscription",
        TraceWorkFamily::HostRequest => "host_request",
    }
}

pub(super) const fn work_start_refusal(value: TraceWorkStartRefusal) -> &'static str {
    match value {
        TraceWorkStartRefusal::ExecutorUnavailable => "executor_unavailable",
        TraceWorkStartRefusal::ExecutorFull => "executor_full",
        TraceWorkStartRefusal::ExecutorClosed => "executor_closed",
        TraceWorkStartRefusal::ExecutorRejected => "executor_rejected",
        TraceWorkStartRefusal::SubscriptionUnavailable => "subscription_unavailable",
        TraceWorkStartRefusal::SubscriptionFull => "subscription_full",
        TraceWorkStartRefusal::SubscriptionClosed => "subscription_closed",
        TraceWorkStartRefusal::SubscriptionRejected => "subscription_rejected",
        TraceWorkStartRefusal::TimerZeroInterval => "timer_zero_interval",
        TraceWorkStartRefusal::TimerDeadlineOverflow => "timer_deadline_overflow",
    }
}

pub(super) const fn timer_terminal_outcome(value: TraceTimerTerminalOutcome) -> &'static str {
    match value {
        TraceTimerTerminalOutcome::Completed => "completed",
        TraceTimerTerminalOutcome::RepeatDeadlineOverflow => "repeat_deadline_overflow",
    }
}

pub(super) const fn surface_phase(value: SurfacePhase) -> &'static str {
    match value {
        SurfacePhase::Tree => "tree",
        SurfacePhase::Style => "style",
        SurfacePhase::Layout => "layout",
        SurfacePhase::HitTesting => "hit_testing",
        SurfacePhase::Paint => "paint",
        SurfacePhase::Semantics => "semantics",
        SurfacePhase::Diagnostics => "diagnostics",
        SurfacePhase::FocusValidation => "focus_validation",
    }
}
