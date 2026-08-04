use runenui_core::{CommandOrigin, MonotonicInstant};

use crate::{MountedNodeId, ReconciliationGeneration, WorkSequence};

use super::{
    TraceContext, TraceRecord, TraceRecordKind, TraceSequence, TraceTarget, TraceWorkIdentity,
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
///
/// Producer-facing family APIs will progressively replace the transitional
/// store entry points with domain-specific construction of this value.
#[derive(Debug)]
pub(super) struct TraceRecordDraft {
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
