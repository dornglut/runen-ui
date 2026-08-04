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
