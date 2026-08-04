use runenui_core::MonotonicInstant;

use crate::{TraceRecordKind, trace::TraceRecordDraft};

/// One accepted logical-time decision shared by an application transaction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct ApplicationTraceTransaction {
    logical_time: MonotonicInstant,
}

impl ApplicationTraceTransaction {
    pub(super) const fn new(logical_time: MonotonicInstant) -> Self {
        Self { logical_time }
    }

    pub(super) const fn logical_time(self) -> MonotonicInstant {
        self.logical_time
    }

    pub(super) const fn fact(self, kind: TraceRecordKind) -> TraceRecordDraft {
        TraceRecordDraft::application_fact(kind, self.logical_time)
    }
}
