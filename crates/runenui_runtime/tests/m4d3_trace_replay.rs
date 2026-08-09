#![allow(refining_impl_trait)]

use runenui_core::{NoHostProtocol, UiApp, text};
use runenui_runtime::{
    AppRuntime, PumpBudget, RuntimeConfig, TraceConfig, TraceReplay, TraceReplayCompleteness,
};

#[derive(Clone, Copy)]
enum ReplayAction {
    Increment,
}

struct ReplayApp;

impl UiApp for ReplayApp {
    type State = usize;
    type Action = ReplayAction;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl runenui_core::View<Self::Action> {
        text(format!("replay-{state}"))
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            ReplayAction::Increment => *state += 1,
        }
    }
}

fn settle(runtime: &mut AppRuntime<ReplayApp>) {
    let report = runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
    assert!(
        report.is_quiescent(),
        "replay fixture did not settle: {report:?}"
    );
}

fn mounted(trace_capacity: usize) -> AppRuntime<ReplayApp> {
    let mut runtime = AppRuntime::<ReplayApp>::mount_with_config(
        0,
        RuntimeConfig::default().with_trace_config(TraceConfig::new(trace_capacity)),
    );
    settle(&mut runtime);
    runtime
}

#[test]
fn replay_01_real_jsonl_round_trip_preserves_canonical_causal_header_facts() {
    let (jsonl, expected) = {
        let mut runtime = mounted(128);
        runtime
            .submit_action(ReplayAction::Increment)
            .unwrap_or_else(|_| unreachable!("fixture action is admitted"));
        settle(&mut runtime);
        assert_eq!(runtime.state(), &1);

        let expected: Vec<_> = runtime
            .trace()
            .records()
            .map(|record| {
                (
                    record.sequence().get(),
                    record
                        .work_sequence()
                        .map(runenui_runtime::WorkSequence::get),
                    record
                        .causal_parent()
                        .map(runenui_runtime::TraceSequence::get),
                    record
                        .reconciliation_before()
                        .map(runenui_runtime::ReconciliationGeneration::get),
                    record
                        .reconciliation_after()
                        .map(runenui_runtime::ReconciliationGeneration::get),
                    record.instant().map(runenui_core::MonotonicInstant::as_nanos),
                )
            })
            .collect();
        (runtime.trace().export_jsonl(), expected)
    };

    let replay = TraceReplay::parse_jsonl(&jsonl)
        .unwrap_or_else(|error| unreachable!("canonical export must replay: {error}"));
    assert!(replay.is_complete());
    assert_eq!(replay.completeness(), TraceReplayCompleteness::Complete);

    let actual: Vec<_> = replay
        .records()
        .map(|record| {
            (
                record.sequence().get(),
                record
                    .work_sequence()
                    .map(runenui_runtime::TraceReplayWorkSequence::get),
                record
                    .causal_parent()
                    .map(runenui_runtime::TraceReplaySequence::get),
                record.reconciliation_before(),
                record.reconciliation_after(),
                record.instant_nanos(),
            )
        })
        .collect();
    assert_eq!(actual, expected);
    assert!(
        replay
            .records()
            .any(|record| record.kind().as_str() == "application_state_updated")
    );
}

#[test]
fn replay_01_real_evicted_projection_is_explicitly_incomplete_and_contiguous() {
    let jsonl = {
        let mut runtime = mounted(4);
        for _ in 0..3 {
            runtime
                .submit_action(ReplayAction::Increment)
                .unwrap_or_else(|_| unreachable!("fixture action is admitted"));
            settle(&mut runtime);
        }
        assert_eq!(runtime.state(), &3);
        assert!(runtime.trace().dropped_before_sequence().is_some());
        runtime.trace().export_jsonl()
    };

    let replay = TraceReplay::parse_jsonl(&jsonl)
        .unwrap_or_else(|error| unreachable!("evicted canonical export must replay: {error}"));
    let before = replay
        .dropped_before_sequence()
        .unwrap_or_else(|| unreachable!("evicted export owns one watermark"));
    assert_eq!(
        replay.completeness(),
        TraceReplayCompleteness::DroppedPrefix { before }
    );
    assert!(!replay.is_complete());

    let sequences: Vec<_> = replay
        .records()
        .map(|record| record.sequence().get())
        .collect();
    assert_eq!(sequences.first().copied(), Some(before.get()));
    assert!(sequences.windows(2).all(|pair| pair[1] == pair[0] + 1));
}
