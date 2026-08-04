#![cfg(feature = "internal-test-seams")]
#![allow(refining_impl_trait)]

use std::{
    cell::Cell,
    future::Future,
    pin::Pin,
    rc::Rc,
    task::{Context, Poll},
};

use runenui_core::{
    ChildLayout, ChildLayoutWidget, CommandOrigin, Effects, Element, IntoEffects, NoHostProtocol,
    SemanticCommand, StyleTokens, UiApp, View, Widget, WidgetMountContext, button, text,
};
use runenui_runtime::{
    AppRuntime, LayoutConstraints, MountedNodeId, PumpBudget, PumpOutcome, RuntimeStatus,
    RuntimeTerminalReason, SubmitActionErrorKind, SubmitCommandErrorKind, SurfaceBuildContext,
    TraceRecordKind,
};

fn focus<App: UiApp>(runtime: &mut AppRuntime<App>, target: MountedNodeId) {
    runtime
        .submit_command(
            target,
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("the exact live focus target is accepted"));
    runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
}

#[derive(Debug, Eq, PartialEq)]
struct Action(u32);

#[derive(Debug)]
struct State {
    updates: Vec<u32>,
    factory_calls: Rc<Cell<usize>>,
}

#[derive(Debug)]
struct LifecycleWorkWidget;

impl Widget<bool> for LifecycleWorkWidget {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn mount(&self, (): &mut Self::State, context: &mut WidgetMountContext<bool>) {
        context.local_task(std::future::pending());
    }
}

struct DynamicTraceState {
    mounted: bool,
    updates: usize,
}

struct DynamicTraceApp;

impl UiApp for DynamicTraceApp {
    type State = DynamicTraceState;
    type Action = bool;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> Element<Self::Action> {
        if state.mounted {
            Element::new(LifecycleWorkWidget).key("lifecycle-work")
        } else {
            text("removed").into_element()
        }
    }

    fn update(state: &mut Self::State, mounted: Self::Action) {
        state.updates += 1;
        state.mounted = mounted;
    }
}

#[test]
fn post_update_dynamic_trace_admission_failure_is_poisoned() {
    let mut runtime = AppRuntime::<DynamicTraceApp>::mount(DynamicTraceState {
        mounted: true,
        updates: 0,
    });
    runtime.pump(PumpBudget::new(16, 0, 0, 0));
    assert_eq!(runtime.__live_work_record_count_for_test(), 1);
    runtime
        .submit_action(false)
        .unwrap_or_else(|_| unreachable!("removal action is accepted"));
    assert!(runtime.__surface_publication_trace_reserved_for_test());
    runtime.__seed_next_trace_sequence_for_test(u64::MAX - 4);
    runtime.pump(PumpBudget::new(1, 0, 0, 0));

    assert_eq!(runtime.state().updates, 1);
    assert!(!runtime.state().mounted);
    assert_eq!(
        runtime.status(),
        RuntimeStatus::Terminal(RuntimeTerminalReason::Poisoned)
    );
    assert_eq!(runtime.__live_work_record_count_for_test(), 0);
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

struct CountedPendingFuture(Rc<Cell<usize>>);

impl Future for CountedPendingFuture {
    type Output = Option<()>;

    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        self.0.set(self.0.get() + 1);
        Poll::Pending
    }
}

struct PollApp;

impl UiApp for PollApp {
    type State = Rc<Cell<usize>>;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(_: &Self::State) -> impl View<Self::Action> {
        text("poll integrity")
    }

    fn initial_effects(state: &Self::State) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        Effects::local_task(CountedPendingFuture(Rc::clone(state)))
    }

    fn update(_: &mut Self::State, (): Self::Action) {}
}

#[test]
fn work_sequence_exhaustion_prevents_local_future_poll() {
    let calls = Rc::new(Cell::new(0));
    let mut runtime = AppRuntime::<PollApp>::mount(Rc::clone(&calls));
    runtime.pump(PumpBudget::new(2, 0, 0, 0));
    runtime.__seed_next_work_sequence_for_test(0);

    runtime.pump(PumpBudget::new(0, 0, 1, 0));

    assert_eq!(calls.get(), 0);
    assert_eq!(
        runtime.status(),
        RuntimeStatus::Terminal(RuntimeTerminalReason::WorkSequenceExhausted)
    );
}

#[test]
fn trace_sequence_exhaustion_prevents_local_future_poll() {
    let calls = Rc::new(Cell::new(0));
    let mut runtime = AppRuntime::<PollApp>::mount(Rc::clone(&calls));
    runtime.pump(PumpBudget::new(2, 0, 0, 0));
    runtime.__seed_next_trace_sequence_for_test(0);

    runtime.pump(PumpBudget::new(0, 0, 1, 0));

    assert_eq!(calls.get(), 0);
    assert_eq!(
        runtime.status(),
        RuntimeStatus::Terminal(RuntimeTerminalReason::TraceSequenceExhausted)
    );
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

    let report = runtime.pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX));
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
fn command_submission_sequence_exhaustion_recovers_inputs_and_closes_mutation() {
    let calls = Rc::new(Cell::new(0));
    let mut runtime = AppRuntime::<App>::mount(state(&calls));
    let target = runtime.index().nodes()[0].id().clone();
    focus(&mut runtime, target.clone());

    let tokens = StyleTokens::new();
    let context = SurfaceBuildContext::new(&tokens, LayoutConstraints::unbounded());
    let publication_before = runtime.publish_surface(&context);
    let report_before = runtime.reconciliation_report().clone();

    runtime.__seed_next_work_sequence_for_test(0);
    let Err(error) = runtime.submit_command(
        target.clone(),
        SemanticCommand::Activate,
        CommandOrigin::programmatic(),
    ) else {
        unreachable!("an exhausted sequence authority rejects the command")
    };
    assert_eq!(error.kind(), SubmitCommandErrorKind::WorkSequenceExhausted);
    let (recovered_target, recovered_command, recovered_origin) =
        error.into_unaccepted().into_parts();
    assert_eq!(recovered_target, target);
    assert_eq!(recovered_command, SemanticCommand::Activate);
    assert_eq!(recovered_origin, CommandOrigin::programmatic());
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
    assert_eq!(
        runtime
            .pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX))
            .processed_envelopes(),
        0
    );

    let Err(later) = runtime.submit_action(Action(2)) else {
        unreachable!()
    };
    assert_eq!(later.into_action(), Action(2));
}

#[test]
fn trace_exhaustion_during_pump_cancels_failed_and_waiting_envelopes() {
    let calls = Rc::new(Cell::new(0));
    let mut runtime = AppRuntime::<App>::mount(state(&calls));
    runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
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

    let report = runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));
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
    assert_eq!(
        runtime
            .pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX))
            .processed_envelopes(),
        0
    );
}

#[test]
fn reconciliation_generation_exhaustion_cancels_accepted_envelopes() {
    let calls = Rc::new(Cell::new(0));
    let mut runtime = AppRuntime::<App>::mount(state(&calls));
    runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
    let target = runtime.index().nodes()[0].id().clone();
    focus(&mut runtime, target.clone());
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

    let report = runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));
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
    assert_eq!(
        runtime
            .pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX))
            .processed_envelopes(),
        0
    );
}

#[derive(Debug)]
struct MountCountProbe(Rc<Cell<usize>>);

impl Widget<()> for MountCountProbe {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn mount(&self, (): &mut Self::State, _: &mut WidgetMountContext<()>) {
        self.0.set(self.0.get() + 1);
    }
}

struct InitialMountedCapacityApp;

impl UiApp for InitialMountedCapacityApp {
    type State = Rc<Cell<usize>>;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> Element<Self::Action> {
        Element::new(MountCountProbe(Rc::clone(state))).key("capacity-root")
    }

    fn update(_: &mut Self::State, (): Self::Action) {}
}

#[test]
fn initial_mounted_identity_exhaustion_runs_no_mount_callback_and_is_terminal() {
    let mount_calls = Rc::new(Cell::new(0));
    let config =
        runenui_runtime::RuntimeConfig::default().__with_mounted_public_slot_limit_for_test(0);
    let mut runtime =
        AppRuntime::<InitialMountedCapacityApp>::mount_with_config(Rc::clone(&mount_calls), config);

    assert_eq!(mount_calls.get(), 0);
    assert!(runtime.index().nodes().is_empty());
    assert_eq!(
        runtime.status(),
        RuntimeStatus::Terminal(RuntimeTerminalReason::MountedIdentityExhausted)
    );
    assert!(runtime.trace().kinds().any(|kind| matches!(
        kind,
        TraceRecordKind::RuntimeTerminal {
            reason: RuntimeTerminalReason::MountedIdentityExhausted
        }
    )));
}

#[derive(Debug)]
struct CapacityRoot;

impl Widget<()> for CapacityRoot {
    type State = ();

    fn create_state(&self) -> Self::State {}
}

impl ChildLayoutWidget<()> for CapacityRoot {
    fn child_layout(&self, (): &Self::State) -> ChildLayout {
        ChildLayout::Linear {
            axis: runenui_core::Axis::Vertical,
        }
    }
}

#[derive(Debug)]
struct MountedCapacityState {
    expanded: bool,
    child_mount_calls: Rc<Cell<usize>>,
}

struct ReconcileMountedCapacityApp;

impl UiApp for ReconcileMountedCapacityApp {
    type State = MountedCapacityState;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> Element<Self::Action> {
        let children = if state.expanded {
            vec![
                Element::new(MountCountProbe(Rc::clone(&state.child_mount_calls)))
                    .key("capacity-child"),
            ]
        } else {
            Vec::new()
        };
        runenui_core::container(CapacityRoot, children)
            .key("capacity-root")
            .into_element()
    }

    fn update(state: &mut Self::State, (): Self::Action) {
        state.expanded = true;
    }
}

#[test]
fn post_update_mounted_identity_exhaustion_poisoned_without_partial_tree_mutation() {
    let child_mount_calls = Rc::new(Cell::new(0));
    let config =
        runenui_runtime::RuntimeConfig::default().__with_mounted_public_slot_limit_for_test(1);
    let mut runtime = AppRuntime::<ReconcileMountedCapacityApp>::mount_with_config(
        MountedCapacityState {
            expanded: false,
            child_mount_calls: Rc::clone(&child_mount_calls),
        },
        config,
    );
    assert_eq!(runtime.index().nodes().len(), 1);
    runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));

    runtime
        .submit_action(())
        .unwrap_or_else(|_| unreachable!("capacity transition action is accepted"));
    runtime.pump(PumpBudget::new(1, 0, 0, 0));

    assert!(runtime.state().expanded);
    assert_eq!(child_mount_calls.get(), 0);
    assert_eq!(runtime.index().nodes().len(), 1);
    assert_eq!(
        runtime.status(),
        RuntimeStatus::Terminal(RuntimeTerminalReason::Poisoned)
    );
}
