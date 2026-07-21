use runenui_core::{
    __runtime::MountedEffect, CommandOrigin, MonotonicInstant, SemanticCommand, WidgetInvalidation,
    WorkSequence,
};

use super::super::CollectedRoutedOutput;
use crate::trace::TraceReservation;
use crate::{MountedNodeId, TraceSequence, TraceTarget, queue::SemanticCommandEnvelope};

pub(super) struct RoutedCommandFacts {
    pub(super) sequence: WorkSequence,
    pub(super) target: MountedNodeId,
    pub(super) command: SemanticCommand,
    pub(super) origin: CommandOrigin,
    pub(super) instant: MonotonicInstant,
    pub(super) causal_parent: Option<TraceSequence>,
    pub(super) trace_reservation: TraceReservation,
}

pub(super) struct RoutedFailureFacts {
    pub(super) sequence: WorkSequence,
    pub(super) target: MountedNodeId,
    pub(super) origin: CommandOrigin,
    pub(super) instant: MonotonicInstant,
    pub(super) causal_parent: Option<TraceSequence>,
}

impl From<SemanticCommandEnvelope> for RoutedCommandFacts {
    fn from(envelope: SemanticCommandEnvelope) -> Self {
        let SemanticCommandEnvelope {
            sequence,
            target,
            command,
            origin,
            instant,
            causal_parent,
            trace_reservation,
        } = envelope;
        Self {
            sequence,
            target,
            command,
            origin,
            instant,
            causal_parent,
            trace_reservation,
        }
    }
}

pub(super) struct RoutedTransaction<Action> {
    pub(super) sequence: WorkSequence,
    pub(super) target: MountedNodeId,
    pub(super) command: SemanticCommand,
    pub(super) origin: CommandOrigin,
    pub(super) instant: MonotonicInstant,
    pub(super) route: Vec<MountedNodeId>,
    pub(super) target_trace: TraceTarget,
    pub(super) parent: Option<TraceSequence>,
    pub(super) remaining_outputs: usize,
    pub(super) propagation_stopped: bool,
    pub(super) default_prevented: bool,
    pub(super) routed_outputs: Vec<CollectedRoutedOutput<Action>>,
    pub(super) default_outputs: Vec<CollectedRoutedOutput<Action>>,
    pub(super) mounted_work: Vec<(MountedNodeId, MountedEffect<Action>)>,
    pub(super) subscription_dirty: Vec<MountedNodeId>,
    pub(super) invalidation: WidgetInvalidation,
    pub(super) failure_current_target: Option<MountedNodeId>,
}

impl<Action> RoutedTransaction<Action> {
    pub(super) fn failure_facts(&self) -> RoutedFailureFacts {
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
}
