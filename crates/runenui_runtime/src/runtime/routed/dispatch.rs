use runenui_core::{
    __runtime::RoutedEventOutput, EventPhase, HostProtocol, PointerId, UiEvent, WidgetInvalidation,
};

use super::{
    super::{CollectedRoutedOutput, Runtime, with_routed_parent},
    transaction::RoutedTransaction,
};
use crate::{MountedNodeId, TraceRecordKind, TraceRoutedIntegrityFailure};

#[derive(Clone, Copy)]
pub(in crate::runtime) struct PointerDispatchFacts<'a> {
    pointer_id: PointerId,
    physical_target: Option<&'a MountedNodeId>,
    physical_path: &'a [MountedNodeId],
    related_target: Option<&'a MountedNodeId>,
    default_cancelable: bool,
}

impl<'a> PointerDispatchFacts<'a> {
    pub(in crate::runtime) const fn new(
        pointer_id: PointerId,
        physical_target: Option<&'a MountedNodeId>,
        physical_path: &'a [MountedNodeId],
        related_target: Option<&'a MountedNodeId>,
        default_cancelable: bool,
    ) -> Self {
        Self {
            pointer_id,
            physical_target,
            physical_path,
            related_target,
            default_cancelable,
        }
    }
}

impl<State, Action, Protocol: HostProtocol> Runtime<State, Action, Protocol> {
    pub(in crate::runtime) fn invoke_routed_callbacks(
        &mut self,
        transaction: &mut RoutedTransaction<Action>,
        event: &UiEvent,
        pointer: Option<PointerDispatchFacts<'_>>,
    ) -> Result<(), TraceRoutedIntegrityFailure> {
        let target_index = transaction.route.len() - 1;
        let mut invocations =
            Vec::with_capacity(transaction.route.len().saturating_mul(2).saturating_sub(1));
        invocations.extend(
            transaction.route[..target_index]
                .iter()
                .cloned()
                .map(|id| (EventPhase::Capture, id)),
        );
        invocations.push((EventPhase::Target, transaction.target.clone()));
        invocations.extend(
            transaction.route[..target_index]
                .iter()
                .rev()
                .cloned()
                .map(|id| (EventPhase::Bubble, id)),
        );
        for (phase, current) in invocations {
            if transaction.propagation_stopped {
                break;
            }
            transaction.failure_current_target = Some(current.clone());
            self.invoke_routed_callback(transaction, event, pointer, phase, &current)?;
            transaction.failure_current_target = None;
        }
        Ok(())
    }

    pub(in crate::runtime) fn invoke_target_only_pointer_callback(
        &mut self,
        transaction: &mut RoutedTransaction<Action>,
        event: &UiEvent,
        pointer: PointerDispatchFacts<'_>,
        target: &MountedNodeId,
    ) -> Result<(), TraceRoutedIntegrityFailure> {
        let original_target = core::mem::replace(&mut transaction.target, target.clone());
        let original_target_trace = core::mem::replace(
            &mut transaction.target_trace,
            self.tree.trace_target(target),
        );
        let propagation_stopped = transaction.propagation_stopped;
        let default_prevented = transaction.default_prevented;
        let collecting_notification_outputs = transaction.collecting_notification_outputs;
        transaction.propagation_stopped = false;
        transaction.default_prevented = false;
        transaction.collecting_notification_outputs = true;
        transaction.failure_current_target = Some(target.clone());
        let result = self.invoke_routed_callback(
            transaction,
            event,
            Some(pointer),
            EventPhase::Target,
            target,
        );
        transaction.failure_current_target = None;
        transaction.target = original_target;
        transaction.target_trace = original_target_trace;
        transaction.propagation_stopped = propagation_stopped;
        transaction.default_prevented = default_prevented;
        transaction.collecting_notification_outputs = collecting_notification_outputs;
        result
    }

    fn invoke_routed_callback(
        &mut self,
        transaction: &mut RoutedTransaction<Action>,
        event: &UiEvent,
        pointer: Option<PointerDispatchFacts<'_>>,
        phase: EventPhase,
        current: &MountedNodeId,
    ) -> Result<(), TraceRoutedIntegrityFailure> {
        transaction.parent = self.trace.record_event(
            TraceRecordKind::EventPhaseInvoked { phase },
            transaction.sequence,
            transaction.parent,
            Some(self.tree.trace_target(current)),
            transaction.instant,
            &transaction.target,
            Some(current),
            transaction.origin,
        );
        let was_stopped = transaction.propagation_stopped;
        let was_prevented = transaction.default_prevented;
        let subscription_credit = transaction.subscription_credit(current);
        #[cfg(feature = "internal-test-seams")]
        if self.routed_callback_bridge_failure_for_test {
            return Err(TraceRoutedIntegrityFailure::CallbackBridgeFailure);
        }
        let invocation = match pointer {
            Some(pointer) => self.tree.invoke_pointer_event(
                current,
                &transaction.target,
                pointer.related_target,
                event,
                phase,
                transaction.origin,
                transaction.sequence,
                transaction.instant,
                pointer.pointer_id,
                pointer.physical_target,
                pointer.physical_path,
                pointer.default_cancelable,
                transaction.default_prevented,
                transaction.propagation_stopped,
                transaction.output_allowance(current),
            ),
            None => self.tree.invoke_event(
                current,
                &transaction.target,
                event,
                phase,
                transaction.origin,
                transaction.sequence,
                transaction.instant,
                transaction.default_prevented,
                transaction.propagation_stopped,
                transaction.output_allowance(current),
            ),
        }
        .map_err(|_| TraceRoutedIntegrityFailure::CallbackBridgeFailure)?;
        transaction.remaining_outputs = invocation.output.remaining_outputs;
        transaction.propagation_stopped = invocation.output.propagation_stopped;
        transaction.default_prevented = invocation.output.default_prevented;
        if invocation.output.overflowed {
            return Err(TraceRoutedIntegrityFailure::OutputAllowanceExceeded);
        }
        transaction
            .pointer_capture_requests
            .extend(invocation.output.pointer_capture);
        self.record_event_mutation(
            transaction,
            current,
            invocation.widget.state_changed(),
            invocation.output.invalidation,
            invocation.output.subscription_invalidation,
            subscription_credit,
        );
        self.collect_routed_outputs(transaction, current, invocation.output.ordered);
        transaction.mounted_work.extend(
            invocation
                .output
                .mounted_work
                .into_iter()
                .map(|effect| (current.clone(), effect)),
        );
        self.record_control_changes(transaction, current, was_stopped, was_prevented);
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn record_event_mutation(
        &mut self,
        transaction: &mut RoutedTransaction<Action>,
        current: &MountedNodeId,
        state_changed: bool,
        invalidation: WidgetInvalidation,
        subscription_invalidation: bool,
        subscription_credit: usize,
    ) {
        if state_changed {
            transaction.parent = self.trace.record_event(
                TraceRecordKind::WidgetStateMutated,
                transaction.sequence,
                transaction.parent,
                Some(self.tree.trace_target(current)),
                transaction.instant,
                &transaction.target,
                Some(current),
                transaction.origin,
            );
        }
        if !invalidation.is_empty() {
            transaction.invalidation |= invalidation;
            transaction.parent = self.trace.record_event(
                TraceRecordKind::WidgetInvalidated { invalidation },
                transaction.sequence,
                transaction.parent,
                Some(self.tree.trace_target(current)),
                transaction.instant,
                &transaction.target,
                Some(current),
                transaction.origin,
            );
        }
        if subscription_invalidation && subscription_credit == 0 {
            transaction.subscription_dirty.push(current.clone());
            transaction.parent = self.trace.record_event(
                TraceRecordKind::MountedSubscriptionInvalidated,
                transaction.sequence,
                transaction.parent,
                Some(self.tree.trace_target(current)),
                transaction.instant,
                &transaction.target,
                Some(current),
                transaction.origin,
            );
        } else if !subscription_invalidation && subscription_credit == 1 {
            transaction.remaining_outputs = transaction
                .remaining_outputs
                .checked_sub(1)
                .unwrap_or_else(|| unreachable!("unused coalescing credit remains"));
        }
    }

    fn collect_routed_outputs(
        &mut self,
        transaction: &mut RoutedTransaction<Action>,
        current: &MountedNodeId,
        outputs: Vec<RoutedEventOutput<Action>>,
    ) {
        for output in outputs {
            let (kind, output) = match output {
                RoutedEventOutput::Action(action) => (
                    TraceRecordKind::RoutedActionCollected,
                    CollectedRoutedOutput::Action {
                        action,
                        causal_parent: None,
                        current_target: current.clone(),
                    },
                ),
                RoutedEventOutput::Command {
                    target,
                    command,
                    origin,
                } => (
                    TraceRecordKind::DelegatedCommandCollected { command },
                    CollectedRoutedOutput::Command {
                        target,
                        command,
                        origin,
                        causal_parent: None,
                    },
                ),
            };
            transaction.parent = self.trace.record_event(
                kind,
                transaction.sequence,
                transaction.parent,
                Some(self.tree.trace_target(current)),
                transaction.instant,
                &transaction.target,
                Some(current),
                transaction.origin,
            );
            let output = with_routed_parent(output, transaction.parent);
            if transaction.collecting_notification_outputs {
                transaction.notification_outputs.push(output);
            } else {
                transaction.routed_outputs.push(output);
            }
        }
    }

    fn record_control_changes(
        &mut self,
        transaction: &mut RoutedTransaction<Action>,
        current: &MountedNodeId,
        was_stopped: bool,
        was_prevented: bool,
    ) {
        if !was_stopped && transaction.propagation_stopped {
            transaction.parent = self.trace.record_event(
                TraceRecordKind::PropagationStopped,
                transaction.sequence,
                transaction.parent,
                Some(self.tree.trace_target(current)),
                transaction.instant,
                &transaction.target,
                Some(current),
                transaction.origin,
            );
        }
        if !was_prevented && transaction.default_prevented {
            transaction.parent = self.trace.record_event(
                TraceRecordKind::DefaultPrevented,
                transaction.sequence,
                transaction.parent,
                Some(self.tree.trace_target(current)),
                transaction.instant,
                &transaction.target,
                Some(current),
                transaction.origin,
            );
        }
    }
}
