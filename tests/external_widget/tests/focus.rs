#![allow(refining_impl_trait)]

use std::{cell::RefCell, rc::Rc};

use runenui_core::{
    CommandOrigin, Element, EventPhase, FocusBoundaryPolicy, FocusEventKind, FocusReason,
    FocusScope, FocusScopePolicy, NoHostProtocol, SemanticCommand, UiApp, View, column,
};
use runenui_external_widget_conformance::{
    ExternalFocusFact, ExternalFocusWidget, external_focus_panel,
};
use runenui_runtime::{
    AppRuntime, InputModality, MountedNodeId, PumpBudget, TraceDeliveryOutcome, TraceEventFamily,
    TraceFocusRecordRole, TraceRecordKind, TraceTarget,
};

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
    focus_root: &MountedNodeId,
    old_target: &MountedNodeId,
    new_target: &MountedNodeId,
) {
    let transition_trace = runtime
        .trace()
        .records()
        .skip(trace_start)
        .collect::<Vec<_>>();
    let committed = transition_trace
        .iter()
        .position(|record| {
            if !matches!(
                record.kind(),
                TraceRecordKind::FocusTransitionCommitted {
                    reason: FocusReason::ProgrammaticRequest,
                }
            ) {
                return false;
            }
            if record.context().focus_record_role() != Some(TraceFocusRecordRole::Transition) {
                return false;
            }
            let Some(transition) = record.context().target_transition() else {
                return false;
            };
            transition.previous().map(TraceTarget::mounted_node_id) == Some(old_target)
                && transition.current().map(TraceTarget::mounted_node_id) == Some(new_target)
        })
        .unwrap_or_else(|| unreachable!("the exact transition is traced"));
    let within = transition_trace
        .iter()
        .position(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::FocusWithinInvalidated {
                    left: 1,
                    entered: 1,
                }
            )
        })
        .unwrap_or_else(|| unreachable!("only the changed leaf routes invalidate"));
    let out = transition_trace
        .iter()
        .position(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::FocusNotificationResolved {
                    kind: FocusEventKind::Out,
                }
            ) && record.context().focus_record_role() == Some(TraceFocusRecordRole::Notification)
                && record.context().delivery() == Some(TraceDeliveryOutcome::Delivered)
                && record.context().event().is_some_and(|event| {
                    event.family() == TraceEventFamily::Focus && !event.is_cancelable()
                })
                && record.context().route().is_some_and(|route| {
                    route
                        .targets()
                        .iter()
                        .any(|target| target.mounted_node_id() == focus_root)
                        && route.targets().last().map(TraceTarget::mounted_node_id)
                            == Some(old_target)
                        && route.related_target().map(TraceTarget::mounted_node_id)
                            == Some(new_target)
                })
        })
        .unwrap_or_else(|| unreachable!("focus out is resolved after delivery"));
    let input = transition_trace
        .iter()
        .position(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::FocusNotificationResolved {
                    kind: FocusEventKind::In,
                }
            ) && record.context().focus_record_role() == Some(TraceFocusRecordRole::Notification)
                && record.context().delivery() == Some(TraceDeliveryOutcome::Delivered)
                && record.context().route().is_some_and(|route| {
                    route
                        .targets()
                        .iter()
                        .any(|target| target.mounted_node_id() == focus_root)
                        && route.targets().last().map(TraceTarget::mounted_node_id)
                            == Some(new_target)
                        && route.related_target().map(TraceTarget::mounted_node_id)
                            == Some(old_target)
                })
        })
        .unwrap_or_else(|| unreachable!("focus in is resolved after delivery"));
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

    assert_transition_trace(&runtime, trace_start, &root, &a, &b);
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
