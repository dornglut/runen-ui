#![cfg(feature = "internal-test-seams")]

use std::{cell::Cell, rc::Rc};

use runenui_core::{Element, StyleTokens, View, button};
use runenui_runtime::{
    ActivationResult, AppRuntime, FocusTargetResult, LayoutConstraints, PumpBudget, PumpOutcome,
    RuntimeStatus, RuntimeTerminalReason, SubmitActionErrorKind, SurfaceBuildContext,
    TraceRecordKind, UiApp,
};

#[derive(Debug, Eq, PartialEq)]
struct Action(u32);

#[derive(Debug)]
struct State {
    updates: Vec<u32>,
    factory_calls: Rc<Cell<usize>>,
}

struct App;

impl UiApp for App {
    type State = State;
    type Action = Action;

    fn root(state: &Self::State) -> Element<Self::Action> {
        let calls = Rc::clone(&state.factory_calls);
        button("Activate")
            .id("activate")
            .key("activate")
            .on_activate(move || {
                calls.set(calls.get() + 1);
                Action(7)
            })
            .into_element()
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        state.updates.push(action.0);
    }
}

fn state(calls: &Rc<Cell<usize>>) -> State {
    State {
        updates: Vec::new(),
        factory_calls: Rc::clone(calls),
    }
}

#[test]
fn direct_work_sequence_exhaustion_returns_action_and_closes_mutation() {
    let calls = Rc::new(Cell::new(0));
    let mut runtime = AppRuntime::<App>::mount(state(&calls));
    runtime.__seed_next_work_sequence_for_test(0);

    let Err(rejected) = runtime.submit_action(Action(1)) else {
        unreachable!()
    };
    assert_eq!(
        rejected.kind(),
        SubmitActionErrorKind::Terminal(RuntimeTerminalReason::WorkSequenceExhausted)
    );
    assert_eq!(rejected.into_action(), Action(1));
    assert!(runtime.state().updates.is_empty());
    assert_eq!(calls.get(), 0);
    assert_eq!(
        runtime.status(),
        RuntimeStatus::Terminal(RuntimeTerminalReason::WorkSequenceExhausted)
    );

    let report = runtime.pump(PumpBudget::new(8));
    assert_eq!(report.processed_envelopes(), 0);
    assert_eq!(report.remaining_queued_envelopes(), 0);
    assert_eq!(report.cancelled_by_terminal_transition(), 0);
    assert_eq!(
        report.outcome(),
        PumpOutcome::Terminal(RuntimeTerminalReason::WorkSequenceExhausted)
    );
    let Err(later) = runtime.submit_action(Action(2)) else {
        unreachable!()
    };
    assert_eq!(later.into_action(), Action(2));
}

#[test]
fn activation_work_sequence_exhaustion_preserves_mounted_authority() {
    let calls = Rc::new(Cell::new(0));
    let mut runtime = AppRuntime::<App>::mount(state(&calls));
    let target = runtime.index().nodes()[0].id().clone();
    assert_eq!(
        runtime.set_focus(target.clone()),
        FocusTargetResult::Focused
    );

    let tokens = StyleTokens::new();
    let context = SurfaceBuildContext::new(&tokens, LayoutConstraints::unbounded());
    let publication_before = runtime.publish_surface(&context);
    let report_before = runtime.reconciliation_report().clone();

    runtime.__seed_next_work_sequence_for_test(0);
    assert_eq!(
        runtime.activate_node(&target),
        ActivationResult::Terminal(RuntimeTerminalReason::WorkSequenceExhausted)
    );
    assert_eq!(
        runtime.status(),
        RuntimeStatus::Terminal(RuntimeTerminalReason::WorkSequenceExhausted)
    );
    assert_eq!(calls.get(), 0);
    assert!(runtime.state().updates.is_empty());
    assert_eq!(runtime.focus().focused_node(), Some(&target));
    assert_eq!(runtime.reconciliation_report(), &report_before);
    assert_eq!(runtime.index().nodes()[0].id(), &target);
    assert_eq!(runtime.publish_surface(&context), publication_before);
}

#[test]
fn direct_trace_sequence_exhaustion_returns_action_and_closes_mutation() {
    let calls = Rc::new(Cell::new(0));
    let mut runtime = AppRuntime::<App>::mount(state(&calls));
    runtime.__seed_next_trace_sequence_for_test(0);

    let Err(rejected) = runtime.submit_action(Action(1)) else {
        unreachable!()
    };
    assert_eq!(
        rejected.kind(),
        SubmitActionErrorKind::Terminal(RuntimeTerminalReason::TraceSequenceExhausted)
    );
    assert_eq!(rejected.into_action(), Action(1));
    assert!(runtime.state().updates.is_empty());
    assert_eq!(calls.get(), 0);
    assert_eq!(
        runtime.status(),
        RuntimeStatus::Terminal(RuntimeTerminalReason::TraceSequenceExhausted)
    );
    assert_eq!(runtime.pump(PumpBudget::new(8)).processed_envelopes(), 0);

    let Err(later) = runtime.submit_action(Action(2)) else {
        unreachable!()
    };
    assert_eq!(later.into_action(), Action(2));
}

#[test]
fn trace_exhaustion_during_pump_cancels_failed_and_waiting_envelopes() {
    let calls = Rc::new(Cell::new(0));
    let mut runtime = AppRuntime::<App>::mount(state(&calls));
    let target = runtime.index().nodes()[0].id().clone();
    let tokens = StyleTokens::new();
    let context = SurfaceBuildContext::new(&tokens, LayoutConstraints::unbounded());
    let publication_before = runtime.publish_surface(&context);
    let report_before = runtime.reconciliation_report().clone();

    runtime
        .submit_action(Action(1))
        .unwrap_or_else(|_| unreachable!());
    runtime
        .submit_action(Action(2))
        .unwrap_or_else(|_| unreachable!());
    runtime.__seed_next_trace_sequence_for_test(0);

    let report = runtime.pump(PumpBudget::new(1));
    assert_eq!(report.processed_envelopes(), 1);
    assert_eq!(report.remaining_queued_envelopes(), 0);
    assert_eq!(report.cancelled_by_terminal_transition(), 2);
    assert_eq!(
        report.outcome(),
        PumpOutcome::Terminal(RuntimeTerminalReason::TraceSequenceExhausted)
    );
    assert!(runtime.state().updates.is_empty());
    assert_eq!(runtime.reconciliation_report(), &report_before);
    assert_eq!(runtime.index().nodes()[0].id(), &target);
    assert_eq!(runtime.publish_surface(&context), publication_before);
    assert_eq!(
        runtime.status(),
        RuntimeStatus::Terminal(RuntimeTerminalReason::TraceSequenceExhausted)
    );

    let Err(later) = runtime.submit_action(Action(3)) else {
        unreachable!()
    };
    assert_eq!(
        later.kind(),
        SubmitActionErrorKind::Terminal(RuntimeTerminalReason::TraceSequenceExhausted)
    );
    assert_eq!(later.into_action(), Action(3));
    assert_eq!(runtime.pump(PumpBudget::new(8)).processed_envelopes(), 0);
}

#[test]
fn reconciliation_generation_exhaustion_cancels_accepted_envelopes() {
    let calls = Rc::new(Cell::new(0));
    let mut runtime = AppRuntime::<App>::mount(state(&calls));
    let target = runtime.index().nodes()[0].id().clone();
    assert_eq!(
        runtime.set_focus(target.clone()),
        FocusTargetResult::Focused
    );
    let tokens = StyleTokens::new();
    let context = SurfaceBuildContext::new(&tokens, LayoutConstraints::unbounded());
    let publication_before = runtime.publish_surface(&context);
    let report_before = runtime.reconciliation_report().clone();

    runtime
        .submit_action(Action(1))
        .unwrap_or_else(|_| unreachable!());
    runtime
        .submit_action(Action(2))
        .unwrap_or_else(|_| unreachable!());
    assert!(runtime.state().updates.is_empty());
    assert_eq!(runtime.reconciliation_report(), &report_before);
    runtime.__seed_reconciliation_generation_for_test(u64::MAX);

    let report = runtime.pump(PumpBudget::new(1));
    assert_eq!(report.processed_envelopes(), 1);
    assert_eq!(report.remaining_queued_envelopes(), 0);
    assert_eq!(report.cancelled_by_terminal_transition(), 2);
    assert_eq!(
        report.outcome(),
        PumpOutcome::Terminal(RuntimeTerminalReason::ReconciliationGenerationExhausted)
    );
    assert!(runtime.state().updates.is_empty());
    assert_eq!(runtime.reconciliation_report(), &report_before);
    assert_eq!(runtime.focus().focused_node(), Some(&target));
    assert_eq!(runtime.index().nodes()[0].id(), &target);
    assert_eq!(runtime.publish_surface(&context), publication_before);
    assert_eq!(
        runtime.status(),
        RuntimeStatus::Terminal(RuntimeTerminalReason::ReconciliationGenerationExhausted)
    );
    assert!(
        runtime
            .trace()
            .kinds()
            .any(|kind| matches!(kind, TraceRecordKind::QueuedWorkCancelled { count: 2 }))
    );

    let Err(later) = runtime.submit_action(Action(3)) else {
        unreachable!()
    };
    assert_eq!(later.into_action(), Action(3));
    assert_eq!(runtime.pump(PumpBudget::new(8)).processed_envelopes(), 0);
}
