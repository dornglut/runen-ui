use runenui_core::{HostProtocol, PointerCaptureKind, PointerId, WorkSequence};

use super::super::PointerRegistry;
use crate::{
    MountedNodeId, TraceRecordKind, TraceSequence,
    mounted::TargetStatus,
    runtime::{MandatoryTracePlan, Runtime},
};

#[derive(Clone, Copy)]
struct PointerCleanup {
    pointer_id: PointerId,
    pressed: bool,
    capture: bool,
    physical_path: bool,
}

impl PointerCleanup {
    const fn reconciliation_fact_count(self) -> usize {
        if self.capture { 2 } else { 1 }
    }

    const fn closure_fact_count(self) -> usize {
        if self.capture { 3 } else { 2 }
    }
}

#[derive(Clone)]
struct PointerReconciliationSnapshot {
    pointer_id: PointerId,
    pressed_owner: Option<MountedNodeId>,
    capture_owner: Option<MountedNodeId>,
    physical_path: Vec<MountedNodeId>,
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
            parent = self.record_pointer_cleanup(cleanup, Some(sequence), parent);
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
                (pressed || capture || physical_path).then_some(PointerCleanup {
                    pointer_id: snapshot.pointer_id,
                    pressed,
                    capture,
                    physical_path,
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
            parent = self.record_pointer_cleanup(cleanup, None, parent);
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

    fn record_pointer_cleanup(
        &mut self,
        cleanup: PointerCleanup,
        work_sequence: Option<WorkSequence>,
        mut parent: Option<TraceSequence>,
    ) -> Option<TraceSequence> {
        parent = self.trace.record(
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
        );
        if cleanup.capture {
            parent = self.trace.record(
                TraceRecordKind::PointerCaptureNotificationSuppressed {
                    pointer_id: cleanup.pointer_id,
                    kind: PointerCaptureKind::Lost,
                },
                work_sequence,
                parent,
                None,
                None,
                None,
            );
        }
        parent
    }
}
