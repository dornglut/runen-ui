use runenui_core::{CommandOrigin, MonotonicInstant};

use crate::{MountedNodeId, ReconciliationGeneration, WorkSequence};

use super::{
    TraceContext, TracePublicationContext, TraceRecord, TraceRecordKind, TraceSequence,
    TraceSurfaceContext, TraceTarget, TraceWorkIdentity,
};

/// Named reconciliation facts owned by one trace-record construction.
#[derive(Debug)]
pub(super) struct TraceReconciliation {
    pub(super) before: Option<ReconciliationGeneration>,
    pub(super) after: Option<ReconciliationGeneration>,
}

impl TraceReconciliation {
    pub(super) const NONE: Self = Self {
        before: None,
        after: None,
    };

    pub(super) const fn new(
        before: Option<ReconciliationGeneration>,
        after: Option<ReconciliationGeneration>,
    ) -> Self {
        Self { before, after }
    }
}

/// Routed endpoints and origin owned by one routed trace record.
#[derive(Debug)]
pub(super) struct TraceRoutedEndpoints {
    pub(super) original_target: MountedNodeId,
    pub(super) current_target: Option<MountedNodeId>,
    pub(super) command_origin: CommandOrigin,
}

impl TraceRoutedEndpoints {
    pub(super) const fn new(
        original_target: MountedNodeId,
        current_target: Option<MountedNodeId>,
        command_origin: CommandOrigin,
    ) -> Self {
        Self {
            original_target,
            current_target,
            command_origin,
        }
    }
}

/// Complete named input for constructing one immutable canonical trace record.
#[derive(Debug)]
pub(crate) struct TraceRecordDraft {
    pub(super) kind: TraceRecordKind,
    pub(super) work_sequence: Option<WorkSequence>,
    pub(super) causal_parent: Option<TraceSequence>,
    pub(super) reconciliation: TraceReconciliation,
    pub(super) target: Option<TraceTarget>,
    pub(super) work: Option<TraceWorkIdentity>,
    pub(super) logical_time: Option<MonotonicInstant>,
    pub(super) routed: Option<TraceRoutedEndpoints>,
    pub(super) context: TraceContext,
}

impl TraceRecordDraft {
    pub(super) const fn new(kind: TraceRecordKind) -> Self {
        Self {
            kind,
            work_sequence: None,
            causal_parent: None,
            reconciliation: TraceReconciliation::NONE,
            target: None,
            work: None,
            logical_time: None,
            routed: None,
            context: TraceContext::empty(),
        }
    }

    #[must_use]
    pub(crate) fn work_fact(
        kind: TraceRecordKind,
        logical_time: MonotonicInstant,
        work: TraceWorkIdentity,
    ) -> Self {
        let mut draft = Self::new(kind);
        draft.logical_time = Some(logical_time);
        draft.work = Some(work);
        draft
    }

    #[must_use]
    pub(crate) const fn lifecycle_fact(
        kind: TraceRecordKind,
        logical_time: MonotonicInstant,
    ) -> Self {
        let mut draft = Self::new(kind);
        draft.logical_time = Some(logical_time);
        draft
    }

    #[must_use]
    pub(crate) const fn redraw_fact(kind: TraceRecordKind, logical_time: MonotonicInstant) -> Self {
        let mut draft = Self::new(kind);
        draft.logical_time = Some(logical_time);
        draft
    }

    #[must_use]
    pub(crate) const fn application_fact(
        kind: TraceRecordKind,
        logical_time: MonotonicInstant,
    ) -> Self {
        let mut draft = Self::new(kind);
        draft.logical_time = Some(logical_time);
        draft
    }

    #[must_use]
    pub(crate) fn routed_fact(
        kind: TraceRecordKind,
        logical_time: MonotonicInstant,
        context: TraceContext,
    ) -> Self {
        Self::context_fact(kind, logical_time, context)
    }

    #[must_use]
    pub(crate) fn pointer_fact(
        kind: TraceRecordKind,
        logical_time: MonotonicInstant,
        context: TraceContext,
    ) -> Self {
        Self::context_fact(kind, logical_time, context)
    }

    #[must_use]
    pub(crate) fn focus_fact(
        kind: TraceRecordKind,
        logical_time: MonotonicInstant,
        context: TraceContext,
    ) -> Self {
        Self::context_fact(kind, logical_time, context)
    }

    #[must_use]
    pub(crate) fn input_fact(
        kind: TraceRecordKind,
        logical_time: MonotonicInstant,
        context: TraceContext,
    ) -> Self {
        Self::context_fact(kind, logical_time, context)
    }

    #[must_use]
    pub(crate) const fn input_marker(
        kind: TraceRecordKind,
        logical_time: MonotonicInstant,
    ) -> Self {
        let mut draft = Self::new(kind);
        draft.logical_time = Some(logical_time);
        draft
    }

    #[must_use]
    pub(crate) fn automation_fact(
        kind: TraceRecordKind,
        logical_time: MonotonicInstant,
        context: TraceContext,
    ) -> Self {
        Self::context_fact(kind, logical_time, context)
    }

    #[must_use]
    pub(crate) fn action_fact(
        kind: TraceRecordKind,
        logical_time: MonotonicInstant,
        context: TraceContext,
    ) -> Self {
        Self::context_fact(kind, logical_time, context)
    }

    fn context_fact(
        kind: TraceRecordKind,
        logical_time: MonotonicInstant,
        context: TraceContext,
    ) -> Self {
        let mut draft = Self::new(kind);
        draft.logical_time = Some(logical_time);
        draft.context = context;
        draft
    }

    #[must_use]
    pub(crate) fn surface_fact(
        kind: TraceRecordKind,
        logical_time: MonotonicInstant,
        surface: TraceSurfaceContext,
    ) -> Self {
        let mut draft = Self::new(kind);
        draft.logical_time = Some(logical_time);
        draft.context = TraceContext::surface_record(surface);
        draft
    }

    #[must_use]
    pub(crate) fn publication_fact(
        kind: TraceRecordKind,
        logical_time: MonotonicInstant,
        publication: TracePublicationContext,
    ) -> Self {
        let mut draft = Self::new(kind);
        draft.logical_time = Some(logical_time);
        draft.context = TraceContext::publication_record(publication);
        draft
    }

    #[must_use]
    pub(crate) const fn with_work_sequence(mut self, work_sequence: Option<WorkSequence>) -> Self {
        self.work_sequence = work_sequence;
        self
    }

    #[must_use]
    pub(crate) const fn with_causal_parent(mut self, causal_parent: Option<TraceSequence>) -> Self {
        self.causal_parent = causal_parent;
        self
    }

    #[must_use]
    pub(crate) const fn with_reconciliation(
        mut self,
        before: Option<ReconciliationGeneration>,
        after: Option<ReconciliationGeneration>,
    ) -> Self {
        self.reconciliation = TraceReconciliation::new(before, after);
        self
    }

    #[must_use]
    pub(crate) fn with_target(mut self, target: Option<TraceTarget>) -> Self {
        self.target = target;
        self
    }

    #[must_use]
    pub(crate) fn with_routed_endpoints(
        mut self,
        original_target: MountedNodeId,
        current_target: Option<MountedNodeId>,
        command_origin: CommandOrigin,
    ) -> Self {
        self.routed = Some(TraceRoutedEndpoints::new(
            original_target,
            current_target,
            command_origin,
        ));
        self
    }

    pub(super) fn into_record(self, sequence: TraceSequence) -> TraceRecord {
        let (original_target, current_target, command_origin) = match self.routed {
            Some(routed) => (
                Some(routed.original_target),
                routed.current_target,
                Some(routed.command_origin),
            ),
            None => (None, None, None),
        };
        TraceRecord {
            sequence,
            kind: self.kind,
            work_sequence: self.work_sequence,
            causal_parent: self.causal_parent,
            reconciliation_before: self.reconciliation.before,
            reconciliation_after: self.reconciliation.after,
            target: self.target,
            work: self.work,
            instant: self.logical_time,
            original_target,
            current_target,
            command_origin,
            context: self.context,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TraceRecordDraft;
    use crate::TraceRecordKind;

    #[test]
    fn base_draft_has_no_detached_optional_facts() {
        let draft = TraceRecordDraft::new(TraceRecordKind::RuntimeMounted);

        assert_eq!(draft.work_sequence, None);
        assert_eq!(draft.causal_parent, None);
        assert_eq!(draft.reconciliation.before, None);
        assert_eq!(draft.reconciliation.after, None);
        assert_eq!(draft.target, None);
        assert_eq!(draft.work, None);
        assert_eq!(draft.logical_time, None);
        assert!(draft.routed.is_none());
        assert_eq!(draft.context.event(), None);
    }
}
