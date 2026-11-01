use core::{fmt, num::NonZeroU64};
use std::{collections::VecDeque, sync::Arc};

use runenui_core::{__runtime::RuntimeNamespace, CommandOrigin, MonotonicInstant};

use super::{
    TracePayloadCapture, TraceSink, TraceSinkDeliveryOutcome, TraceSinkReceiver,
    admission::{MandatoryTracePlan, TraceReservation},
    construction::{TraceReconciliation, TraceRecordDraft, TraceRoutedEndpoints},
    export::encode_trace_jsonl,
    model::{TraceConfig, TraceRecord, TraceRecordKind, TraceSequence, TraceTarget},
};
use crate::{MountedNodeId, ReconciliationGeneration, WorkSequence};

/// One bounded canonical trace store.
pub struct Trace {
    capacity: usize,
    payload_capture: TracePayloadCapture,
    runtime: RuntimeNamespace,
    records: VecDeque<Arc<TraceRecord>>,
    next_sequence: Option<NonZeroU64>,
    reserved_records: usize,
    application_turn_remaining: Option<usize>,
    dropped_before_sequence: Option<TraceSequence>,
    sink: Option<TraceSink>,
}

impl fmt::Debug for Trace {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Trace")
            .field("capacity", &self.capacity)
            .field("payload_capture", &self.payload_capture)
            .field("records", &self.records)
            .field("next_sequence", &self.next_sequence)
            .field("reserved_records", &self.reserved_records)
            .field(
                "application_turn_remaining",
                &self.application_turn_remaining,
            )
            .field("dropped_before_sequence", &self.dropped_before_sequence)
            .field(
                "sink_open",
                &self.sink.as_ref().is_some_and(TraceSink::is_open),
            )
            .finish_non_exhaustive()
    }
}

impl PartialEq for Trace {
    fn eq(&self, other: &Self) -> bool {
        self.capacity == other.capacity
            && self.payload_capture == other.payload_capture
            && self.runtime.__runtime_same_as(&other.runtime)
            && self.records == other.records
            && self.next_sequence == other.next_sequence
            && self.reserved_records == other.reserved_records
            && self.application_turn_remaining == other.application_turn_remaining
            && self.dropped_before_sequence == other.dropped_before_sequence
            && match (&self.sink, &other.sink) {
                (Some(left), Some(right)) => left.state_eq(right),
                (None, None) => true,
                (Some(_), None) | (None, Some(_)) => false,
            }
    }
}

impl Eq for Trace {}

impl Trace {
    #[must_use]
    pub(crate) fn new_for_runtime(config: TraceConfig, runtime: RuntimeNamespace) -> Self {
        let capacity = config.capacity();
        let sink = if capacity == 0 {
            None
        } else {
            config
                .sink_capacity()
                .map(|capacity| TraceSink::bounded(capacity.get(), runtime.clone()))
        };
        Self {
            capacity,
            payload_capture: config.payload_capture(),
            runtime,
            records: VecDeque::new(),
            next_sequence: NonZeroU64::new(1),
            reserved_records: 0,
            application_turn_remaining: None,
            dropped_before_sequence: None,
            sink,
        }
    }

    pub(crate) const fn is_enabled(&self) -> bool {
        self.capacity != 0
    }

    #[cfg(test)]
    pub(crate) const fn reserved_records_for_test(&self) -> usize {
        self.reserved_records
    }

    pub(crate) const fn payload_capture(&self) -> TracePayloadCapture {
        if self.is_enabled() {
            self.payload_capture
        } else {
            TracePayloadCapture::Redacted
        }
    }

    pub(crate) fn can_admit(&self, plan: MandatoryTracePlan) -> bool {
        !self.is_enabled()
            || (self
                .application_turn_remaining
                .is_none_or(|remaining| plan.records <= remaining)
                && self.can_admit_with_reserved(plan, self.reserved_records))
    }

    /// Protects a complete qualified turn from intervening optional diagnostics.
    /// This lends the trace store's existing sequence authority; it creates no
    /// trace buffer or second sequence allocator. Future outcome reservations
    /// transfer one unit out of this budget into `reserved_records`.
    pub(crate) fn begin_application_turn(&mut self, plan: MandatoryTracePlan) -> bool {
        if self.application_turn_remaining.is_some() || !self.can_admit(plan) {
            return false;
        }
        self.application_turn_remaining = Some(plan.records);
        true
    }

    pub(crate) const fn finish_application_turn(&mut self) {
        self.application_turn_remaining = None;
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

    pub(crate) fn reserve_semantic_action_outcome(&mut self) -> Option<TraceReservation> {
        self.reserve_outcome_with_prefix(MandatoryTracePlan::semantic_action_acceptance())
    }

    pub(crate) fn reserve_pointer_outcome(&mut self) -> Option<TraceReservation> {
        self.reserve_outcome_with_prefix(MandatoryTracePlan::pointer_acceptance())
    }

    pub(crate) fn reserve_input_outcome(&mut self) -> Option<TraceReservation> {
        self.reserve_outcome_with_prefix(MandatoryTracePlan::input_acceptance())
    }

    pub(crate) fn reserve_surface_command_outcome(&mut self) -> Option<TraceReservation> {
        self.reserve_outcome_with_prefix(MandatoryTracePlan::surface_command_acceptance())
    }

    pub(crate) fn reserve_surface_publication(&mut self) -> Option<TraceReservation> {
        self.reserve_outcome_with_prefix(MandatoryTracePlan::none())
    }

    fn reserve_outcome_with_prefix(
        &mut self,
        prefix: MandatoryTracePlan,
    ) -> Option<TraceReservation> {
        if !self.is_enabled() {
            return Some(TraceReservation::DISABLED);
        }
        if self.application_turn_remaining.is_some_and(|remaining| {
            prefix
                .records
                .checked_add(1)
                .is_none_or(|required| required > remaining)
        }) {
            return None;
        }
        let reserved = self.reserved_records.checked_add(1)?;
        if !self.can_admit_with_reserved(prefix, reserved) {
            return None;
        }
        self.reserved_records = reserved;
        if let Some(remaining) = &mut self.application_turn_remaining {
            *remaining -= 1;
        }
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
        if self.application_turn_remaining.is_some_and(|remaining| {
            remaining
                .checked_add(1)
                .is_none_or(|available| plan.records > available)
        }) {
            return false;
        }
        self.can_admit_with_reserved(plan, reserved)
    }

    pub(crate) fn release_reservation(&mut self, reservation: TraceReservation) {
        if reservation.is_active() {
            self.reserved_records = self
                .reserved_records
                .checked_sub(1)
                .unwrap_or_else(|| unreachable!("accepted ingress retains one trace reservation"));
            if let Some(remaining) = &mut self.application_turn_remaining {
                *remaining = remaining
                    .checked_add(1)
                    .unwrap_or_else(|| unreachable!("released trace credit was already funded"));
            }
        }
    }

    pub(crate) fn release_reservations(&mut self, count: usize) {
        self.reserved_records = self
            .reserved_records
            .checked_sub(count)
            .unwrap_or_else(|| unreachable!("queued ingress retains exact trace reservations"));
        if let Some(remaining) = &mut self.application_turn_remaining {
            *remaining = remaining
                .checked_add(count)
                .unwrap_or_else(|| unreachable!("released trace credits were already funded"));
        }
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

    pub(crate) fn record_reserved_draft(
        &mut self,
        reservation: TraceReservation,
        draft: TraceRecordDraft,
    ) -> Option<TraceSequence> {
        self.release_reservation(reservation);
        self.record_draft(draft)
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
        let mut draft = TraceRecordDraft::new(kind);
        draft.work_sequence = work_sequence;
        draft.causal_parent = causal_parent;
        draft.reconciliation =
            TraceReconciliation::new(reconciliation_before, reconciliation_after);
        draft.target = target;
        self.record_draft(draft)
    }

    pub(crate) fn record_draft(&mut self, draft: TraceRecordDraft) -> Option<TraceSequence> {
        if self.capacity == 0 {
            return None;
        }
        if !self.can_admit(MandatoryTracePlan::one_fact()) {
            return None;
        }
        if let Some(remaining) = &mut self.application_turn_remaining {
            *remaining -= 1;
        }
        self.record_admitted_draft(draft)
    }

    pub(crate) fn record_optional(
        &mut self,
        kind: TraceRecordKind,
        work_sequence: Option<WorkSequence>,
        causal_parent: Option<TraceSequence>,
        target: Option<TraceTarget>,
    ) -> Option<TraceSequence> {
        let mut draft = TraceRecordDraft::new(kind);
        draft.work_sequence = work_sequence;
        draft.causal_parent = causal_parent;
        draft.target = target;
        self.record_optional_draft(draft)
    }

    fn record_optional_draft(&mut self, draft: TraceRecordDraft) -> Option<TraceSequence> {
        if !self.is_enabled() {
            return None;
        }
        let required = self
            .application_turn_remaining
            .unwrap_or(0)
            .checked_add(1)?;
        if !self.can_admit_with_reserved(
            MandatoryTracePlan::planned_scheduler_transaction(required),
            self.reserved_records,
        ) {
            return None;
        }
        self.record_admitted_draft(draft)
    }

    fn record_admitted_draft(&mut self, draft: TraceRecordDraft) -> Option<TraceSequence> {
        let sequence = TraceSequence::new(self.next_sequence?);
        self.next_sequence = sequence.get().checked_add(1).and_then(NonZeroU64::new);
        if self.records.len() == self.capacity
            && let Some(evicted) = self.records.pop_front()
            && let Some(next) = evicted
                .sequence()
                .get()
                .checked_add(1)
                .and_then(NonZeroU64::new)
        {
            self.dropped_before_sequence = Some(TraceSequence::new(next));
        }

        let mut record = draft.into_record(sequence);
        let sink_reservation = self.sink.as_mut().map(TraceSink::reserve_delivery);
        match sink_reservation {
            Some(Err(outcome)) => {
                record.sink_delivery = Some(outcome);
                if matches!(outcome, TraceSinkDeliveryOutcome::Closed) {
                    self.sink = None;
                }
            }
            Some(Ok(permit)) => {
                record.sink_delivery = Some(TraceSinkDeliveryOutcome::Delivered);
                let shared = Arc::new(record);
                self.records.push_back(shared);
                let outgoing = Arc::clone(
                    self.records
                        .back()
                        .unwrap_or_else(|| unreachable!("just-appended trace record is retained")),
                );
                if let Err(returned) = permit.deliver(outgoing) {
                    drop(returned);
                    self.sink = None;
                    let mut retained = self
                        .records
                        .pop_back()
                        .unwrap_or_else(|| unreachable!("just-appended trace record is retained"));
                    Arc::get_mut(&mut retained)
                        .unwrap_or_else(|| {
                            unreachable!("failed sink delivery leaves sole retained ownership")
                        })
                        .sink_delivery = Some(TraceSinkDeliveryOutcome::Closed);
                    self.records.push_back(retained);
                }
                return Some(sequence);
            }
            None => {}
        }
        self.records.push_back(Arc::new(record));
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
        let mut draft = TraceRecordDraft::new(kind);
        draft.work_sequence = Some(work_sequence);
        draft.causal_parent = causal_parent;
        draft.target = target;
        draft.logical_time = Some(instant);
        draft.routed = Some(TraceRoutedEndpoints::new(
            original_target.clone(),
            current_target.cloned(),
            origin,
        ));
        self.record_draft(draft)
    }

    pub(crate) fn latest_runtime_terminal_sequence(&self) -> Option<TraceSequence> {
        self.records
            .iter()
            .rev()
            .find(|record| matches!(record.kind(), TraceRecordKind::RuntimeTerminal { .. }))
            .map(|record| record.sequence())
    }

    pub(crate) fn take_sink_receiver(&mut self) -> Option<TraceSinkReceiver> {
        self.sink.as_mut().and_then(TraceSink::take_receiver)
    }

    pub(crate) fn close_sink(&mut self) {
        if let Some(sink) = self.sink.as_mut() {
            sink.close();
        }
    }

    /// Projects the retained canonical trace as versioned deterministic JSONL.
    #[must_use]
    pub fn export_jsonl(&self) -> String {
        encode_trace_jsonl(
            &self.runtime,
            self.dropped_before_sequence,
            self.records.iter().map(Arc::as_ref),
        )
    }

    /// Borrows canonical records from oldest retained to newest.
    pub fn records(&self) -> impl ExactSizeIterator<Item = &TraceRecord> {
        self.records.iter().map(Arc::as_ref)
    }

    /// Borrows structured record kinds from oldest retained to newest.
    pub fn kinds(&self) -> impl ExactSizeIterator<Item = &TraceRecordKind> {
        self.records.iter().map(|record| record.kind())
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

#[cfg(test)]
mod application_turn_tests {
    use super::*;

    #[test]
    fn an_existing_outcome_can_lend_its_exact_credit_to_the_turn() {
        let mut trace =
            Trace::new_for_runtime(TraceConfig::new(16), RuntimeNamespace::__runtime_new());
        trace.next_sequence = NonZeroU64::new(u64::MAX - 4);
        let outcome = trace
            .reserve_command_outcome()
            .unwrap_or_else(|| unreachable!("one existing outcome"));
        assert!(trace.begin_application_turn(MandatoryTracePlan::planned_scheduler_transaction(4)));
        assert!(!trace.can_admit(MandatoryTracePlan::planned_scheduler_transaction(5)));
        assert!(trace.can_replace_reservation(
            outcome,
            MandatoryTracePlan::planned_scheduler_transaction(5)
        ));
        trace.release_reservation(outcome);
        for _ in 0..5 {
            assert!(
                trace
                    .record_draft(TraceRecordDraft::new(TraceRecordKind::RuntimeMounted))
                    .is_some()
            );
        }
        trace.finish_application_turn();
        assert_eq!(trace.reserved_records, 0);
        assert!(trace.next_sequence.is_none());
    }

    #[test]
    fn optional_records_cannot_spend_the_whole_turn_floor() {
        let mut trace =
            Trace::new_for_runtime(TraceConfig::new(16), RuntimeNamespace::__runtime_new());
        trace.next_sequence = NonZeroU64::new(u64::MAX - 4);
        assert!(trace.begin_application_turn(MandatoryTracePlan::planned_scheduler_transaction(4)));
        assert!(
            trace
                .record_optional_draft(TraceRecordDraft::new(TraceRecordKind::RuntimeMounted))
                .is_some()
        );
        assert!(
            trace
                .record_optional_draft(TraceRecordDraft::new(TraceRecordKind::RuntimeMounted))
                .is_none()
        );
        for _ in 0..4 {
            assert!(
                trace
                    .record_draft(TraceRecordDraft::new(TraceRecordKind::RuntimeMounted))
                    .is_some()
            );
        }
        assert!(trace.next_sequence.is_none());
        trace.finish_application_turn();
        assert!(trace.application_turn_remaining.is_none());
    }

    #[test]
    fn an_outcome_transfers_its_credit_and_survives_the_turn() {
        let mut trace =
            Trace::new_for_runtime(TraceConfig::new(16), RuntimeNamespace::__runtime_new());
        assert!(trace.begin_application_turn(MandatoryTracePlan::planned_scheduler_transaction(5)));
        let outcome = trace
            .reserve_command_outcome()
            .unwrap_or_else(|| unreachable!("funded future outcome"));
        assert_eq!(trace.application_turn_remaining, Some(4));
        assert_eq!(trace.reserved_records, 1);
        trace.finish_application_turn();
        assert_eq!(trace.reserved_records, 1);
        assert!(
            trace
                .record_reserved_draft(
                    outcome,
                    TraceRecordDraft::new(TraceRecordKind::RuntimeMounted)
                )
                .is_some()
        );
        assert_eq!(trace.reserved_records, 0);
    }
}
