#[path = "../src/app.rs"]
mod app;
#[path = "../src/ui.rs"]
mod ui;

use std::time::Instant;

use app::{Counter, CounterAction, CounterApp};
use runenui_core::UiApp;
use runenui_runtime::{
    AppRuntime, PumpBudget, RuntimeConfig, RuntimeStatus, SubmitActionErrorKind, TraceConfig,
};

#[test]
fn accepted_counter_consumer_measures_reserved_roots_and_drain_cost() {
    let full = PumpBudget::new(usize::MAX, usize::MAX, usize::MAX, usize::MAX);
    for trace_capacity in [0, 4096] {
        let started = Instant::now();
        let mut runtime = AppRuntime::<CounterApp>::mount_with_config(
            crate::app::CounterAppState::new(Counter::new()),
            RuntimeConfig::default().with_trace_config(TraceConfig::new(trace_capacity)),
        )
        .unwrap_or_else(|error| unreachable!("counter initial profile fits: {error}"));
        assert!(
            runtime
                .pump(full)
                .unwrap_or_else(|_| unreachable!())
                .report()
                .is_quiescent()
        );
        let environment = ui::style_environment();
        runtime
            .publish_surface(&runenui_runtime::SurfaceBuildContext::tight(
                &environment,
                runenui_runtime::LogicalSize::try_new(240.0, 160.0)
                    .unwrap_or_else(|_| unreachable!()),
            ))
            .unwrap_or_else(|error| unreachable!("counter publishes: {error:?}"));
        let mount_us = started.elapsed().as_micros();
        let credits = CounterApp::action_producer_budget(runtime.state(), &CounterAction::Reset)
            .queue_credits();
        let mut accepted = 0;
        let refused = loop {
            match runtime.submit_action(CounterAction::Reset) {
                Ok(_) => {
                    accepted += 1;
                    assert!(accepted <= 4096);
                }
                Err(error) => break error,
            }
        };
        assert_eq!(refused.kind(), SubmitActionErrorKind::Full);
        assert_eq!(refused.into_action(), CounterAction::Reset);
        assert_eq!(
            accepted,
            RuntimeConfig::default().queue_capacity() / credits
        );
        let started = Instant::now();
        let drained = runtime.pump(full).unwrap_or_else(|_| unreachable!());
        let drain_us = started.elapsed().as_micros();
        assert!(drained.report().is_quiescent());
        assert_eq!(drained.report().processed_envelopes(), accepted);
        assert_eq!(runtime.status(), RuntimeStatus::Running);
        assert_eq!(runtime.state().counter.count, 0);
        eprintln!(
            "U475_COUNTER trace={trace_capacity} mounted={} credits_per_root={credits} accepted_roots={accepted} mount_us={mount_us} drain_us={drain_us}",
            runtime.index().nodes().len()
        );
        runtime.shutdown().unwrap_or_else(|_| unreachable!());
    }
}
