use runenui_core::MonotonicInstant;

use crate::trace::TraceRecordDraft;

use super::{
    HashMap, HostProtocol, Runtime, TraceRecordKind, TraceSequence, TraceWorkIdentity,
    WorkSequence, public_trace_work_identity,
};

impl<State, Action, Protocol: HostProtocol> Runtime<State, Action, Protocol> {
    pub(in crate::runtime) fn trace_work_identity(
        &self,
        generation: crate::work::WorkGeneration,
    ) -> Option<TraceWorkIdentity> {
        self.work
            .trace_identity(generation)
            .map(public_trace_work_identity)
    }

    pub(in crate::runtime) fn record_work_fact(
        &mut self,
        kind: TraceRecordKind,
        identity: TraceWorkIdentity,
    ) -> Option<TraceSequence> {
        let generation = self.work.generation_with_value(identity.generation());
        let causal_parent = generation.and_then(|generation| self.work.trace_parent(generation));
        let logical_time = self.now();
        self.record_work_fact_at(kind, None, causal_parent, identity, logical_time)
    }

    pub(in crate::runtime) fn record_work_fact_from_envelope(
        &mut self,
        kind: TraceRecordKind,
        work_sequence: WorkSequence,
        identity: TraceWorkIdentity,
    ) -> Option<TraceSequence> {
        let generation = self.work.generation_with_value(identity.generation());
        let causal_parent = generation.and_then(|generation| self.work.trace_parent(generation));
        let logical_time = self.now();
        self.record_work_fact_at(
            kind,
            Some(work_sequence),
            causal_parent,
            identity,
            logical_time,
        )
    }

    pub(in crate::runtime) fn record_work_fact_from_envelope_with_parent(
        &mut self,
        kind: TraceRecordKind,
        work_sequence: WorkSequence,
        causal_parent: Option<TraceSequence>,
        identity: TraceWorkIdentity,
    ) -> Option<TraceSequence> {
        let logical_time = self.now();
        self.record_work_fact_at(
            kind,
            Some(work_sequence),
            causal_parent,
            identity,
            logical_time,
        )
    }

    pub(in crate::runtime) fn record_work_fact_with_parent(
        &mut self,
        kind: TraceRecordKind,
        causal_parent: Option<TraceSequence>,
        identity: TraceWorkIdentity,
    ) -> Option<TraceSequence> {
        let logical_time = self.now();
        self.record_work_fact_at(kind, None, causal_parent, identity, logical_time)
    }

    fn record_work_fact_at(
        &mut self,
        kind: TraceRecordKind,
        work_sequence: Option<WorkSequence>,
        causal_parent: Option<TraceSequence>,
        identity: TraceWorkIdentity,
        logical_time: MonotonicInstant,
    ) -> Option<TraceSequence> {
        let generation = self.work.generation_with_value(identity.generation());
        let draft = TraceRecordDraft::work_fact(kind, logical_time, identity)
            .with_work_sequence(work_sequence)
            .with_causal_parent(causal_parent);
        let trace = self.trace.record_draft(draft);
        if let (Some(generation), Some(trace)) = (generation, trace) {
            self.work.set_trace(generation, trace);
        }
        trace
    }

    pub(in crate::runtime) fn record_invalidation_facts(
        &mut self,
        identities: &[TraceWorkIdentity],
        transaction_parent: Option<TraceSequence>,
    ) -> HashMap<u64, (TraceWorkIdentity, Option<TraceSequence>)> {
        let logical_time = self.now();
        identities
            .iter()
            .map(|identity| {
                let bound = self.record_work_fact_at(
                    TraceRecordKind::WorkCancellationBound,
                    None,
                    transaction_parent,
                    identity.clone(),
                    logical_time,
                );
                let invalidated = self.record_work_fact_at(
                    TraceRecordKind::WorkLogicallyInvalidated,
                    None,
                    bound,
                    identity.clone(),
                    logical_time,
                );
                (identity.generation(), (identity.clone(), invalidated))
            })
            .collect()
    }
}
