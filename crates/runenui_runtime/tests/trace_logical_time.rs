#![allow(refining_impl_trait)]

use std::time::Duration;

use runenui_core::{Effects, IntoEffects, NoHostProtocol, UiApp, View, text};
use runenui_runtime::{AppRuntime, MonotonicInstant, PumpBudget, TraceRecord, TraceRecordKind};

struct LogicalTimeApp;

impl UiApp for LogicalTimeApp {
    type State = usize;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(_: &Self::State) -> impl View<Self::Action> {
        text("logical time")
    }

    fn initial_effects(_: &Self::State) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        Effects::local_task(async { Some(()) })
    }

    fn update(state: &mut Self::State, (): Self::Action) {
        *state += 1;
    }
}

struct ApplicationTimeApp;

impl UiApp for ApplicationTimeApp {
    type State = usize;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(_: &Self::State) -> impl View<Self::Action> {
        text("application time")
    }

    fn update(
        state: &mut Self::State,
        (): Self::Action,
    ) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        *state += 1;
        Effects::local_task(async { None::<()> })
    }
}

fn required_instant(record: &TraceRecord) -> MonotonicInstant {
    record.instant().unwrap_or_else(|| unreachable!())
}

#[test]
fn scheduler_work_facts_retain_monotonic_logical_time() {
    let mut runtime = AppRuntime::<LogicalTimeApp>::mount(0);
    runtime.pump(PumpBudget::new(3, 0, 1, 0));

    let work_records: Vec<_> = runtime
        .trace()
        .records()
        .filter(|record| record.work().is_some())
        .collect();

    assert!(
        work_records
            .iter()
            .any(|record| matches!(record.kind(), TraceRecordKind::WorkRequested))
    );
    assert!(
        work_records
            .iter()
            .any(|record| matches!(record.kind(), TraceRecordKind::WorkStartAccepted))
    );
    assert!(
        work_records
            .iter()
            .any(|record| matches!(record.kind(), TraceRecordKind::LocalWorkReady))
    );
    assert!(work_records.iter().all(|record| record.instant().is_some()));

    let instants: Vec<_> = work_records
        .iter()
        .map(|record| required_instant(record))
        .collect();
    assert!(instants.windows(2).all(|pair| pair[0] <= pair[1]));
}

#[test]
fn initial_application_transaction_facts_share_one_accepted_instant() {
    let runtime = AppRuntime::<LogicalTimeApp>::mount(0);
    let transaction = runtime
        .trace()
        .records()
        .find(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::InitialApplicationTransactionStarted
            )
        })
        .unwrap_or_else(|| unreachable!());
    let transaction_instant = required_instant(transaction);
    let requested = runtime
        .trace()
        .records()
        .find(|record| {
            matches!(record.kind(), TraceRecordKind::WorkRequested)
                && record.causal_parent() == Some(transaction.sequence())
        })
        .unwrap_or_else(|| unreachable!());
    let committed = runtime
        .trace()
        .records()
        .find(|record| {
            matches!(record.kind(), TraceRecordKind::WorkGenerationCommitted)
                && record.causal_parent() == Some(requested.sequence())
        })
        .unwrap_or_else(|| unreachable!());

    assert_eq!(required_instant(requested), transaction_instant);
    assert_eq!(required_instant(committed), transaction_instant);

    let summary_instants: Vec<_> = runtime
        .trace()
        .records()
        .filter(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::InitialApplicationTransactionStarted
                    | TraceRecordKind::InitialEffectsCommitted { .. }
                    | TraceRecordKind::SubscriptionDiffCommitted { .. }
            )
        })
        .map(required_instant)
        .collect();
    assert_eq!(summary_instants.len(), 3);
    assert!(
        summary_instants
            .iter()
            .all(|instant| *instant == transaction_instant)
    );
}

fn update_transaction<'a>(records: &'a [&'a TraceRecord]) -> &'a TraceRecord {
    records
        .iter()
        .copied()
        .find(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::ApplicationActionTransactionStarted
            )
        })
        .unwrap_or_else(|| unreachable!())
}

fn assert_update_summary_time(records: &[&TraceRecord], transaction: &TraceRecord) {
    let sequence = transaction.sequence();
    let work_sequence = transaction.work_sequence();
    let instant = required_instant(transaction);
    let summary_instants: Vec<_> = records
        .iter()
        .copied()
        .filter(|record| match record.kind() {
            TraceRecordKind::ApplicationActionTransactionStarted => record.sequence() == sequence,
            TraceRecordKind::ApplicationStateUpdated
            | TraceRecordKind::TreeReconciled
            | TraceRecordKind::UpdateEffectsCommitted { .. } => {
                record.work_sequence() == work_sequence
            }
            TraceRecordKind::SubscriptionDiffCommitted { .. } => {
                record.causal_parent() == Some(sequence)
            }
            _ => false,
        })
        .map(required_instant)
        .collect();

    assert_eq!(summary_instants.len(), 5);
    assert!(
        summary_instants
            .iter()
            .all(|candidate| *candidate == instant)
    );
}

fn assert_update_work_time(records: &[&TraceRecord], transaction: &TraceRecord) {
    let instant = required_instant(transaction);
    let requested = records
        .iter()
        .copied()
        .find(|record| {
            matches!(record.kind(), TraceRecordKind::WorkRequested)
                && record.causal_parent() == Some(transaction.sequence())
        })
        .unwrap_or_else(|| unreachable!());
    let committed = records
        .iter()
        .copied()
        .find(|record| {
            matches!(record.kind(), TraceRecordKind::WorkGenerationCommitted)
                && record.causal_parent() == Some(requested.sequence())
        })
        .unwrap_or_else(|| unreachable!());

    assert_eq!(required_instant(requested), instant);
    assert_eq!(required_instant(committed), instant);
}

#[test]
fn update_application_transaction_facts_share_one_accepted_instant() {
    let mut runtime = AppRuntime::<ApplicationTimeApp>::mount(0);
    let initial_instant = runtime
        .trace()
        .records()
        .find_map(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::InitialApplicationTransactionStarted
            )
            .then(|| required_instant(record))
        })
        .unwrap_or_else(|| unreachable!());
    let retained_before_update = runtime.trace().len();

    runtime
        .advance_time(Duration::from_millis(1))
        .unwrap_or_else(|_| unreachable!());
    runtime.submit_action(()).unwrap_or_else(|_| unreachable!());
    runtime.pump(PumpBudget::new(8, 0, 0, 0));

    let records: Vec<_> = runtime
        .trace()
        .records()
        .skip(retained_before_update)
        .collect();
    let transaction = update_transaction(&records);
    assert!(required_instant(transaction) > initial_instant);
    assert_update_summary_time(&records, transaction);
    assert_update_work_time(&records, transaction);
}

#[cfg(feature = "internal-test-seams")]
#[test]
fn terminal_and_shutdown_facts_retain_transition_time() {
    let mut runtime = AppRuntime::<LogicalTimeApp>::mount(0);
    runtime.__seed_next_work_sequence_for_test(0);
    assert!(runtime.submit_action(()).is_err());

    let terminal_instant = runtime
        .trace()
        .records()
        .find_map(|record| {
            matches!(record.kind(), TraceRecordKind::RuntimeTerminal { .. })
                .then(|| required_instant(record))
        })
        .unwrap_or_else(|| unreachable!());
    let cancelled_instant = runtime
        .trace()
        .records()
        .find_map(|record| {
            matches!(record.kind(), TraceRecordKind::QueuedWorkCancelled { .. })
                .then(|| required_instant(record))
        })
        .unwrap_or_else(|| unreachable!());
    assert_eq!(cancelled_instant, terminal_instant);

    let report = runtime.shutdown();
    assert!(!report.already_complete());
    let shutdown_records: Vec<_> = runtime
        .trace()
        .records()
        .filter(|record| matches!(record.kind(), TraceRecordKind::RuntimeShutdown { .. }))
        .collect();
    assert_eq!(shutdown_records.len(), 1);
    assert!(required_instant(shutdown_records[0]) >= terminal_instant);

    assert!(runtime.shutdown().already_complete());
    assert_eq!(
        runtime
            .trace()
            .records()
            .filter(|record| matches!(record.kind(), TraceRecordKind::RuntimeShutdown { .. }))
            .count(),
        1
    );
}
