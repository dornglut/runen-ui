use runenui_core::{
    __runtime::{MountedEffect, PointerCaptureRequest},
    CommandOrigin, MonotonicInstant, WidgetInvalidation, WorkSequence,
};

use super::super::CollectedRoutedOutput;
use crate::trace::TraceReservation;
use crate::{MountedNodeId, TraceSequence, TraceTarget};

pub(in crate::runtime) struct RoutedIngressFacts {
    pub(in crate::runtime) sequence: WorkSequence,
    pub(in crate::runtime) target: MountedNodeId,
    pub(in crate::runtime) origin: CommandOrigin,
    pub(in crate::runtime) instant: MonotonicInstant,
    pub(in crate::runtime) causal_parent: Option<TraceSequence>,
    pub(in crate::runtime) trace_reservation: TraceReservation,
}

impl RoutedIngressFacts {
    pub(in crate::runtime) const fn new(
        sequence: WorkSequence,
        target: MountedNodeId,
        origin: CommandOrigin,
        instant: MonotonicInstant,
        causal_parent: Option<TraceSequence>,
        trace_reservation: TraceReservation,
    ) -> Self {
        Self {
            sequence,
            target,
            origin,
            instant,
            causal_parent,
            trace_reservation,
        }
    }
}

pub(in crate::runtime) struct RoutedFailureFacts {
    pub(in crate::runtime) sequence: WorkSequence,
    pub(in crate::runtime) target: MountedNodeId,
    pub(in crate::runtime) origin: CommandOrigin,
    pub(in crate::runtime) instant: MonotonicInstant,
    pub(in crate::runtime) causal_parent: Option<TraceSequence>,
}

pub(in crate::runtime) struct RoutedTransaction<Action> {
    pub(in crate::runtime) sequence: WorkSequence,
    pub(in crate::runtime) target: MountedNodeId,
    pub(in crate::runtime) origin: CommandOrigin,
    pub(in crate::runtime) instant: MonotonicInstant,
    pub(in crate::runtime) route: Vec<MountedNodeId>,
    pub(in crate::runtime) pointer_callback_targets: Vec<MountedNodeId>,
    pub(in crate::runtime) target_trace: TraceTarget,
    pub(in crate::runtime) parent: Option<TraceSequence>,
    pub(in crate::runtime) remaining_outputs: usize,
    pub(in crate::runtime) propagation_stopped: bool,
    pub(in crate::runtime) default_prevented: bool,
    pub(in crate::runtime) collecting_notification_outputs: bool,
    pub(in crate::runtime) notification_outputs: Vec<CollectedRoutedOutput<Action>>,
    pub(in crate::runtime) routed_outputs: Vec<CollectedRoutedOutput<Action>>,
    pub(in crate::runtime) default_outputs: Vec<CollectedRoutedOutput<Action>>,
    pub(in crate::runtime) mounted_work: Vec<(MountedNodeId, MountedEffect<Action>)>,
    pub(in crate::runtime) subscription_dirty: Vec<MountedNodeId>,
    pub(in crate::runtime) pointer_capture_requests: Vec<PointerCaptureRequest>,
    pub(in crate::runtime) invalidation: WidgetInvalidation,
    pub(in crate::runtime) failure_current_target: Option<MountedNodeId>,
}

impl<Action> RoutedTransaction<Action> {
    pub(in crate::runtime) fn failure_facts(&self) -> RoutedFailureFacts {
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
