use core::num::NonZeroU64;
use std::collections::VecDeque;

use runenui_core::{CommandOrigin, MonotonicInstant};

use super::{
    admission::{MandatoryTracePlan, TraceReservation},
    model::{
        TraceConfig, TraceRecord, TraceRecordKind, TraceSequence, TraceTarget, TraceWorkIdentity,
    },
};
use crate::{MountedNodeId, ReconciliationGeneration, WorkSequence};

/// One bounded canonical trace store.
#[derive(Debug, Eq, PartialEq)]
pub struct Trace {
    capacity: usize,
    records: VecDeque<TraceRecord>,
    next_sequence: Option<NonZeroU64>,
    reserved_records: usize,
    dropped_before_sequence: Option<TraceSequence>,
}

impl Trace {
    #[must_use]
    pub(crate) const fn new(config: TraceConfig) -> Self {
        let capacity = config.capacity();
        Self {
            capacity,
            records: VecDeque::new(),
            next_sequence: NonZeroU64::new(1),
            reserved_records: 0,
            dropped_before_sequence: None,
        }
    }

    pub(crate) const fn is_enabled(&self) -> bool {
        self.capacity != 0
    }

    pub(crate) fn can_admit(&self, plan: MandatoryTracePlan) -> bool {
        self.can_admit_with_reserved(plan, self.reserved_records)
    }

    fn can_admit_with_reserved(&self, plan: MandatoryTracePlan, reserved: usize) -> bool {
        if !self.is_enabled() {
            return true;
        }
        let Some(required) = reserved.checked_add(plan.records) else {
            return false;
        };
        if required == 0 {
            return true;
        }
        self.next_sequence.is_some_and(|next| {
            u64::try_from(required - 1)
                .ok()
                .and_then(|additional| next.get().checked_add(additional))
                .is_some()
        })
    }

    pub(crate) fn reserve_command_outcome(&mut self) -> Option<TraceReservation> {
        self.reserve_outcome_with_prefix(MandatoryTracePlan::command_acceptance())
    }

    pub(crate) fn reserve_pointer_outcome(&mut self) -> Option<TraceReservation> {
        self.reserve_outcome_with_prefix(MandatoryTracePlan::pointer_acceptance())
    }

    pub(crate) fn reserve_surface_command_outcome(&mut self) -> Option<TraceReservation> {
        self.reserve_outcome_with_prefix(MandatoryTracePlan::surface_command_acceptance())
    }

    fn reserve_outcome_with_prefix(
        &mut self,
        prefix: MandatoryTracePlan,
    ) -> Option<TraceReservation> {
        if !self.is_enabled() {
            return Some(TraceReservation::DISABLED);
        }
        let reserved = self.reserved_records.checked_add(1)?;
        if !self.can_admit_with_reserved(prefix, reserved) {
            return None;
        }
        self.reserved_records = reserved;
        Some(TraceReservation::ACTIVE)
    }

    pub(crate) fn can_replace_reservation(
        &self,
        reservation: TraceReservation,
        plan: MandatoryTracePlan,
    ) -> bool {
        if !reservation.is_active() {
            return self.can_admit(plan);
        }
        let Some(reserved) = self.reserved_records.checked_sub(1) else {
            return false;
        };
        self.can_admit_with_reserved(plan, reserved)
    }

    pub(crate) fn release_reservation(&mut self, reservation: TraceReservation) {
        if reservation.is_active() {
            self.reserved_records = self
                .reserved_records
                .checked_sub(1)
                .unwrap_or_else(|| unreachable!("accepted ingress retains one trace reservation"));
        }
    }

    pub(crate) fn release_reservations(&mut self, count: usize) {
        self.reserved_records = self
            .reserved_records
            .checked_sub(count)
            .unwrap_or_else(|| unreachable!("queued ingress retains exact trace reservations"));
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn record_reserved_event(
        &mut self,
        reservation: TraceReservation,
        kind: TraceRecordKind,
        work_sequence: WorkSequence,
        causal_parent: Option<TraceSequence>,
        target: Option<TraceTarget>,
        instant: MonotonicInstant,
        original_target: &MountedNodeId,
        current_target: Option<&MountedNodeId>,
        origin: CommandOrigin,
    ) -> Option<TraceSequence> {
        self.release_reservation(reservation);
        self.record_event(
            kind,
            work_sequence,
            causal_parent,
            target,
            instant,
            original_target,
            current_target,
            origin,
        )
    }

    pub(crate) fn record_reserved(
        &mut self,
        reservation: TraceReservation,
        kind: TraceRecordKind,
        work_sequence: WorkSequence,
        causal_parent: Option<TraceSequence>,
    ) -> Option<TraceSequence> {
        self.release_reservation(reservation);
        self.record(kind, Some(work_sequence), causal_parent, None, None, None)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn record(
        &mut self,
        kind: TraceRecordKind,
        work_sequence: Option<WorkSequence>,
        causal_parent: Option<TraceSequence>,
        reconciliation_before: Option<ReconciliationGeneration>,
        reconciliation_after: Option<ReconciliationGeneration>,
        target: Option<TraceTarget>,
    ) -> Option<TraceSequence> {
        if self.capacity == 0 {
            return None;
        }
        if !self.can_admit(MandatoryTracePlan::one_fact()) {
            return None;
        }
        let sequence = TraceSequence::new(self.next_sequence?);
        self.next_sequence = sequence.get().checked_add(1).and_then(NonZeroU64::new);
        if self.records.len() == self.capacity
            && let Some(evicted) = self.records.pop_front()
            && let Some(next) = evicted
                .sequence
                .get()
                .checked_add(1)
                .and_then(NonZeroU64::new)
        {
            self.dropped_before_sequence = Some(TraceSequence::new(next));
        }
        self.records.push_back(TraceRecord {
            sequence,
            kind,
            work_sequence,
            causal_parent,
            reconciliation_before,
            reconciliation_after,
            target,
            work: None,
            instant: None,
            original_target: None,
            current_target: None,
            command_origin: None,
        });
        Some(sequence)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn record_event(
        &mut self,
        kind: TraceRecordKind,
        work_sequence: WorkSequence,
        causal_parent: Option<TraceSequence>,
        target: Option<TraceTarget>,
        instant: MonotonicInstant,
        original_target: &MountedNodeId,
        current_target: Option<&MountedNodeId>,
        origin: CommandOrigin,
    ) -> Option<TraceSequence> {
        let sequence = self.record(kind, Some(work_sequence), causal_parent, None, None, target)?;
        if let Some(record) = self.records.back_mut() {
            record.instant = Some(instant);
            record.original_target = Some(original_target.clone());
            record.current_target = current_target.cloned();
            record.command_origin = Some(origin);
        }
        Some(sequence)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn record_work(
        &mut self,
        kind: TraceRecordKind,
        work_sequence: Option<WorkSequence>,
        causal_parent: Option<TraceSequence>,
        reconciliation_before: Option<ReconciliationGeneration>,
        reconciliation_after: Option<ReconciliationGeneration>,
        target: Option<TraceTarget>,
        work: TraceWorkIdentity,
    ) -> Option<TraceSequence> {
        let sequence = self.record(
            kind,
            work_sequence,
            causal_parent,
            reconciliation_before,
            reconciliation_after,
            target,
        )?;
        if let Some(record) = self.records.back_mut() {
            record.work = Some(work);
        }
        Some(sequence)
    }

    /// Borrows canonical records from oldest retained to newest.
    #[must_use]
    pub fn records(&self) -> impl ExactSizeIterator<Item = &TraceRecord> {
        self.records.iter()
    }

    /// Borrows structured record kinds from oldest retained to newest.
    #[must_use]
    pub fn kinds(&self) -> impl ExactSizeIterator<Item = &TraceRecordKind> {
        self.records.iter().map(TraceRecord::kind)
    }

    /// Returns the number of retained records.
    #[must_use]
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Returns whether no records are retained.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Returns the exclusive watermark for records evicted from retention.
    #[must_use]
    pub const fn dropped_before_sequence(&self) -> Option<TraceSequence> {
        self.dropped_before_sequence
    }

    #[cfg(feature = "internal-test-seams")]
    pub(crate) const fn seed_next_sequence_for_test(&mut self, next: u64) {
        self.next_sequence = NonZeroU64::new(next);
    }

    #[cfg(feature = "internal-test-seams")]
    pub(crate) const fn reserved_records_for_test(&self) -> usize {
        self.reserved_records
    }

    #[cfg(feature = "internal-test-seams")]
    pub(crate) const fn next_sequence_for_test(&self) -> Option<u64> {
        match self.next_sequence {
            Some(sequence) => Some(sequence.get()),
            None => None,
        }
    }
}
