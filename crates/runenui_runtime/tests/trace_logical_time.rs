#![allow(refining_impl_trait)]

use std::time::Duration;

use runenui_core::{Effects, IntoEffects, NoHostProtocol, UiApp, View, text};
use runenui_runtime::{AppRuntime, PumpBudget, TraceRecordKind};

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
        .map(|record| record.instant().unwrap_or_else(|| unreachable!()))
        .collect();
    assert!(instants.windows(2).all(|pair| pair[0] <= pair[1]));
}

#[test]
fn application_transaction_facts_share_one_accepted_instant() {
    let initial_work_runtime = AppRuntime::<LogicalTimeApp>::mount(0);
    let initial_transaction = initial_work_runtime
        .trace()
        .records()
        .find(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::InitialApplicationTransactionStarted
            )
        })
        .unwrap_or_else(|| unreachable!());
    let initial_transaction_instant = initial_transaction
        .instant()
        .unwrap_or_else(|| unreachable!());
    let initial_requested = initial_work_runtime
        .trace()
        .records()
        .find(|record| {
            matches!(record.kind(), TraceRecordKind::WorkRequested)
                && record.causal_parent() == Some(initial_transaction.sequence())
        })
        .unwrap_or_else(|| unreachable!());
    let initial_committed = initial_work_runtime
        .trace()
        .records()
        .find(|record| {
            matches!(record.kind(), TraceRecordKind::WorkGenerationCommitted)
                && record.causal_parent() == Some(initial_requested.sequence())
        })
        .unwrap_or_else(|| unreachable!());
    assert_eq!(
        initial_requested.instant(),
        Some(initial_transaction_instant)
    );
    assert_eq!(
        initial_committed.instant(),
        Some(initial_transaction_instant)
    );

    let mut runtime = AppRuntime::<ApplicationTimeApp>::mount(0);
    let initial_instants: Vec<_> = runtime
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
        .map(|record| record.instant().unwrap_or_else(|| unreachable!()))
        .collect();
    assert_eq!(initial_instants.len(), 3);
    assert!(
        initial_instants
            .iter()
            .all(|instant| *instant == initial_instants[0])
    );

    let retained_before_update = runtime.trace().len();
    runtime
        .advance_time(Duration::from_millis(1))
        .unwrap_or_else(|_| unreachable!());
    runtime.submit_action(()).unwrap_or_else(|_| unreachable!());
    runtime.pump(PumpBudget::new(1, 0, 0, 0));

    let update_records: Vec<_> = runtime
        .trace()
        .records()
        .skip(retained_before_update)
        .collect();
    let update_transaction = update_records
        .iter()
        .copied()
        .find(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::ApplicationActionTransactionStarted
            )
        })
        .unwrap_or_else(|| unreachable!());
    let update_transaction_instant = update_transaction
        .instant()
        .unwrap_or_else(|| unreachable!());
    let update_transaction_sequence = update_transaction.sequence();
    let update_work_sequence = update_transaction.work_sequence();

    let update_summary_instants: Vec<_> = update_records
        .iter()
        .copied()
        .filter(|record| match record.kind() {
            TraceRecordKind::ApplicationActionTransactionStarted => {
                record.sequence() == update_transaction_sequence
            }
            TraceRecordKind::ApplicationStateUpdated
            | TraceRecordKind::TreeReconciled
            | TraceRecordKind::UpdateEffectsCommitted { .. } => {
                record.work_sequence() == update_work_sequence
            }
            TraceRecordKind::SubscriptionDiffCommitted { .. } => {
                record.causal_parent() == Some(update_transaction_sequence)
            }
            _ => false,
        })
        .map(|record| record.instant().unwrap_or_else(|| unreachable!()))
        .collect();
    assert_eq!(update_summary_instants.len(), 5);
    assert!(
        update_summary_instants
            .iter()
            .all(|instant| *instant == update_transaction_instant)
    );
    assert!(update_transaction_instant > initial_instants[0]);

    let update_requested = update_records
        .iter()
        .copied()
        .find(|record| {
            matches!(record.kind(), TraceRecordKind::WorkRequested)
                && record.causal_parent() == Some(update_transaction_sequence)
        })
        .unwrap_or_else(|| unreachable!());
    let update_committed = update_records
        .iter()
        .copied()
        .find(|record| {
            matches!(record.kind(), TraceRecordKind::WorkGenerationCommitted)
                && record.causal_parent() == Some(update_requested.sequence())
        })
        .unwrap_or_else(|| unreachable!());
    assert_eq!(update_requested.instant(), Some(update_transaction_instant));
    assert_eq!(update_committed.instant(), Some(update_transaction_instant));
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
                .then(|| record.instant().unwrap_or_else(|| unreachable!()))
        })
        .unwrap_or_else(|| unreachable!());
    let cancelled_instant = runtime
        .trace()
        .records()
        .find_map(|record| {
            matches!(record.kind(), TraceRecordKind::QueuedWorkCancelled { .. })
                .then(|| record.instant().unwrap_or_else(|| unreachable!()))
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
    let shutdown_instant = shutdown_records[0]
        .instant()
        .unwrap_or_else(|| unreachable!());
    assert!(shutdown_instant >= terminal_instant);

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
