#![allow(refining_impl_trait)]

use runenui_core::{CommandOrigin, Element, NoHostProtocol, SemanticCommand, UiApp, View, button};
use runenui_runtime::{AppRuntime, PumpBudget, RuntimeConfig, TraceConfig, TraceRecordKind};

struct App;

impl UiApp for App {
    type State = usize;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(_: &Self::State) -> Element<Self::Action> {
        button("Go")
            .id("go")
            .key("go")
            .on_activate(|| ())
            .into_element()
    }

    fn update(state: &mut Self::State, (): Self::Action) {
        *state += 1;
    }
}

#[test]
fn capacity_zero_retains_nothing_and_changes_no_behavior() {
    let config = RuntimeConfig::default().with_trace_config(TraceConfig::new(0));
    let mut runtime = AppRuntime::<App>::mount_with_config(0, config);
    assert!(runtime.trace().is_empty());
    runtime.submit_action(()).unwrap_or_else(|_| unreachable!());
    runtime.pump(PumpBudget::new(2, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(runtime.state(), &1);
    assert!(runtime.trace().is_empty());
    assert_eq!(runtime.trace().dropped_before_sequence(), None);
}

#[test]
fn logical_trace_capacity_does_not_eagerly_reserve() {
    let config = RuntimeConfig::default().with_trace_config(TraceConfig::new(usize::MAX));
    let mut runtime = AppRuntime::<App>::mount_with_config(0, config);

    runtime.submit_action(()).unwrap_or_else(|_| unreachable!());
    let report = runtime.pump(PumpBudget::new(2, usize::MAX, usize::MAX, usize::MAX));

    assert_eq!(report.processed_envelopes(), 2);
    assert_eq!(runtime.state(), &1);
    assert!(!runtime.trace().is_empty());
    assert!(
        runtime
            .trace()
            .kinds()
            .any(|kind| matches!(kind, TraceRecordKind::TreeReconciled))
    );
}

#[test]
fn bounded_retention_evicts_oldest_and_advances_the_exclusive_watermark() {
    let config = RuntimeConfig::default().with_trace_config(TraceConfig::new(3));
    let mut runtime = AppRuntime::<App>::mount_with_config(0, config);
    runtime.submit_action(()).unwrap_or_else(|_| unreachable!());
    runtime.submit_action(()).unwrap_or_else(|_| unreachable!());
    runtime.submit_action(()).unwrap_or_else(|_| unreachable!());
    let sequences: Vec<_> = runtime
        .trace()
        .records()
        .map(|record| record.sequence().get())
        .collect();
    assert_eq!(sequences, [7, 8, 9]);
    assert_eq!(
        runtime
            .trace()
            .dropped_before_sequence()
            .map(runenui_runtime::TraceSequence::get),
        Some(7)
    );

    let one = RuntimeConfig::default().with_trace_config(TraceConfig::new(1));
    let mut one_runtime = AppRuntime::<App>::mount_with_config(0, one);
    one_runtime
        .submit_action(())
        .unwrap_or_else(|_| unreachable!());
    let retained = one_runtime
        .trace()
        .records()
        .next()
        .unwrap_or_else(|| unreachable!());
    assert_eq!(retained.sequence().get(), 7);
    assert!(matches!(
        retained.kind(),
        TraceRecordKind::ActionSubmissionAccepted
    ));
    assert_eq!(
        one_runtime
            .trace()
            .dropped_before_sequence()
            .map(runenui_runtime::TraceSequence::get),
        Some(7)
    );
}

#[test]
fn repeated_eviction_advances_exclusive_watermark_exactly() {
    let config = RuntimeConfig::default().with_trace_config(TraceConfig::new(2));
    let mut runtime = AppRuntime::<App>::mount_with_config(0, config);
    let mut observed = Vec::new();

    for _ in 1..=4 {
        runtime.submit_action(()).unwrap_or_else(|_| unreachable!());
        observed.push(
            runtime
                .trace()
                .dropped_before_sequence()
                .map(runenui_runtime::TraceSequence::get),
        );
    }

    assert_eq!(observed, [Some(6), Some(7), Some(8), Some(9)]);
    assert_eq!(
        runtime
            .trace()
            .records()
            .map(|record| record.sequence().get())
            .collect::<Vec<_>>(),
        [9, 10]
    );
}

#[test]
fn direct_submission_records_work_sequence_without_a_causal_parent() {
    let mut runtime = AppRuntime::<App>::mount(0);
    let work = runtime.submit_action(()).unwrap_or_else(|_| unreachable!());
    let acceptance = runtime
        .trace()
        .records()
        .find(|record| matches!(record.kind(), TraceRecordKind::ActionSubmissionAccepted))
        .unwrap_or_else(|| unreachable!());
    assert_eq!(work.get(), 2);
    assert_eq!(acceptance.work_sequence(), Some(work));
    assert_eq!(acceptance.causal_parent(), None);
    assert_eq!(acceptance.target(), None);
}

#[test]
fn routed_action_acceptance_links_target_collection_and_application_transaction() {
    let mut runtime = AppRuntime::<App>::mount(0);
    runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));
    let target = runtime.index().nodes()[0].id().clone();
    runtime
        .submit_command(
            target.clone(),
            SemanticCommand::Activate,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("the exact live target is accepted"));
    runtime.pump(PumpBudget::new(1, 0, 0, 0));
    assert_eq!(runtime.state(), &0);
    let records: Vec<_> = runtime.trace().records().collect();
    let acceptance = records
        .iter()
        .find(|record| matches!(record.kind(), TraceRecordKind::ActionSubmissionAccepted))
        .unwrap_or_else(|| unreachable!());
    let collection = records
        .iter()
        .find(|record| matches!(record.kind(), TraceRecordKind::RoutedActionCollected))
        .unwrap_or_else(|| unreachable!());
    assert_eq!(acceptance.causal_parent(), Some(collection.sequence()));
    assert_eq!(
        acceptance
            .target()
            .map(runenui_runtime::TraceTarget::mounted_node_id),
        Some(&target)
    );

    runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));
    let started = runtime
        .trace()
        .records()
        .find(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::ApplicationActionTransactionStarted
            )
        })
        .unwrap_or_else(|| unreachable!());
    assert_eq!(
        started
            .reconciliation_before()
            .map(runenui_runtime::ReconciliationGeneration::get),
        Some(1)
    );
    let reconciled = runtime
        .trace()
        .records()
        .find(|record| matches!(record.kind(), TraceRecordKind::TreeReconciled))
        .unwrap_or_else(|| unreachable!());
    assert_eq!(
        reconciled
            .reconciliation_before()
            .map(runenui_runtime::ReconciliationGeneration::get),
        Some(1)
    );
    assert_eq!(
        reconciled
            .reconciliation_after()
            .map(runenui_runtime::ReconciliationGeneration::get),
        Some(2)
    );
}

fn logical_record_order() -> Vec<TraceRecordKind> {
    let mut runtime = AppRuntime::<App>::mount(0);
    runtime.submit_action(()).unwrap_or_else(|_| unreachable!());
    runtime.pump(PumpBudget::new(2, usize::MAX, usize::MAX, usize::MAX));
    runtime.trace().kinds().cloned().collect()
}

#[test]
fn identical_logical_execution_has_identical_record_order() {
    assert_eq!(logical_record_order(), logical_record_order());
}

#[test]
fn shutdown_cancellation_is_visible_in_the_canonical_trace() {
    let mut runtime = AppRuntime::<App>::mount(0);
    runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));
    runtime.submit_action(()).unwrap_or_else(|_| unreachable!());
    runtime.submit_action(()).unwrap_or_else(|_| unreachable!());
    let shutdown = runtime.shutdown();
    assert_eq!(shutdown.cancelled_queued_envelopes(), 2);
    assert!(runtime.trace().records().any(|record| matches!(
        record.kind(),
        TraceRecordKind::RuntimeShutdown {
            cancelled_queued: 2,
            unmounted_lifetimes: 1
        }
    )));
}
