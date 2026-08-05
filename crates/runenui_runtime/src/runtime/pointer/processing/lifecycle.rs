use runenui_core::{
    CommandOrigin, HostProtocol, InputDeviceId, MonotonicInstant, PointerCaptureEvent,
    PointerCaptureKind, PointerDeviceKind, PointerId, SurfaceInputContext, UiEvent, WorkSequence,
};

use super::super::PointerRegistry;
use crate::{
    MountedNodeId, TraceContext, TraceDeliveryOutcome, TraceEventContext, TraceEventFamily,
    TracePointerCleanup, TracePointerContext, TracePointerPath, TraceRecordKind,
    TraceRouteSnapshot, TraceRoutedIntegrityFailure, TraceSequence, TraceSurfaceContext,
    TraceTargetTransition,
    mounted::TargetStatus,
    runtime::{MandatoryTracePlan, PointerDispatchFacts, RoutedIngressFacts, Runtime},
    trace::{TraceRecordDraft, TraceReservation},
};

#[derive(Clone)]
struct PointerCleanup {
    pointer_id: PointerId,
    device_id: Option<InputDeviceId>,
    device_kind: PointerDeviceKind,
    pressed_owner: Option<MountedNodeId>,
    capture_owner: Option<MountedNodeId>,
    physical_path: Vec<MountedNodeId>,
    surface_context: Option<SurfaceInputContext>,
    pressed: bool,
    capture: bool,
    clear_physical_path: bool,
    capture_notification: Option<PointerCaptureNotification>,
}

#[derive(Clone)]
struct PointerCaptureNotification {
    owner: MountedNodeId,
    context: SurfaceInputContext,
    physical_path: Vec<MountedNodeId>,
}

struct CaptureLossTraceFacts<'a> {
    cleanup: &'a PointerCleanup,
    physical_path: &'a [MountedNodeId],
    surface_context: Option<&'a SurfaceInputContext>,
    delivery: TraceDeliveryOutcome,
    work_sequence: Option<WorkSequence>,
    instant: MonotonicInstant,
}

impl PointerCleanup {
    const fn reconciliation_fact_count(&self) -> usize {
        if self.capture { 2 } else { 1 }
    }

    const fn closure_fact_count(&self) -> usize {
        if self.capture { 3 } else { 2 }
    }

    fn remaining_physical_path(&self) -> &[MountedNodeId] {
        if self.clear_physical_path {
            &[]
        } else {
            &self.physical_path
        }
    }
}

#[derive(Clone)]
struct PointerReconciliationSnapshot {
    pointer_id: PointerId,
    device_id: Option<InputDeviceId>,
    device_kind: PointerDeviceKind,
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
                    device_id: stream.device_id(),
                    device_kind: stream.device_kind(),
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
                    device_id: stream.device_id(),
                    device_kind: stream.device_kind(),
                    pressed_owner: stream.pressed_owner().cloned(),
                    capture_owner: stream.capture_owner().cloned(),
                    physical_path: stream.physical_path().to_vec(),
                    surface_context: stream.surface_context().cloned(),
                    pressed: stream.pressed_owner().is_some(),
                    capture: stream.capture_owner().is_some(),
                    clear_physical_path: !stream.physical_path().is_empty(),
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
            if cleanup.clear_physical_path {
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
        let instant = self.now();
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
            parent = self.record_pointer_cleanup_fact(&cleanup, Some(sequence), parent, instant);
            if cleanup.capture {
                if let Some(notification) = cleanup.capture_notification.as_ref() {
                    parent = self.deliver_reconciled_capture_loss(
                        sequence,
                        parent,
                        instant,
                        &cleanup,
                        notification,
                    )?;
                } else {
                    let facts = CaptureLossTraceFacts {
                        cleanup: &cleanup,
                        physical_path: cleanup.remaining_physical_path(),
                        surface_context: cleanup.surface_context.as_ref(),
                        delivery: TraceDeliveryOutcome::Suppressed,
                        work_sequence: Some(sequence),
                        instant,
                    };
                    parent = self.record_capture_loss_resolution(&facts, parent);
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
                let clear_physical_path = snapshot
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
                                        physical_path: if clear_physical_path {
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
                (pressed || capture || clear_physical_path).then_some(PointerCleanup {
                    pointer_id: snapshot.pointer_id,
                    device_id: snapshot.device_id,
                    device_kind: snapshot.device_kind,
                    pressed_owner: snapshot.pressed_owner,
                    capture_owner: snapshot.capture_owner,
                    physical_path: snapshot.physical_path,
                    surface_context: snapshot.surface_context,
                    pressed,
                    capture,
                    clear_physical_path,
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
        let instant = self.now();
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
            parent = self.record_pointer_cleanup_fact(&cleanup, None, parent, instant);
            if cleanup.capture {
                let facts = CaptureLossTraceFacts {
                    cleanup: &cleanup,
                    physical_path: cleanup.remaining_physical_path(),
                    surface_context: cleanup.surface_context.as_ref(),
                    delivery: TraceDeliveryOutcome::Suppressed,
                    work_sequence: None,
                    instant,
                };
                parent = self.record_capture_loss_resolution(&facts, parent);
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
        instant: MonotonicInstant,
    ) -> Option<TraceSequence> {
        if !self.trace.is_enabled() {
            return parent;
        }
        let physical_path = TracePointerPath::new(
            cleanup
                .physical_path
                .iter()
                .map(|node| self.tree.trace_target(node))
                .collect(),
        );
        let pressed_owner = cleanup.pressed.then(|| {
            TraceTargetTransition::new(
                cleanup
                    .pressed_owner
                    .as_ref()
                    .map(|owner| self.tree.trace_target(owner)),
                None,
            )
        });
        let capture_owner = cleanup.capture.then(|| {
            TraceTargetTransition::new(
                cleanup
                    .capture_owner
                    .as_ref()
                    .map(|owner| self.tree.trace_target(owner)),
                None,
            )
        });
        let pointer = TracePointerContext::stream(
            cleanup.pointer_id,
            cleanup.device_id,
            cleanup.device_kind,
        );
        let surface = cleanup
            .surface_context
            .as_ref()
            .map(TraceSurfaceContext::requested);
        let context = TraceContext::pointer_integrity_cleanup(
            surface,
            pointer,
            physical_path,
            TracePointerCleanup::new(
                pressed_owner,
                capture_owner,
                cleanup.clear_physical_path,
            ),
        );
        self.trace.record_draft(
            TraceRecordDraft::pointer_fact(
                TraceRecordKind::PointerIntegrityCleanupCommitted,
                instant,
                context,
            )
            .with_work_sequence(work_sequence)
            .with_causal_parent(parent),
        )
    }

    fn record_capture_loss_resolution(
        &mut self,
        facts: &CaptureLossTraceFacts<'_>,
        parent: Option<TraceSequence>,
    ) -> Option<TraceSequence> {
        if !self.trace.is_enabled() {
            return parent;
        }
        let owner = facts
            .cleanup
            .capture_owner
            .as_ref()
            .unwrap_or_else(|| unreachable!("capture cleanup retains its previous owner"));
        let target = self.tree.trace_target(owner);
        let route = TraceRouteSnapshot::new(vec![target.clone()], None);
        let physical_path = TracePointerPath::new(
            facts
                .physical_path
                .iter()
                .map(|node| self.tree.trace_target(node))
                .collect(),
        );
        let transition = TraceTargetTransition::new(Some(target.clone()), None);
        let pointer = TracePointerContext::stream(
            facts.cleanup.pointer_id,
            facts.cleanup.device_id,
            facts.cleanup.device_kind,
        );
        let surface = facts.surface_context.map(TraceSurfaceContext::requested);
        let context = TraceContext::pointer_capture_notification(
            surface,
            pointer,
            route,
            physical_path,
            transition,
            facts.delivery,
        );
        self.trace.record_draft(
            TraceRecordDraft::pointer_fact(
                TraceRecordKind::PointerCaptureNotificationResolved {
                    kind: PointerCaptureKind::Lost,
                },
                facts.instant,
                context,
            )
            .with_work_sequence(facts.work_sequence)
            .with_causal_parent(parent)
            .with_target(Some(target)),
        )
    }

    fn deliver_reconciled_capture_loss(
        &mut self,
        sequence: WorkSequence,
        parent: Option<TraceSequence>,
        instant: MonotonicInstant,
        cleanup: &PointerCleanup,
        notification: &PointerCaptureNotification,
    ) -> Result<Option<TraceSequence>, ()> {
        let facts = RoutedIngressFacts::new(
            sequence,
            notification.owner.clone(),
            CommandOrigin::__runtime_pointer(),
            instant,
            TraceEventContext::new(TraceEventFamily::PointerCapture, false),
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
            false,
        ) else {
            return Err(());
        };
        let capture = PointerCaptureEvent::__runtime_new(
            cleanup.pointer_id,
            PointerCaptureKind::Lost,
            notification.owner.clone(),
            None,
            notification.context.clone(),
        );
        let physical_target = notification.physical_path.last();
        let dispatch = PointerDispatchFacts::new(
            cleanup.pointer_id,
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
        let capture_facts = CaptureLossTraceFacts {
            cleanup,
            physical_path: &notification.physical_path,
            surface_context: Some(&notification.context),
            delivery: TraceDeliveryOutcome::Delivered,
            work_sequence: Some(sequence),
            instant,
        };
        transaction.parent = self.record_capture_loss_resolution(&capture_facts, transaction.parent);
        let resolution = transaction.parent;
        let failure = transaction.failure_facts();
        if self.commit_routed_transaction(transaction).is_err() {
            self.poison_routed_event(
                &failure,
                TraceRoutedIntegrityFailure::CommitInvariantFailure,
                Some(&notification.owner),
            );
            return Err(());
        }
        Ok(resolution)
    }
}
