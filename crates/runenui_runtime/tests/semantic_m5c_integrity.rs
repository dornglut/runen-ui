#![cfg(feature = "internal-test-seams")]
#![allow(refining_impl_trait)]

use std::{
    cell::Cell,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use runenui_core::{
    Element, NoHostProtocol, SemanticAction, SemanticActionRequest, StyleEnvironment, UiApp, View,
    button,
};
use runenui_runtime::{
    AppRuntime, LayoutConstraints, PumpBudget, RuntimeConfig, RuntimeStatus, RuntimeTerminalReason,
    SubmitSemanticActionError, SubmitSemanticActionErrorKind, SurfaceBuildContext, TraceRecordKind,
};

#[derive(Debug)]
struct Action;

#[derive(Debug)]
struct State {
    activation_calls: Rc<Cell<usize>>,
}

struct App;

impl UiApp for App {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> Element<Self::Action> {
        let calls = Rc::clone(&state.activation_calls);
        button("Activate")
            .id("activate")
            .key("activate")
            .on_activate(move || {
                calls.set(
                    calls
                        .get()
                        .checked_add(1)
                        .unwrap_or_else(|| unreachable!("test activation count does not overflow")),
                );
                Action
            })
            .into_element()
    }

    fn update(_: &mut Self::State, _: Self::Action) {}
}

fn runtime() -> AppRuntime<App> {
    AppRuntime::<App>::mount(state())
}

fn runtime_with_config(config: RuntimeConfig) -> AppRuntime<App> {
    AppRuntime::<App>::mount_with_config(state(), config)
}

fn state() -> State {
    State {
        activation_calls: Rc::new(Cell::new(0)),
    }
}

fn current_request(runtime: &mut AppRuntime<App>) -> SemanticActionRequest {
    runtime.pump(PumpBudget::new(usize::MAX, 0, 0, 0));
    let style_environment = StyleEnvironment::default();
    let publication = runtime
        .publish_surface(&SurfaceBuildContext::new(
            &style_environment,
            LayoutConstraints::unbounded(),
        ))
        .unwrap_or_else(|_| unreachable!("semantic publication is admitted"));
    let snapshot = publication.semantic_publication().snapshot();
    let node = snapshot
        .nodes()
        .iter()
        .find(|node| node.supported_actions().contains(&SemanticAction::Activate))
        .unwrap_or_else(|| unreachable!("actionable button publishes Activate"));
    SemanticActionRequest::new(
        snapshot.surface_id().clone(),
        node.id().clone(),
        SemanticAction::Activate,
    )
}

fn install_wake_counter(runtime: &AppRuntime<App>) -> Arc<AtomicUsize> {
    let calls = Arc::new(AtomicUsize::new(0));
    let transport_calls = Arc::clone(&calls);
    runtime.set_wake_transport(move || {
        transport_calls.fetch_add(1, Ordering::SeqCst);
    });
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    calls
}

fn expect_rejection(
    result: Result<runenui_runtime::CommandSubmission, SubmitSemanticActionError>,
) -> SubmitSemanticActionError {
    let Err(error) = result else {
        unreachable!("semantic request was expected to reject")
    };
    error
}

fn semantic_binding_count(runtime: &AppRuntime<App>) -> usize {
    runtime
        .trace()
        .records()
        .filter(|record| matches!(record.kind(), TraceRecordKind::SemanticActionBound { .. }))
        .count()
}

#[test]
fn same_runtime_wrong_surface_and_missing_target_reject_without_admission_side_effects() {
    let mut runtime = runtime();
    let request = current_request(&mut runtime);
    let wakes = install_wake_counter(&runtime);
    let sequence_state = runtime.__routed_sequence_state_for_test();
    let bindings = semantic_binding_count(&runtime);

    let wrong_surface = SemanticActionRequest::new(
        runtime.__surface_id_for_test(u32::MAX, 1),
        request.target().clone(),
        SemanticAction::Activate,
    );
    let expected = wrong_surface.clone();
    let error = expect_rejection(runtime.submit_semantic_action(wrong_surface));
    assert_eq!(error.kind(), SubmitSemanticActionErrorKind::WrongSurface);
    assert_eq!(error.into_request(), expected);
    assert_eq!(runtime.__routed_sequence_state_for_test(), sequence_state);
    assert_eq!(semantic_binding_count(&runtime), bindings);
    assert_eq!(runtime.state().activation_calls.get(), 0);
    assert_eq!(wakes.load(Ordering::SeqCst), 0);
    assert_eq!(runtime.status(), RuntimeStatus::Running);

    let missing_target = SemanticActionRequest::new(
        request.surface_id().clone(),
        runtime.__semantic_id_for_test(u32::MAX, 1),
        SemanticAction::Activate,
    );
    let expected = missing_target.clone();
    let error = expect_rejection(runtime.submit_semantic_action(missing_target));
    assert_eq!(error.kind(), SubmitSemanticActionErrorKind::MissingTarget);
    assert_eq!(error.into_request(), expected);
    assert_eq!(runtime.__routed_sequence_state_for_test(), sequence_state);
    assert_eq!(semantic_binding_count(&runtime), bindings);
    assert_eq!(runtime.state().activation_calls.get(), 0);
    assert_eq!(wakes.load(Ordering::SeqCst), 0);
    assert_eq!(runtime.status(), RuntimeStatus::Running);
}

#[test]
fn full_semantic_rejection_adds_no_wake_or_partial_admission() {
    let mut runtime = runtime_with_config(RuntimeConfig::default().with_queue_capacity(1));
    let request = current_request(&mut runtime);
    let wakes = install_wake_counter(&runtime);
    let bindings = semantic_binding_count(&runtime);
    runtime
        .submit_action(Action)
        .unwrap_or_else(|_| unreachable!("the single queue slot is available"));
    assert_eq!(wakes.load(Ordering::SeqCst), 1);
    let sequence_state = runtime.__routed_sequence_state_for_test();

    let expected = request.clone();
    let error = expect_rejection(runtime.submit_semantic_action(request));
    assert_eq!(error.kind(), SubmitSemanticActionErrorKind::Full);
    assert_eq!(error.into_request(), expected);
    assert_eq!(runtime.__routed_sequence_state_for_test(), sequence_state);
    assert_eq!(semantic_binding_count(&runtime), bindings);
    assert_eq!(runtime.state().activation_calls.get(), 0);
    assert_eq!(wakes.load(Ordering::SeqCst), 1);
    assert_eq!(runtime.status(), RuntimeStatus::Running);
}

#[test]
fn semantic_work_sequence_exhaustion_is_atomic_and_terminal() {
    let mut runtime = runtime();
    let request = current_request(&mut runtime);
    let wakes = install_wake_counter(&runtime);
    let trace_before = runtime
        .__routed_sequence_state_for_test()
        .1
        .unwrap_or_else(|| unreachable!("trace sequence authority is live before exhaustion"));
    let bindings = semantic_binding_count(&runtime);
    runtime.__seed_next_work_sequence_for_test(0);

    let expected = request.clone();
    let error = expect_rejection(runtime.submit_semantic_action(request));
    assert_eq!(
        error.kind(),
        SubmitSemanticActionErrorKind::WorkSequenceExhausted
    );
    assert_eq!(error.into_request(), expected);
    let sequence_state = runtime.__routed_sequence_state_for_test();
    assert_eq!(sequence_state.0, None);
    assert_eq!(
        sequence_state.1,
        Some(
            trace_before
                .checked_add(1)
                .unwrap_or_else(|| unreachable!("test trace sequence does not overflow"))
        )
    );
    assert_eq!(semantic_binding_count(&runtime), bindings);
    assert_eq!(runtime.state().activation_calls.get(), 0);
    assert_eq!(wakes.load(Ordering::SeqCst), 0);
    assert_eq!(
        runtime.status(),
        RuntimeStatus::Terminal(RuntimeTerminalReason::WorkSequenceExhausted)
    );
    assert!(runtime.trace().records().any(|record| {
        matches!(
            record.kind(),
            TraceRecordKind::RuntimeTerminal {
                reason: RuntimeTerminalReason::WorkSequenceExhausted
            }
        )
    }));

    let terminal_trace_state = runtime.__routed_sequence_state_for_test().1;
    let error = expect_rejection(runtime.submit_semantic_action(expected.clone()));
    assert_eq!(
        error.kind(),
        SubmitSemanticActionErrorKind::Terminal(RuntimeTerminalReason::WorkSequenceExhausted)
    );
    assert_eq!(error.into_request(), expected);
    assert_eq!(
        runtime.__routed_sequence_state_for_test().1,
        terminal_trace_state
    );
    assert_eq!(semantic_binding_count(&runtime), bindings);
    assert_eq!(wakes.load(Ordering::SeqCst), 0);
}

#[test]
fn semantic_trace_sequence_exhaustion_is_atomic_and_terminal() {
    let mut runtime = runtime();
    let request = current_request(&mut runtime);
    let wakes = install_wake_counter(&runtime);
    let work_before = runtime.__routed_sequence_state_for_test().0;
    let bindings = semantic_binding_count(&runtime);
    runtime.__seed_next_trace_sequence_for_test(0);

    let expected = request.clone();
    let error = expect_rejection(runtime.submit_semantic_action(request));
    assert_eq!(
        error.kind(),
        SubmitSemanticActionErrorKind::TraceSequenceExhausted
    );
    assert_eq!(error.into_request(), expected);
    let sequence_state = runtime.__routed_sequence_state_for_test();
    assert_eq!(sequence_state.0, work_before);
    assert_eq!(sequence_state.1, None);
    assert_eq!(runtime.__routed_trace_reservations_for_test(), 0);
    assert_eq!(semantic_binding_count(&runtime), bindings);
    assert_eq!(runtime.state().activation_calls.get(), 0);
    assert_eq!(wakes.load(Ordering::SeqCst), 0);
    assert_eq!(
        runtime.status(),
        RuntimeStatus::Terminal(RuntimeTerminalReason::TraceSequenceExhausted)
    );
}
