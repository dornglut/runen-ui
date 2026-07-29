#![allow(refining_impl_trait)]

use std::{cell::RefCell, rc::Rc};

use runenui_core::{
    CommandOrigin, Element, EventPhase, FocusBoundaryPolicy, FocusEventKind, FocusReason,
    FocusScope, FocusScopePolicy, NoHostProtocol, SemanticCommand, UiApp, View, column,
};
use runenui_external_widget_conformance::{
    ExternalFocusFact, ExternalFocusWidget, external_focus_panel,
};
use runenui_runtime::{AppRuntime, InputModality, MountedNodeId, PumpBudget, TraceRecordKind};

struct State {
    log: Rc<RefCell<Vec<ExternalFocusFact>>>,
}

struct App;

impl UiApp for App {
    type State = State;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(state: &State) -> Element<()> {
        let scope =
            external_focus_panel(Rc::clone(&state.log)).focus_scope(FocusScope::new().with_policy(
                FocusScopePolicy::new(FocusBoundaryPolicy::Trap, FocusBoundaryPolicy::Delegate),
            ));
        column(vec![scope]).key("shell").into_element()
    }

    fn update(_: &mut State, (): ()) {}
}

fn id(runtime: &mut AppRuntime<App>, authored: &str) -> MountedNodeId {
    let authored = runenui_core::ElementId::new(authored).unwrap_or_else(|_| unreachable!());
    runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored))
        .unwrap_or_else(|| unreachable!("downstream-authored node is mounted"))
        .id()
        .clone()
}

fn focus(runtime: &mut AppRuntime<App>, target: MountedNodeId) {
    runtime
        .submit_command(
            target,
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("the public exact target is accepted"));
    assert_eq!(
        runtime
            .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
            .processed_envelopes(),
        1
    );
}

fn assert_transition_trace(
    runtime: &AppRuntime<App>,
    trace_start: usize,
    old_target: &MountedNodeId,
    new_target: &MountedNodeId,
) {
    let transition_trace = runtime
        .trace()
        .records()
        .skip(trace_start)
        .map(runenui_runtime::TraceRecord::kind)
        .collect::<Vec<_>>();
    let committed = transition_trace
        .iter()
        .position(|kind| {
            matches!(
                kind,
                TraceRecordKind::FocusTransitionCommitted {
                    reason: FocusReason::ProgrammaticRequest,
                    old_target: Some(old),
                    new_target: Some(new),
                } if old == old_target && new == new_target
            )
        })
        .unwrap_or_else(|| unreachable!("the exact transition is traced"));
    let within = transition_trace
        .iter()
        .position(|kind| {
            matches!(
                kind,
                TraceRecordKind::FocusWithinInvalidated {
                    left: 1,
                    entered: 1,
                }
            )
        })
        .unwrap_or_else(|| unreachable!("only the changed leaf routes invalidate"));
    let out = transition_trace
        .iter()
        .position(|kind| {
            matches!(
                kind,
                TraceRecordKind::FocusNotificationQueued {
                    kind: FocusEventKind::Out,
                }
            )
        })
        .unwrap_or_else(|| unreachable!("focus out is queued"));
    let input = transition_trace
        .iter()
        .position(|kind| {
            matches!(
                kind,
                TraceRecordKind::FocusNotificationQueued {
                    kind: FocusEventKind::In,
                }
            )
        })
        .unwrap_or_else(|| unreachable!("focus in is queued"));
    assert!(committed < within && within < out && out < input);
}

#[test]
fn downstream_focus_scope_events_reasons_and_focus_within_use_only_public_apis() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = AppRuntime::<App>::mount(State {
        log: Rc::clone(&log),
    });
    runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
    let root = id(&mut runtime, "focus.root");
    let a = id(&mut runtime, "focus.a");
    let b = id(&mut runtime, "focus.b");
    assert_eq!(
        runtime
            .index()
            .node(&root)
            .unwrap_or_else(|| unreachable!())
            .focus_scope(),
        Some(FocusScope::new().with_policy(FocusScopePolicy::new(
            FocusBoundaryPolicy::Trap,
            FocusBoundaryPolicy::Delegate,
        )))
    );
    for target in [&a, &b] {
        assert!(
            runtime
                .index()
                .node(target)
                .unwrap_or_else(|| unreachable!())
                .is_focusable()
        );
    }

    focus(&mut runtime, a.clone());
    log.borrow_mut().clear();
    let trace_start = runtime.trace().len();
    focus(&mut runtime, b.clone());

    assert_eq!(runtime.focus().focused_node(), Some(&b));
    assert_eq!(
        runtime.focus().reason(),
        Some(FocusReason::ProgrammaticRequest)
    );
    assert!(runtime.focus().is_focus_within(&root));
    assert!(!runtime.focus().is_focus_within(&a));
    assert!(runtime.focus().is_focus_within(&b));
    assert_eq!(
        log.borrow().as_slice(),
        &[
            ExternalFocusFact {
                widget: "root",
                phase: EventPhase::Capture,
                kind: FocusEventKind::Out,
                reason: FocusReason::ProgrammaticRequest,
                target_is_callback_target: false,
                has_related_target: true
            },
            ExternalFocusFact {
                widget: "a",
                phase: EventPhase::Target,
                kind: FocusEventKind::Out,
                reason: FocusReason::ProgrammaticRequest,
                target_is_callback_target: true,
                has_related_target: true
            },
            ExternalFocusFact {
                widget: "root",
                phase: EventPhase::Bubble,
                kind: FocusEventKind::Out,
                reason: FocusReason::ProgrammaticRequest,
                target_is_callback_target: false,
                has_related_target: true
            },
            ExternalFocusFact {
                widget: "root",
                phase: EventPhase::Capture,
                kind: FocusEventKind::In,
                reason: FocusReason::ProgrammaticRequest,
                target_is_callback_target: false,
                has_related_target: true
            },
            ExternalFocusFact {
                widget: "b",
                phase: EventPhase::Target,
                kind: FocusEventKind::In,
                reason: FocusReason::ProgrammaticRequest,
                target_is_callback_target: true,
                has_related_target: true
            },
            ExternalFocusFact {
                widget: "root",
                phase: EventPhase::Bubble,
                kind: FocusEventKind::In,
                reason: FocusReason::ProgrammaticRequest,
                target_is_callback_target: false,
                has_related_target: true
            },
        ]
    );

    assert_transition_trace(&runtime, trace_start, &a, &b);
}

struct PreventApp;

impl UiApp for PreventApp {
    type State = Rc<RefCell<Vec<ExternalFocusFact>>>;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> Element<()> {
        Element::new(
            ExternalFocusWidget::new("prevented", Rc::clone(state), true)
                .prevent_focus_request(true),
        )
        .id("prevented")
        .focusable(true)
    }

    fn update(_: &mut Self::State, (): ()) {}
}

#[test]
fn prevented_initiating_command_changes_modality_but_commits_no_focus_notification() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = AppRuntime::<PreventApp>::mount(Rc::clone(&log));
    let target = runtime.index().nodes()[0].id().clone();
    runtime
        .submit_command(
            target,
            SemanticCommand::RequestFocus,
            CommandOrigin::automation(),
        )
        .unwrap_or_else(|_| unreachable!("command ingress is accepted"));
    runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
    assert_eq!(runtime.focus().focused_node(), None);
    assert_eq!(runtime.focus().modality(), Some(InputModality::Automation));
    assert!(log.borrow().is_empty());
}
