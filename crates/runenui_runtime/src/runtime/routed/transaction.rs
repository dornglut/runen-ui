use runenui_core::{
    __runtime::{MountedEffect, PointerCaptureRequest},
    CommandOrigin, InputModality, MonotonicInstant, SemanticActionTarget, WidgetInvalidation,
    WorkSequence,
};

use super::super::CollectedRoutedOutput;
use crate::trace::TraceReservation;
use crate::{MountedNodeId, TraceEventContext, TraceSequence, TraceTarget};

#[derive(Clone, Copy)]
pub(crate) struct RoutedFailureLineage {
    causal_parent: Option<TraceSequence>,
}

impl RoutedFailureLineage {
    pub(crate) const fn new(causal_parent: Option<TraceSequence>) -> Self {
        Self { causal_parent }
    }

    pub(crate) const fn causal_parent(self) -> Option<TraceSequence> {
        self.causal_parent
    }
}

pub(crate) struct RoutedIngressFacts {
    pub(crate) sequence: WorkSequence,
    pub(crate) target: MountedNodeId,
    pub(crate) origin: CommandOrigin,
    pub(crate) instant: MonotonicInstant,
    pub(crate) event: TraceEventContext,
    pub(crate) causal_parent: Option<TraceSequence>,
    pub(crate) trace_reservation: TraceReservation,
    pub(crate) semantic_target: Option<SemanticActionTarget>,
}

impl RoutedIngressFacts {
    pub(crate) const fn new(
        sequence: WorkSequence,
        target: MountedNodeId,
        origin: CommandOrigin,
        instant: MonotonicInstant,
        event: TraceEventContext,
        causal_parent: Option<TraceSequence>,
        trace_reservation: TraceReservation,
    ) -> Self {
        Self {
            sequence,
            target,
            origin,
            instant,
            event,
            causal_parent,
            trace_reservation,
            semantic_target: None,
        }
    }

    pub(crate) fn with_semantic_target(mut self, target: SemanticActionTarget) -> Self {
        self.semantic_target = Some(target);
        self
    }
}

pub(crate) struct RoutedFailureFacts {
    pub(in crate::runtime) sequence: WorkSequence,
    pub(in crate::runtime) target: MountedNodeId,
    pub(in crate::runtime) origin: CommandOrigin,
    pub(in crate::runtime) instant: MonotonicInstant,
    pub(in crate::runtime) causal_parent: Option<TraceSequence>,
}

pub(crate) struct RoutedTransaction<Action> {
    pub(crate) sequence: WorkSequence,
    pub(crate) target: MountedNodeId,
    pub(crate) origin: CommandOrigin,
    pub(crate) semantic_target: Option<SemanticActionTarget>,
    pub(crate) instant: MonotonicInstant,
    pub(in crate::runtime) route: Vec<MountedNodeId>,
    pub(in crate::runtime) pointer_callback_targets: Vec<MountedNodeId>,
    pub(crate) target_trace: TraceTarget,
    pub(crate) parent: Option<TraceSequence>,
    pub(in crate::runtime) remaining_outputs: usize,
    pub(crate) remaining_default_commands: usize,
    pub(in crate::runtime) propagation_stopped: bool,
    pub(crate) default_prevented: bool,
    pub(in crate::runtime) collecting_notification_outputs: bool,
    pub(in crate::runtime) notification_outputs: Vec<CollectedRoutedOutput<Action>>,
    pub(in crate::runtime) routed_outputs: Vec<CollectedRoutedOutput<Action>>,
    pub(crate) default_outputs: Vec<CollectedRoutedOutput<Action>>,
    pub(in crate::runtime) mounted_work: Vec<(MountedNodeId, MountedEffect<Action>)>,
    pub(in crate::runtime) subscription_dirty: Vec<MountedNodeId>,
    pub(in crate::runtime) pointer_capture_requests: Vec<PointerCaptureRequest>,
    pub(in crate::runtime) invalidation: WidgetInvalidation,
    pub(in crate::runtime) focus_before: Option<MountedNodeId>,
    pub(crate) failure_current_target: Option<MountedNodeId>,
    pub(in crate::runtime) pending_modality: InputModality,
}

impl<Action> RoutedTransaction<Action> {
    pub(crate) fn failure_facts(&self) -> RoutedFailureFacts {
        RoutedFailureFacts {
            sequence: self.sequence,
            target: self.target.clone(),
            origin: self.origin,
            instant: self.instant,
            causal_parent: self.parent,
        }
    }

    pub(super) fn subscription_credit(&self, owner: &MountedNodeId) -> usize {
        usize::from(self.subscription_dirty.contains(owner))
    }

    pub(super) fn output_allowance(&self, owner: &MountedNodeId) -> usize {
        self.remaining_outputs
            .checked_add(self.subscription_credit(owner))
            .unwrap_or_else(|| unreachable!("one coalescing credit fits"))
    }

    pub(crate) const fn consume_mandatory_default_command(
        &mut self,
    ) -> Result<(), crate::TraceRoutedIntegrityFailure> {
        let Some(remaining) = self.remaining_default_commands.checked_sub(1) else {
            return Err(crate::TraceRoutedIntegrityFailure::OutputAllowanceExceeded);
        };
        self.remaining_default_commands = remaining;
        Ok(())
    }
}
