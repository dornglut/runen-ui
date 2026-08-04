//! Application mutation, reconciliation, and transaction orchestration.

use super::{
    ApplicationActionEnvelope, ApplicationActionOrigin, ApplicationTransactionInput, CommitError,
    HashMap, HashSet, HostProtocol, IntoEffects, LiveSubscription, MandatoryTracePlan,
    MountedNodeId, MutationPhase, OwnedTransactionLedger, PlannedApplicationTransaction,
    PlannedOutput, PlannedStartPayload, PlannedWorkSemanticEvent, ProcessApplicationActionOutcome,
    ReconciliationGeneration, ReconciliationReport, Runtime, RuntimeStatus, RuntimeTerminalReason,
    SubscriptionDiff, SubscriptionSet, TargetStatus, TraceRecordKind, TraceSequence,
    TraceWorkIdentity, TransactionLedger, TransactionPlanError, UiApp, View, WorkFamily, WorkOwner,
    mounted_effect_into_effect, public_trace_work_identity, revoke_generation_authority,
    trace_work_family, trace_work_owner,
};

mod helpers;
mod initial;
mod process;
mod trace;
mod transaction;

pub(in crate::runtime) use helpers::{
    required_application_transaction_trace_records,
    required_application_transaction_trace_records_from_parts,
};
pub(crate) use process::process_application_action;
pub(super) use trace::ApplicationTraceTransaction;
