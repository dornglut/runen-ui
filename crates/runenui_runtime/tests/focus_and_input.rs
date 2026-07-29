#![allow(refining_impl_trait)]

use runenui_core::{
    CommandOrigin, Element, NoHostProtocol, SemanticCommand, UiApp, View, button, children, row,
};
use runenui_runtime::{
    AppRuntime, InputModality, LogicalPoint, PumpBudget, RuntimeConfig, RuntimeLimits,
    RuntimeStatus, RuntimeTerminalReason, TraceConfig, TraceRecordKind,
};

#[derive(Debug)]
struct CompositionProbe {
    name: &'static str,
    log: std::rc::Rc<std::cell::RefCell<Vec<String>>>,
}

impl runenui_core::Widget<CompositionAction> for CompositionProbe {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn event(
        &mut self,
        _: &mut Self::State,
        event: &runenui_core::UiEvent,
        context: &mut runenui_core::EventContext<'_, CompositionAction>,
    ) -> runenui_core::WidgetEventOutput {
        if let Some(runenui_core::CompositionEvent::Cancel(cancel)) = event.as_composition() {
            self.log.borrow_mut().push(format!(
                "{}:{:?}:{:?}",
                self.name,
                context.phase(),
                cancel.reason()
            ));
        }
        runenui_core::WidgetEventOutput::none()
    }

    fn activation(&self, _: &Self::State) -> runenui_core::WidgetActivation {
        runenui_core::WidgetActivation::actionable(true)
    }

    fn text_input(&self, _: &Self::State) -> runenui_core::WidgetTextInput {
        runenui_core::WidgetTextInput::new(true, true)
    }
}

#[derive(Debug)]
enum CompositionAction {}

struct CompositionApp;

impl UiApp for CompositionApp {
    type State = std::rc::Rc<std::cell::RefCell<Vec<String>>>;
    type Action = CompositionAction;
    type HostProtocol = NoHostProtocol;

    fn root(log: &Self::State) -> Element<Self::Action> {
        row(children![
            Element::new(CompositionProbe {
                name: "a",
                log: std::rc::Rc::clone(log),
            })
            .id("a")
            .key("a")
            .focusable(true),
            Element::new(CompositionProbe {
                name: "b",
                log: std::rc::Rc::clone(log),
            })
            .id("b")
            .key("b")
            .focusable(true),
        ])
        .key("root")
        .into_element()
    }

    fn update(_: &mut Self::State, _: Self::Action) {}
}

fn composition_target(
    runtime: &mut AppRuntime<CompositionApp>,
    authored: &str,
) -> runenui_runtime::MountedNodeId {
    let authored = runenui_core::ElementId::new(authored).unwrap_or_else(|_| unreachable!());
    runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored))
        .unwrap_or_else(|| unreachable!("mounted test node is present"))
        .id()
        .clone()
}

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
            CommandOrigin::__runtime_keyboard(),
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
        (CommandOrigin::__runtime_keyboard(), InputModality::Keyboard),
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

#[test]
fn composition_focus_transfer_routes_cancel_before_focus_out_and_retires_generation() {
    let log = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let mut runtime = AppRuntime::<CompositionApp>::mount(std::rc::Rc::clone(&log));
    runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
    let a = composition_target(&mut runtime, "a");
    runtime
        .submit_command(
            a.clone(),
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("focus request is accepted"));
    runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));
    let start = runtime
        .start_composition(None)
        .unwrap_or_else(|_| unreachable!("composition start is accepted"));
    runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));
    runtime
        .submit_command(a, SemanticCommand::FocusNext, CommandOrigin::programmatic())
        .unwrap_or_else(|_| unreachable!("focus navigation is accepted"));
    runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));

    assert_eq!(
        log.borrow().as_slice(),
        ["a:Target:FocusTransfer"],
        "the old owner observes cancellation while it is still routable"
    );
    assert!(matches!(
        runtime.submit_composition_end(start.generation().clone()),
        Err(error) if error.kind() == runenui_runtime::SubmitCompositionErrorKind::MissingGeneration
    ));
    let kinds: Vec<_> = runtime
        .trace()
        .records()
        .map(|record| record.kind())
        .collect();
    let cancellation = kinds
        .iter()
        .position(|kind| {
            matches!(
                kind,
                TraceRecordKind::CompositionCancelled {
                    reason: runenui_runtime::CompositionCancelReason::FocusTransfer
                }
            )
        })
        .unwrap_or_else(|| unreachable!("cancellation is traced"));
    let focus_out = kinds
        .iter()
        .position(|kind| {
            matches!(
                kind,
                TraceRecordKind::FocusNotificationQueued {
                    kind: runenui_runtime::FocusEventKind::Out
                }
            )
        })
        .unwrap_or_else(|| unreachable!("focus departure is traced"));
    assert!(cancellation < focus_out);
}
