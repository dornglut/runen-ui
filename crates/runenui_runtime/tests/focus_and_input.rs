#![allow(refining_impl_trait)]

use runenui_core::{
    CommandOrigin, Element, NoHostProtocol, SemanticCommand, UiApp, View, button, children, row,
};
use runenui_runtime::{
    AppRuntime, InputModality, LogicalPoint, PumpBudget, RuntimeConfig, RuntimeLimits,
    RuntimeStatus, RuntimeTerminalReason, TraceConfig, TraceRecordKind,
};

#[derive(Debug)]
enum Action {
    A,
    B,
}
struct App;
impl UiApp for App {
    type State = usize;
    type Action = Action;
    type HostProtocol = NoHostProtocol;
    fn root(_: &usize) -> Element<Action> {
        row(children![
            button("A").id("a").key("a").on_activate(|| Action::A),
            button("B").id("b").key("b").on_activate(|| Action::B)
        ])
        .key("root")
        .into_element()
    }
    fn update(state: &mut usize, _: Action) {
        *state += 1;
    }
}

#[test]
fn normalized_keyboard_focus_commands_never_activate_or_emit_actions() {
    let mut runtime = AppRuntime::<App>::mount(0);
    runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
    let a = runtime.index().nodes()[1].id().clone();
    runtime
        .submit_command(
            a.clone(),
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("live focus target is accepted"));
    runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));
    runtime
        .submit_command(
            a.clone(),
            SemanticCommand::FocusNext,
            CommandOrigin::keyboard(),
        )
        .unwrap_or_else(|_| unreachable!("normalized keyboard command is accepted"));
    runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));
    assert_ne!(runtime.focus().focused_node(), Some(&a));
    assert_eq!(runtime.focus().modality(), Some(InputModality::Keyboard));
    assert_eq!(
        runtime.focus().reason(),
        Some(runenui_runtime::FocusReason::LinearNavigation)
    );
    assert_eq!(runtime.state(), &0);
}

#[test]
fn non_finite_pointer_positions_are_rejected() {
    assert!(LogicalPoint::new(f32::NAN, 0.0).is_err());
    assert!(LogicalPoint::new(0.0, f32::INFINITY).is_err());
}

#[test]
fn normalized_command_modalities_are_retained_only_after_accepted_processing() {
    let mut runtime = AppRuntime::<App>::mount(0);
    runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
    let target = runtime.index().nodes()[1].id().clone();
    let cases = [
        (CommandOrigin::programmatic(), InputModality::Programmatic),
        (CommandOrigin::keyboard(), InputModality::Keyboard),
        (CommandOrigin::controller(), InputModality::Controller),
        (CommandOrigin::accessibility(), InputModality::Accessibility),
        (CommandOrigin::automation(), InputModality::Automation),
    ];
    for (origin, expected) in cases {
        runtime
            .submit_command(target.clone(), SemanticCommand::RequestFocus, origin)
            .unwrap_or_else(|_| unreachable!("normalized command is accepted"));
        runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));
        assert_eq!(runtime.focus().modality(), Some(expected));
    }

    let mut foreign = AppRuntime::<App>::mount(0);
    let rejected = foreign.submit_command(
        target,
        SemanticCommand::RequestFocus,
        CommandOrigin::controller(),
    );
    assert!(rejected.is_err());
    assert_eq!(foreign.focus().modality(), None);
}

#[test]
fn shutdown_clears_focus_memory_with_shutdown_reason_and_retains_modality() {
    let mut runtime = AppRuntime::<App>::mount(0);
    runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
    let target = runtime.index().nodes()[1].id().clone();
    runtime
        .submit_command(
            target,
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("live focus target is accepted"));
    runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));
    runtime.shutdown();
    assert_eq!(runtime.focus().focused_node(), None);
    assert_eq!(
        runtime.focus().reason(),
        Some(runenui_runtime::FocusReason::Shutdown)
    );
    assert_eq!(
        runtime.focus().modality(),
        Some(InputModality::Programmatic)
    );
    let shutdown_kinds = runtime
        .trace()
        .records()
        .map(runenui_runtime::TraceRecord::kind)
        .collect::<Vec<_>>();
    let focus_shutdown = shutdown_kinds
        .iter()
        .position(|kind| {
            matches!(
                kind,
                TraceRecordKind::FocusTransitionCommitted {
                    reason: runenui_runtime::FocusReason::Shutdown,
                    ..
                }
            )
        })
        .unwrap_or_else(|| unreachable!("shutdown focus transition is traced"));
    let runtime_shutdown = shutdown_kinds
        .iter()
        .position(|kind| matches!(kind, TraceRecordKind::RuntimeShutdown { .. }))
        .unwrap_or_else(|| unreachable!("runtime shutdown is traced"));
    assert!(focus_shutdown < runtime_shutdown);
    assert!(
        shutdown_kinds[focus_shutdown..runtime_shutdown]
            .iter()
            .any(|kind| matches!(
                kind,
                TraceRecordKind::FocusNotificationSuppressed {
                    kind: runenui_runtime::FocusEventKind::Out,
                }
            ))
    );
}

#[test]
fn disabled_trace_preserves_focus_behavior() {
    let mut runtime = AppRuntime::<App>::mount_with_config(
        0,
        RuntimeConfig::default().with_trace_config(TraceConfig::new(0)),
    );
    let target = runtime.index().nodes()[1].id().clone();
    runtime
        .submit_command(
            target.clone(),
            SemanticCommand::RequestFocus,
            CommandOrigin::automation(),
        )
        .unwrap_or_else(|_| unreachable!("disabled tracing does not reject focus"));
    runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
    assert_eq!(runtime.focus().focused_node(), Some(&target));
    assert_eq!(runtime.focus().modality(), Some(InputModality::Automation));
    assert_eq!(runtime.trace().len(), 0);
}

#[test]
fn processing_admission_exhaustion_commits_no_partial_focus_or_modality() {
    let config =
        RuntimeConfig::default().with_limits(RuntimeLimits::default().with_transaction_outputs(0));
    let mut runtime = AppRuntime::<App>::mount_with_config(0, config);
    let target = runtime.index().nodes()[1].id().clone();
    runtime
        .submit_command(
            target,
            SemanticCommand::RequestFocus,
            CommandOrigin::controller(),
        )
        .unwrap_or_else(|_| unreachable!("ingress accepts before routed admission"));
    runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
    assert_eq!(runtime.focus().focused_node(), None);
    assert_eq!(runtime.focus().modality(), None);
}

#[test]
fn focus_trace_admission_exhaustion_commits_no_partial_focus_or_modality() {
    let mut runtime = AppRuntime::<App>::mount(0);
    runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
    let target = runtime.index().nodes()[1].id().clone();
    runtime.__seed_next_trace_sequence_for_test(u64::MAX - 1);
    runtime
        .submit_command(
            target,
            SemanticCommand::RequestFocus,
            CommandOrigin::controller(),
        )
        .unwrap_or_else(|_| unreachable!("submission owns the final trace reservation"));
    runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
    assert_eq!(runtime.focus().focused_node(), None);
    assert_eq!(runtime.focus().modality(), None);
    assert_eq!(
        runtime.status(),
        RuntimeStatus::Terminal(RuntimeTerminalReason::TraceSequenceExhausted)
    );
}
