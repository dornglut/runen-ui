#![allow(refining_impl_trait)]

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
