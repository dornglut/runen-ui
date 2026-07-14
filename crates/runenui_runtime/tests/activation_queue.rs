use std::{cell::Cell, rc::Rc};

use runenui_core::{Element, StyleTokens, View, button, children, column};
use runenui_runtime::{
    ActivationResult, AppRuntime, FocusTargetResult, Key, KeyModifiers, KeyPhase, KeyboardEvent,
    LayoutConstraints, LogicalPoint, PointerActivationResult, PointerButton, PointerEvent,
    PointerPhase, PumpBudget, RuntimeConfig, RuntimeTerminalReason, SurfaceBuildContext,
    TraceRecordKind, UiApp,
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

#[test]
fn queue_full_rejects_before_factory_widget_state_and_invalidation_mutation() {
    let calls = Rc::new(Cell::new(0));
    let config = RuntimeConfig::default().with_queue_capacity(1);
    let mut runtime = AppRuntime::<App>::mount_with_config(state(&calls), config);
    let target = runtime.index().nodes()[0].id().clone();
    assert_eq!(runtime.set_focus(target), FocusTargetResult::Focused);
    let ActivationResult::Queued { sequence: first } = runtime.activate("activate") else {
        unreachable!()
    };
    assert_eq!(first.get(), 1);
    assert_eq!(calls.get(), 1);
    assert_eq!(runtime.state().updates, 0);

    let tokens = StyleTokens::new();
    let context = SurfaceBuildContext::new(&tokens, LayoutConstraints::unbounded());
    let before_rejection = runtime.publish_surface(&context);
    assert!(
        before_rejection.frame().nodes()[0]
            .paint()
            .description()
            .contains("activations=1")
    );
    let focus_before = runtime.focus().focused_node().cloned();
    let report_before = runtime.reconciliation_report().clone();
    let phase_report_before = runtime.last_surface_phase_report().clone();
    let trace_len_before = runtime.trace().len();

    assert_eq!(runtime.activate("activate"), ActivationResult::QueueFull);
    assert_eq!(calls.get(), 1);
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
        Some(TraceRecordKind::ActivationRejectedFull)
    ));
    let after_rejection = runtime.publish_surface(&context);
    assert_eq!(after_rejection, before_rejection);

    assert_eq!(runtime.pump(PumpBudget::new(1)).processed_envelopes(), 1);
    assert_eq!(runtime.state().updates, 1);
    let ActivationResult::Queued { sequence: second } = runtime.activate("activate") else {
        unreachable!()
    };
    assert_eq!(second.get(), 2);
    assert_eq!(calls.get(), 2);
}

#[test]
fn capacity_zero_closed_and_terminal_activation_invoke_no_factory() {
    let zero_calls = Rc::new(Cell::new(0));
    let zero = RuntimeConfig::default().with_queue_capacity(0);
    let mut zero_runtime = AppRuntime::<App>::mount_with_config(state(&zero_calls), zero);
    assert_eq!(
        zero_runtime.activate("activate"),
        ActivationResult::QueueFull
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
        terminal.__seed_reconciliation_generation_for_test(u64::MAX);
        assert_eq!(
            terminal.activate("activate"),
            ActivationResult::Terminal(RuntimeTerminalReason::ReconciliationGenerationExhausted)
        );
        assert_eq!(terminal_calls.get(), 0);
        assert_eq!(terminal.state().updates, 0);

        let trace_calls = Rc::new(Cell::new(0));
        let mut trace_terminal = AppRuntime::<App>::mount(state(&trace_calls));
        trace_terminal.__seed_next_trace_sequence_for_test(0);
        assert_eq!(
            trace_terminal.activate("activate"),
            ActivationResult::Terminal(RuntimeTerminalReason::TraceSequenceExhausted)
        );
        assert_eq!(trace_calls.get(), 0);
        assert_eq!(trace_terminal.state().updates, 0);
        assert_eq!(trace_terminal.trace().records().count(), 1);
    }
}

struct NegativeApp;

impl UiApp for NegativeApp {
    type State = ();
    type Action = Action;

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
    let id = runtime.index().nodes()[0].id().clone();
    assert_eq!(runtime.set_focus(id.clone()), FocusTargetResult::Focused);
    let keyboard = KeyboardEvent::new(KeyPhase::Pressed, Key::Enter, KeyModifiers::NONE, None);
    assert!(matches!(
        runtime.handle_keyboard_activation(&keyboard),
        runenui_runtime::KeyboardActivationResult::Handled(ActivationResult::Queued { .. })
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
        PointerActivationResult::Handled(ActivationResult::Queued { .. })
    ));
    assert_eq!(runtime.state().updates, 0);
    assert_eq!(calls.get(), 2);
    runtime.pump(PumpBudget::new(2));
    assert_eq!(runtime.state().updates, 2);
}
