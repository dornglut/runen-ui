use runenui_core::{
    CommandOrigin, HostProtocol, PointerCaptureEvent, PointerCaptureKind, PointerId,
    SurfaceInputContext, UiEvent, WorkSequence,
};

use super::super::PointerRegistry;
use crate::{
    MountedNodeId, TraceRecordKind, TraceRoutedIntegrityFailure, TraceSequence,
    mounted::TargetStatus,
    runtime::{MandatoryTracePlan, PointerDispatchFacts, RoutedIngressFacts, Runtime},
    trace::TraceReservation,
};

#[derive(Clone)]
struct PointerCleanup {
    pointer_id: PointerId,
    pressed: bool,
    capture: bool,
    physical_path: bool,
    capture_notification: Option<PointerCaptureNotification>,
}

#[derive(Clone)]
struct PointerCaptureNotification {
    owner: MountedNodeId,
    context: SurfaceInputContext,
    physical_path: Vec<MountedNodeId>,
}

impl PointerCleanup {
    const fn reconciliation_fact_count(&self) -> usize {
        if self.capture { 2 } else { 1 }
    }

    const fn closure_fact_count(&self) -> usize {
        if self.capture { 3 } else { 2 }
    }
}

#[derive(Clone)]
struct PointerReconciliationSnapshot {
    pointer_id: PointerId,
    pressed_owner: Option<MountedNodeId>,
    capture_owner: Option<MountedNodeId>,
    physical_path: Vec<MountedNodeId>,
    surface_context: Option<SurfaceInputContext>,
}

fn ordered_lifecycle_pointer_ids(registry: &PointerRegistry) -> Vec<PointerId> {
    let mut streams = registry
        .streams
        .iter()
        .map(|(pointer_id, stream)| (*pointer_id, stream.registration_sequence()))
        .collect::<Vec<_>>();
    streams.sort_unstable_by_key(|(_, sequence)| *sequence);
    streams
        .into_iter()
        .map(|(pointer_id, _)| pointer_id)
        .collect()
}

impl PointerRegistry {
    fn reconciliation_snapshots(&self) -> Vec<PointerReconciliationSnapshot> {
        ordered_lifecycle_pointer_ids(self)
            .into_iter()
            .map(|pointer_id| {
                let stream = self
                    .streams
                    .get(&pointer_id)
                    .unwrap_or_else(|| unreachable!("planned pointer stream remains registered"));
                PointerReconciliationSnapshot {
                    pointer_id,
                    pressed_owner: stream.pressed_owner().cloned(),
                    capture_owner: stream.capture_owner().cloned(),
                    physical_path: stream.physical_path().to_vec(),
                    surface_context: stream.surface_context().cloned(),
                }
            })
            .collect()
    }

    fn plan_closure_cleanup(&self) -> Vec<PointerCleanup> {
        ordered_lifecycle_pointer_ids(self)
            .into_iter()
            .map(|pointer_id| {
                let stream = self
                    .streams
                    .get(&pointer_id)
                    .unwrap_or_else(|| unreachable!("planned pointer stream remains registered"));
                PointerCleanup {
                    pointer_id,
                    pressed: stream.pressed_owner().is_some(),
                    capture: stream.capture_owner().is_some(),
                    physical_path: !stream.physical_path().is_empty(),
                    capture_notification: None,
                }
            })
            .collect()
    }

    fn commit_reconciled_target_cleanup(&mut self, cleanups: &[PointerCleanup]) {
        for cleanup in cleanups {
            let stream = self
                .streams
                .get_mut(&cleanup.pointer_id)
                .unwrap_or_else(|| unreachable!("preflighted pointer stream remains registered"));
            if cleanup.pressed {
                stream.set_pressed_owner(None);
            }
            if cleanup.capture {
                stream.set_capture_owner(None);
            }
            if cleanup.physical_path {
                stream.physical_path.clear();
            }
        }
    }
}

impl<State, Action, Protocol: HostProtocol> Runtime<State, Action, Protocol> {
    pub(in crate::runtime) fn reconcile_pointer_lifetimes(
        &mut self,
        sequence: WorkSequence,
        mut parent: Option<TraceSequence>,
        unmounted: &[MountedNodeId],
    ) -> Result<Option<TraceSequence>, ()> {
        let cleanups = self.plan_reconciled_pointer_cleanup(unmounted);
        let fact_count = cleanups.iter().try_fold(0usize, |count, cleanup| {
            count.checked_add(cleanup.reconciliation_fact_count())
        });
        let Some(plan) =
            fact_count.and_then(|count| MandatoryTracePlan::one_fact().checked_mul(count))
        else {
            return Err(());
        };
        if !self.trace.can_admit(plan) {
            return Err(());
        }
        self.pointer_registry
            .commit_reconciled_target_cleanup(&cleanups);
        for cleanup in cleanups {
            parent = self.record_pointer_cleanup_fact(&cleanup, Some(sequence), parent);
            if cleanup.capture {
                if let Some(notification) = cleanup.capture_notification.as_ref() {
                    parent = self.trace.record(
                        TraceRecordKind::PointerCaptureTransitionQueued {
                            pointer_id: cleanup.pointer_id,
                            kind: PointerCaptureKind::Lost,
                        },
                        Some(sequence),
                        parent,
                        None,
                        None,
                        Some(self.tree.trace_target(&notification.owner)),
                    );
                    if self
                        .deliver_reconciled_capture_loss(
                            sequence,
                            parent,
                            cleanup.pointer_id,
                            notification,
                        )
                        .is_err()
                    {
                        return Err(());
                    }
                } else {
                    parent = self.record_suppressed_capture_loss(
                        cleanup.pointer_id,
                        Some(sequence),
                        parent,
                    );
                }
            }
        }
        Ok(parent)
    }

    fn plan_reconciled_pointer_cleanup(
        &mut self,
        unmounted: &[MountedNodeId],
    ) -> Vec<PointerCleanup> {
        self.pointer_registry
            .reconciliation_snapshots()
            .into_iter()
            .filter_map(|snapshot| {
                let pressed = snapshot
                    .pressed_owner
                    .as_ref()
                    .is_some_and(|owner| self.pointer_owner_is_ineligible(owner, unmounted));
                let capture = match (
                    snapshot.capture_owner.as_ref(),
                    snapshot.pressed_owner.as_ref(),
                ) {
                    (Some(capture_owner), Some(pressed_owner))
                        if capture_owner == pressed_owner =>
                    {
                        pressed
                    }
                    (Some(owner), _) => self.pointer_owner_is_ineligible(owner, unmounted),
                    (None, _) => false,
                };
                let physical_path = snapshot
                    .physical_path
                    .iter()
                    .any(|target| unmounted.contains(target));
                let capture_notification = if capture {
                    snapshot.capture_owner.as_ref().and_then(|owner| {
                        (self.tree.target_status(owner) == TargetStatus::Live)
                            .then(|| {
                                snapshot.surface_context.clone().map(|context| {
                                    PointerCaptureNotification {
                                        owner: owner.clone(),
                                        context,
                                        physical_path: if physical_path {
                                            Vec::new()
                                        } else {
                                            snapshot.physical_path.clone()
                                        },
                                    }
                                })
                            })
                            .flatten()
                    })
                } else {
                    None
                };
                (pressed || capture || physical_path).then_some(PointerCleanup {
                    pointer_id: snapshot.pointer_id,
                    pressed,
                    capture,
                    physical_path,
                    capture_notification,
                })
            })
            .collect()
    }

    fn pointer_owner_is_ineligible(
        &mut self,
        owner: &MountedNodeId,
        unmounted: &[MountedNodeId],
    ) -> bool {
        if unmounted.contains(owner) || self.tree.target_status(owner) != TargetStatus::Live {
            return true;
        }
        self.tree
            .activation_probe(owner)
            .map_or(true, |activation| {
                !activation.enabled() || !activation.is_actionable()
            })
    }

    pub(in crate::runtime) fn close_pointer_lifetimes(
        &mut self,
        mut parent: Option<TraceSequence>,
    ) -> (usize, Option<TraceSequence>) {
        let cleanups = self.pointer_registry.plan_closure_cleanup();
        let closed = cleanups.len();
        let trace_ready = cleanups
            .iter()
            .try_fold(0usize, |count, cleanup| {
                count.checked_add(cleanup.closure_fact_count())
            })
            .and_then(|count| MandatoryTracePlan::one_fact().checked_mul(count))
            .is_some_and(|plan| self.trace.can_admit(plan));
        self.pointer_registry.clear();
        if !trace_ready {
            return (closed, parent);
        }
        for cleanup in cleanups {
            parent = self.record_pointer_cleanup_fact(&cleanup, None, parent);
            if cleanup.capture {
                parent = self.record_suppressed_capture_loss(cleanup.pointer_id, None, parent);
            }
            parent = self.trace.record(
                TraceRecordKind::PointerStreamClosed {
                    pointer_id: cleanup.pointer_id,
                },
                None,
                parent,
                None,
                None,
                None,
            );
        }
        (closed, parent)
    }

    fn record_pointer_cleanup_fact(
        &mut self,
        cleanup: &PointerCleanup,
        work_sequence: Option<WorkSequence>,
        parent: Option<TraceSequence>,
    ) -> Option<TraceSequence> {
        self.trace.record(
            TraceRecordKind::PointerIntegrityCleanupCommitted {
                pointer_id: cleanup.pointer_id,
                pressed: cleanup.pressed,
                capture: cleanup.capture,
                physical_path: cleanup.physical_path,
            },
            work_sequence,
            parent,
            None,
            None,
            None,
        )
    }

    fn record_suppressed_capture_loss(
        &mut self,
        pointer_id: PointerId,
        work_sequence: Option<WorkSequence>,
        parent: Option<TraceSequence>,
    ) -> Option<TraceSequence> {
        self.trace.record(
            TraceRecordKind::PointerCaptureNotificationSuppressed {
                pointer_id,
                kind: PointerCaptureKind::Lost,
            },
            work_sequence,
            parent,
            None,
            None,
            None,
        )
    }

    fn deliver_reconciled_capture_loss(
        &mut self,
        sequence: WorkSequence,
        parent: Option<TraceSequence>,
        pointer_id: PointerId,
        notification: &PointerCaptureNotification,
    ) -> Result<(), ()> {
        let facts = RoutedIngressFacts::new(
            sequence,
            notification.owner.clone(),
            CommandOrigin::__runtime_pointer(),
            self.now(),
            parent,
            TraceReservation::continuation(),
        );
        let Some(mut transaction) = self.begin_pointer_routed_transaction(
            facts,
            false,
            core::slice::from_ref(&notification.owner),
            &[],
            0,
            MandatoryTracePlan::none(),
        ) else {
            return Err(());
        };
        let capture = PointerCaptureEvent::__runtime_new(
            pointer_id,
            PointerCaptureKind::Lost,
            notification.owner.clone(),
            None,
            notification.context.clone(),
        );
        let physical_target = notification.physical_path.last();
        let dispatch = PointerDispatchFacts::new(
            pointer_id,
            physical_target,
            &notification.physical_path,
            None,
            false,
        );
        if let Err(failure) = self.invoke_target_only_pointer_callback(
            &mut transaction,
            &UiEvent::PointerCapture(capture),
            dispatch,
            &notification.owner,
        ) {
            self.poison_transaction(&transaction, failure, Some(&notification.owner));
            return Err(());
        }
        transaction.pointer_capture_requests.clear();
        let failure = transaction.failure_facts();
        if self.commit_routed_transaction(transaction).is_err() {
            self.poison_routed_event(
                &failure,
                TraceRoutedIntegrityFailure::CommitInvariantFailure,
                Some(&notification.owner),
            );
            return Err(());
        }
        Ok(())
    }
}
