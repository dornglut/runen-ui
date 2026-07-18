#![allow(refining_impl_trait)]

use std::{cell::Cell, rc::Rc};

use runenui_core::{Element, NoHostProtocol, StyleTokens, UiApp, View, button, children, column};
use runenui_runtime::{
    ActivationCapacity, ActivationResult, AppRuntime, FocusTargetResult, Key, KeyModifiers,
    KeyPhase, KeyboardEvent, LayoutConstraints, LogicalPoint, PointerActivationResult,
    PointerButton, PointerEvent, PointerPhase, PumpBudget, RuntimeConfig, RuntimeLimits,
    RuntimeStatus, RuntimeTerminalReason, SurfaceBuildContext, TraceRecordKind,
};

#[derive(Debug)]
struct Action;

#[derive(Debug)]
struct State {
    updates: usize,
    factory_calls: Rc<Cell<usize>>,
}

struct App;

impl UiApp for App {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> Element<Self::Action> {
        let calls = Rc::clone(&state.factory_calls);
        button("Activate")
            .id("activate")
            .key("activate")
            .on_activate(move || {
                calls.set(calls.get() + 1);
                Action
            })
            .into_element()
    }

    fn update(state: &mut Self::State, _: Self::Action) {
        state.updates += 1;
    }
}

fn state(calls: &Rc<Cell<usize>>) -> State {
    State {
        updates: 0,
        factory_calls: Rc::clone(calls),
    }
}

fn settle_initial_mounted_declarations<App: UiApp>(runtime: &mut AppRuntime<App>) {
    runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
}

#[test]
fn queue_full_rejects_before_factory_widget_state_and_invalidation_mutation() {
    let calls = Rc::new(Cell::new(0));
    let limits = RuntimeLimits::default()
        .with_waiting_envelopes(1)
        .with_transaction_outputs(1);
    let config = RuntimeConfig::default().with_limits(limits);
    let mut runtime = AppRuntime::<App>::mount_with_config(state(&calls), config);
    settle_initial_mounted_declarations(&mut runtime);
    let target = runtime.index().nodes()[0].id().clone();
    assert_eq!(runtime.set_focus(target), FocusTargetResult::Focused);

    let tokens = StyleTokens::new();
    let context = SurfaceBuildContext::new(&tokens, LayoutConstraints::unbounded());
    let before_rejection = runtime.publish_surface(&context);
    let focus_before = runtime.focus().focused_node().cloned();
    let report_before = runtime.reconciliation_report().clone();
    let phase_report_before = runtime.last_surface_phase_report().clone();
    let trace_len_before = runtime.trace().len();

    assert_eq!(
        runtime.activate("activate"),
        ActivationResult::Saturated(ActivationCapacity::WaitingEnvelopes)
    );
    assert_eq!(calls.get(), 0);
    assert_eq!(runtime.state().updates, 0);
    assert_eq!(runtime.focus().focused_node(), focus_before.as_ref());
    assert_eq!(runtime.reconciliation_report(), &report_before);
    assert_eq!(runtime.last_surface_phase_report(), &phase_report_before);
    assert_eq!(runtime.trace().len(), trace_len_before + 1);
    assert!(matches!(
        runtime
            .trace()
            .records()
            .last()
            .map(runenui_runtime::TraceRecord::kind),
        Some(TraceRecordKind::ActivationRejectedSaturated {
            capacity: ActivationCapacity::WaitingEnvelopes
        })
    ));
    let after_rejection = runtime.publish_surface(&context);
    assert_eq!(after_rejection, before_rejection);
}

#[test]
fn primary_activation_reports_the_complete_one_envelope_batch() {
    let calls = Rc::new(Cell::new(0));
    let limits = RuntimeLimits::default()
        .with_waiting_envelopes(3)
        .with_transaction_outputs(1);
    let mut runtime = AppRuntime::<App>::mount_with_config(
        state(&calls),
        RuntimeConfig::default().with_limits(limits),
    );
    settle_initial_mounted_declarations(&mut runtime);
    let ActivationResult::Queued(commit) = runtime.activate("activate") else {
        unreachable!("one primary action is accepted")
    };
    assert_eq!(commit.primary_action_sequence, Some(commit.first_sequence));
    assert_eq!(commit.queued_envelopes, 1);
    assert_eq!(calls.get(), 1);
}

#[cfg(feature = "internal-test-seams")]
#[test]
fn conservative_activation_admission_rejects_every_bounded_authority_before_callback() {
    let base = RuntimeLimits::default()
        .with_waiting_envelopes(3)
        .with_transaction_outputs(1);
    let saturated = [
        (base.with_local_tasks(0), ActivationCapacity::LocalTasks),
        (base.with_send_tasks(0), ActivationCapacity::SendTasks),
        (base.with_timers(0), ActivationCapacity::Timers),
    ];
    for (limits, capacity) in saturated {
        let calls = Rc::new(Cell::new(0));
        let mut runtime = AppRuntime::<App>::mount_with_config(
            state(&calls),
            RuntimeConfig::default().with_limits(limits),
        );
        settle_initial_mounted_declarations(&mut runtime);
        assert_eq!(
            runtime.activate("activate"),
            ActivationResult::Saturated(capacity)
        );
        assert_eq!(calls.get(), 0);
        assert_eq!(runtime.__live_work_record_count_for_test(), 0);
    }

    let queue_calls = Rc::new(Cell::new(0));
    let queue_limits = base.with_waiting_envelopes(2);
    let mut queue = AppRuntime::<App>::mount_with_config(
        state(&queue_calls),
        RuntimeConfig::default().with_limits(queue_limits),
    );
    settle_initial_mounted_declarations(&mut queue);
    assert_eq!(
        queue.activate("activate"),
        ActivationResult::Saturated(ActivationCapacity::WaitingEnvelopes)
    );
    assert_eq!(queue_calls.get(), 0);

    let sequence_calls = Rc::new(Cell::new(0));
    let mut sequence = AppRuntime::<App>::mount_with_config(
        state(&sequence_calls),
        RuntimeConfig::default().with_limits(base),
    );
    settle_initial_mounted_declarations(&mut sequence);
    sequence.__seed_next_work_sequence_for_test(u64::MAX);
    assert_eq!(
        sequence.activate("activate"),
        ActivationResult::Terminal(RuntimeTerminalReason::WorkSequenceExhausted)
    );
    assert_eq!(sequence_calls.get(), 0);

    let generation_calls = Rc::new(Cell::new(0));
    let mut generation = AppRuntime::<App>::mount_with_config(
        state(&generation_calls),
        RuntimeConfig::default().with_limits(base),
    );
    settle_initial_mounted_declarations(&mut generation);
    generation.__seed_next_work_generation_for_test(0);
    assert_eq!(
        generation.activate("activate"),
        ActivationResult::Terminal(RuntimeTerminalReason::WorkGenerationExhausted)
    );
    assert_eq!(generation_calls.get(), 0);

    let trace_calls = Rc::new(Cell::new(0));
    let mut trace = AppRuntime::<App>::mount_with_config(
        state(&trace_calls),
        RuntimeConfig::default().with_limits(base),
    );
    settle_initial_mounted_declarations(&mut trace);
    trace.__seed_next_trace_sequence_for_test(u64::MAX - 3);
    assert_eq!(
        trace.activate("activate"),
        ActivationResult::Terminal(RuntimeTerminalReason::TraceSequenceExhausted)
    );
    assert_eq!(trace_calls.get(), 0);
}

#[test]
fn zero_output_allowance_terminalizes_only_after_the_callback_exceeds_it() {
    let calls = Rc::new(Cell::new(0));
    let limits = RuntimeLimits::default()
        .with_waiting_envelopes(1)
        .with_transaction_outputs(0);
    let mut runtime = AppRuntime::<App>::mount_with_config(
        state(&calls),
        RuntimeConfig::default().with_limits(limits),
    );
    settle_initial_mounted_declarations(&mut runtime);
    assert_eq!(
        runtime.activate("activate"),
        ActivationResult::Terminal(RuntimeTerminalReason::Poisoned)
    );
    assert_eq!(calls.get(), 1);
}

#[test]
fn capacity_zero_closed_and_terminal_activation_invoke_no_factory() {
    let zero_calls = Rc::new(Cell::new(0));
    let zero = RuntimeConfig::default().with_queue_capacity(0);
    let mut zero_runtime = AppRuntime::<App>::mount_with_config(state(&zero_calls), zero);
    assert_eq!(
        zero_runtime.activate("activate"),
        ActivationResult::Terminal(RuntimeTerminalReason::Poisoned)
    );
    assert_eq!(
        zero_runtime.status(),
        RuntimeStatus::Terminal(RuntimeTerminalReason::Poisoned)
    );
    assert_eq!(zero_calls.get(), 0);

    let closed_calls = Rc::new(Cell::new(0));
    let mut closed = AppRuntime::<App>::mount(state(&closed_calls));
    closed.shutdown();
    assert_eq!(closed.activate("activate"), ActivationResult::Closed);
    assert_eq!(closed_calls.get(), 0);

    #[cfg(feature = "internal-test-seams")]
    {
        let terminal_calls = Rc::new(Cell::new(0));
        let mut terminal = AppRuntime::<App>::mount(state(&terminal_calls));
        settle_initial_mounted_declarations(&mut terminal);
        terminal.__seed_next_work_generation_for_test(0);
        assert_eq!(
            terminal.activate("activate"),
            ActivationResult::Terminal(RuntimeTerminalReason::WorkGenerationExhausted)
        );
        assert_eq!(terminal_calls.get(), 0);
        assert_eq!(terminal.state().updates, 0);

        let trace_calls = Rc::new(Cell::new(0));
        let mut trace_terminal = AppRuntime::<App>::mount(state(&trace_calls));
        settle_initial_mounted_declarations(&mut trace_terminal);
        let trace_count = trace_terminal.trace().records().count();
        trace_terminal.__seed_next_trace_sequence_for_test(0);
        assert_eq!(
            trace_terminal.activate("activate"),
            ActivationResult::Terminal(RuntimeTerminalReason::TraceSequenceExhausted)
        );
        assert_eq!(trace_calls.get(), 0);
        assert_eq!(trace_terminal.state().updates, 0);
        assert_eq!(trace_terminal.trace().records().count(), trace_count);
    }
}

struct NegativeApp;

impl UiApp for NegativeApp {
    type State = ();
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> Element<Self::Action> {
        column(children![
            button("Disabled")
                .id("disabled")
                .disabled()
                .on_activate(|| Action),
            button::<Action>("Plain").id("plain"),
            button("First").id("duplicate").on_activate(|| Action),
            button("Second").id("duplicate").on_activate(|| Action),
        ])
        .into_element()
    }

    fn update((): &mut Self::State, _: Self::Action) {}
}

#[test]
fn disabled_non_actionable_ambiguous_stale_and_foreign_targets_queue_nothing() {
    let mut runtime = AppRuntime::<NegativeApp>::mount(());
    assert_eq!(runtime.activate("disabled"), ActivationResult::Disabled);
    assert_eq!(runtime.activate("plain"), ActivationResult::NotActivatable);
    assert_eq!(runtime.activate("duplicate"), ActivationResult::AmbiguousId);
    let stale = runtime.index().nodes()[1].id().clone();
    runtime.shutdown();
    assert_eq!(runtime.activate_node(&stale), ActivationResult::Closed);

    let mut first = AppRuntime::<NegativeApp>::mount(());
    let foreign = first.index().nodes()[1].id().clone();
    let mut second = AppRuntime::<NegativeApp>::mount(());
    assert_eq!(
        second.activate_node(&foreign),
        ActivationResult::ForeignRuntime
    );
}

#[test]
fn keyboard_and_pointer_proof_helpers_share_queue_authority() {
    let calls = Rc::new(Cell::new(0));
    let mut runtime = AppRuntime::<App>::mount(state(&calls));
    settle_initial_mounted_declarations(&mut runtime);
    let id = runtime.index().nodes()[0].id().clone();
    assert_eq!(runtime.set_focus(id.clone()), FocusTargetResult::Focused);
    let keyboard = KeyboardEvent::new(KeyPhase::Pressed, Key::Enter, KeyModifiers::NONE, None);
    assert!(matches!(
        runtime.handle_keyboard_activation(&keyboard),
        runenui_runtime::KeyboardActivationResult::Handled(ActivationResult::Queued(_))
    ));
    let pointer = PointerEvent::new(
        PointerPhase::Pressed,
        LogicalPoint::new(1.0, 1.0).unwrap_or_else(|_| unreachable!()),
        Some(PointerButton::Primary),
        KeyModifiers::NONE,
        Some(id),
    );
    assert!(matches!(
        runtime.handle_pointer_activation(&pointer),
        PointerActivationResult::Handled(ActivationResult::Queued(_))
    ));
    assert_eq!(runtime.state().updates, 0);
    assert_eq!(calls.get(), 2);
    runtime.pump(PumpBudget::new(2, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(runtime.state().updates, 2);
}
