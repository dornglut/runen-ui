#![allow(refining_impl_trait)]

use std::{cell::RefCell, rc::Rc, time::Duration};

use runenui_core::{
    CommandOrigin, Effects, Element, EventContext, EventPhase, FocusBoundaryPolicy, FocusEventKind,
    FocusGroup, FocusGroupBoundaryPolicy, FocusGroupEntry, FocusGroupTypeAhead, FocusReason,
    FocusScope, FocusScopePolicy, Focusability, KeyLocation, KeyModifiers,
    KeyboardCompositionState, KeyboardEvent, KeyboardPhase, LogicalKey, NoHostProtocol,
    PhysicalKey, SemanticCommand, UiApp, UiEvent, View, Widget, WidgetActivation,
    WidgetEventOutput, column, container,
};
use runenui_external_widget_conformance::{
    ExternalFocusFact, ExternalFocusWidget, external_focus_panel,
};
use runenui_runtime::{
    AppRuntime, InputModality, MountedNodeId, PumpBudget, TraceDeliveryOutcome, TraceEventFamily,
    TraceFocusRecordRole, TraceRecordKind, TraceTarget,
};

const FULL_PUMP: PumpBudget = PumpBudget::new(usize::MAX, usize::MAX, usize::MAX, usize::MAX);

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
            .unwrap_or_else(|_| unreachable!("pump observation"))
            .report()
            .to_owned()
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
    let _ = runtime
        .pump(FULL_PUMP)
        .unwrap_or_else(|_| unreachable!("pump observation"));
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
#[allow(clippy::assert_is_empty)]
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
    let _ = runtime
        .pump(PumpBudget::new(
            usize::MAX,
            usize::MAX,
            usize::MAX,
            usize::MAX,
        ))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(runtime.focus().focused_node(), None);
    assert_eq!(runtime.focus().modality(), Some(InputModality::Automation));
    assert!(log.borrow().is_empty());
}

struct ExternalGroupApp;

impl UiApp for ExternalGroupApp {
    type State = Rc<RefCell<Vec<ExternalFocusFact>>>;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> Element<()> {
        let before = Element::new(ExternalFocusWidget::new("before", Rc::clone(state), true))
            .id("group.before")
            .focusable(true);
        let group = container(
            ExternalFocusWidget::new("group", Rc::clone(state), false),
            vec![
                Element::new(ExternalFocusWidget::new("a", Rc::clone(state), true))
                    .id("group.a")
                    .focusable(true)
                    .focus_group_search_text("amber"),
                Element::new(ExternalFocusWidget::new("b", Rc::clone(state), true))
                    .id("group.b")
                    .focusable(true)
                    .focus_group_preferred(true)
                    .focus_group_search_text("blue"),
                Element::new(ExternalFocusWidget::new("c", Rc::clone(state), false))
                    .id("group.c")
                    .with_focusability(Focusability::FocusableWhenDisabled)
                    .focus_group_search_text("cyan"),
            ],
        )
        .id("group.root")
        .into_element()
        .focus_group(
            FocusGroup::new()
                .with_boundary(FocusGroupBoundaryPolicy::Wrap)
                .with_type_ahead(
                    FocusGroupTypeAhead::new(Duration::from_millis(500))
                        .unwrap_or_else(|_| unreachable!("downstream timeout is bounded")),
                ),
        );
        let after = Element::new(ExternalFocusWidget::new("after", Rc::clone(state), true))
            .id("group.after")
            .focusable(true);
        column(vec![before, group, after]).into_element()
    }

    fn update(_: &mut Self::State, (): ()) {}
}

fn group_id(runtime: &mut AppRuntime<ExternalGroupApp>, authored: &str) -> MountedNodeId {
    let authored = runenui_core::ElementId::new(authored).unwrap_or_else(|_| unreachable!());
    runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored))
        .unwrap_or_else(|| unreachable!("external group node is mounted"))
        .id()
        .clone()
}

fn group_character(character: &str) -> KeyboardEvent {
    KeyboardEvent::new(
        KeyboardPhase::Down,
        PhysicalKey::Code(String::from("KeyX")),
        LogicalKey::Character(character.to_owned()),
        KeyModifiers::NONE,
        false,
        KeyLocation::Standard,
        KeyboardCompositionState::Inactive,
        None,
    )
}

#[test]
fn downstream_widgets_author_and_use_focus_groups_through_public_contracts() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = AppRuntime::<ExternalGroupApp>::mount(Rc::clone(&log));
    let _ = runtime
        .pump(FULL_PUMP)
        .unwrap_or_else(|_| unreachable!("pump observation"));

    let before = group_id(&mut runtime, "group.before");
    let group = group_id(&mut runtime, "group.root");
    let a = group_id(&mut runtime, "group.a");
    let b = group_id(&mut runtime, "group.b");
    let c = group_id(&mut runtime, "group.c");

    assert_eq!(
        runtime
            .index()
            .node(&group)
            .unwrap_or_else(|| unreachable!("group root is public"))
            .focus_group(),
        Some(
            FocusGroup::new()
                .with_boundary(FocusGroupBoundaryPolicy::Wrap)
                .with_type_ahead(
                    FocusGroupTypeAhead::new(Duration::from_millis(500))
                        .unwrap_or_else(|_| unreachable!("downstream timeout is bounded")),
                ),
        )
    );
    assert_eq!(
        runtime
            .index()
            .node(&b)
            .unwrap_or_else(|| unreachable!("preferred member is public"))
            .focus_group_entry(),
        FocusGroupEntry::Preferred
    );

    runtime
        .submit_command(
            before.clone(),
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("external before target is accepted"));
    let _ = runtime
        .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"));
    runtime
        .submit_command(
            before,
            SemanticCommand::FocusNext,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("external traversal command is accepted"));
    let _ = runtime
        .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"));
    assert_eq!(runtime.focus().focused_node(), Some(&b));

    assert!(
        runtime
            .index()
            .node(&c)
            .unwrap_or_else(|| unreachable!("disabled discoverable member is public"))
            .is_focusable()
    );
    assert!(
        !runtime
            .index()
            .node(&c)
            .unwrap_or_else(|| unreachable!("disabled discoverable member is public"))
            .activation()
            .enabled()
    );

    runtime
        .submit_command(
            b.clone(),
            SemanticCommand::FocusGroupLast,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("external absolute group navigation is accepted"));
    let _ = runtime
        .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"));
    assert_eq!(runtime.focus().focused_node(), Some(&c));

    runtime
        .submit_command(
            c,
            SemanticCommand::FocusGroupFirst,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("external first-member navigation is accepted"));
    let _ = runtime
        .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"));
    assert_eq!(runtime.focus().focused_node(), Some(&a));

    runtime
        .submit_keyboard(group_character("b"))
        .unwrap_or_else(|_| unreachable!("downstream type-ahead keyboard input is accepted"));
    let _ = runtime
        .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"));
    assert_eq!(runtime.focus().focused_node(), Some(&b));
}

struct CompatibleDisableState {
    log: Rc<RefCell<Vec<ExternalFocusFact>>>,
    enabled: bool,
}

struct CompatibleDisableApp;

impl UiApp for CompatibleDisableApp {
    type State = CompatibleDisableState;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> Element<()> {
        container(
            ExternalFocusWidget::new("root", Rc::clone(&state.log), false),
            vec![
                Element::new(ExternalFocusWidget::new(
                    "a",
                    Rc::clone(&state.log),
                    state.enabled,
                ))
                .id("compatible.focus.a")
                .key("a")
                .focusable(true),
            ],
        )
        .id("compatible.focus.root")
        .key("root")
        .into_element()
    }

    fn update(state: &mut Self::State, (): Self::Action) {
        state.enabled = false;
    }
}

/// A still-mounted old focus route must observe the real C/T/B notification,
/// including a disabled target whose private state cannot rely on trace alone.
#[test]
fn compatible_disablement_delivers_real_focus_out_to_live_downstream_route() {
    let log = Rc::new(RefCell::new(Vec::<ExternalFocusFact>::new()));
    let mut runtime = AppRuntime::<CompatibleDisableApp>::mount(CompatibleDisableState {
        log: Rc::clone(&log),
        enabled: true,
    });
    assert!(
        runtime
            .pump(FULL_PUMP)
            .unwrap_or_else(|_| unreachable!())
            .report()
            .is_quiescent()
    );
    let target = runtime
        .index()
        .nodes()
        .iter()
        .find(|node| {
            node.authored_id()
                .is_some_and(|id| id.as_str() == "compatible.focus.a")
        })
        .unwrap_or_else(|| unreachable!("A is mounted"))
        .id()
        .clone();
    runtime
        .submit_command(
            target.clone(),
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("A accepts focus before disablement"));
    assert!(
        runtime
            .pump(FULL_PUMP)
            .unwrap_or_else(|_| unreachable!())
            .report()
            .is_quiescent()
    );
    log.borrow_mut().clear();

    runtime
        .submit_action(())
        .unwrap_or_else(|_| unreachable!("compatible update must be admitted"));
    assert!(
        runtime
            .pump(FULL_PUMP)
            .unwrap_or_else(|_| unreachable!())
            .report()
            .is_quiescent()
    );

    assert_eq!(runtime.focus().focused_node(), None);
    assert!(
        runtime.index().node(&target).is_some(),
        "the exact old widget stays mounted"
    );
    let events = log.borrow();
    assert_eq!(
        events
            .iter()
            .map(|event| (event.widget, event.phase, event.kind, event.reason))
            .collect::<Vec<_>>(),
        [
            (
                "root",
                EventPhase::Capture,
                FocusEventKind::Out,
                FocusReason::Disablement
            ),
            (
                "a",
                EventPhase::Target,
                FocusEventKind::Out,
                FocusReason::Disablement
            ),
            (
                "root",
                EventPhase::Bubble,
                FocusEventKind::Out,
                FocusReason::Disablement
            ),
        ],
        "one noncancelable routed FocusOut precedes any later app effects"
    );
    assert!(events.iter().all(|event| !event.has_related_target));
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FocusOutOrderAction {
    Disable,
    NotificationOutput,
    UpdateOutput,
}

struct FocusOutOrderState {
    enabled: bool,
    processed: Vec<FocusOutOrderAction>,
}

#[derive(Debug)]
struct FocusOutOrderWidget {
    enabled: bool,
}

impl Widget<FocusOutOrderAction> for FocusOutOrderWidget {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn activation(&self, (): &Self::State) -> WidgetActivation {
        WidgetActivation::actionable(self.enabled)
    }

    fn event(
        &mut self,
        (): &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, FocusOutOrderAction>,
    ) -> WidgetEventOutput {
        if context.phase() == EventPhase::Target
            && event.as_focus().is_some_and(|focus| {
                focus.kind() == FocusEventKind::Out && focus.reason() == FocusReason::Disablement
            })
        {
            context.emit(FocusOutOrderAction::NotificationOutput);
        }
        WidgetEventOutput::none()
    }
}

struct FocusOutOrderApp;

impl UiApp for FocusOutOrderApp {
    type State = FocusOutOrderState;
    type Action = FocusOutOrderAction;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> Element<Self::Action> {
        column(vec![
            Element::new(FocusOutOrderWidget {
                enabled: state.enabled,
            })
            .id("focus.output.a")
            .key("a")
            .focusable(true),
        ])
        .id("focus.output.root")
        .key("root")
        .into_element()
    }

    fn update(
        state: &mut Self::State,
        action: Self::Action,
    ) -> Effects<Self::Action, Self::HostProtocol> {
        state.processed.push(action);
        match action {
            FocusOutOrderAction::Disable => {
                state.enabled = false;
                Effects::action(FocusOutOrderAction::UpdateOutput)
            }
            FocusOutOrderAction::NotificationOutput | FocusOutOrderAction::UpdateOutput => {
                Effects::none()
            }
        }
    }
}

/// Reconciled focus notifications emit ordinary work before the initiating
/// application update's own effects. Partial pumps cannot reverse that FIFO.
#[test]
fn compatible_disablement_notification_outputs_precede_application_effects() {
    let mut runtime = AppRuntime::<FocusOutOrderApp>::mount(FocusOutOrderState {
        enabled: true,
        processed: Vec::new(),
    });
    runtime
        .pump(FULL_PUMP)
        .unwrap_or_else(|_| unreachable!("initial pump"));
    let owner = runtime
        .index()
        .nodes()
        .iter()
        .find(|node| {
            node.authored_id()
                .is_some_and(|id| id.as_str() == "focus.output.a")
        })
        .unwrap_or_else(|| unreachable!("downstream target exists"))
        .id()
        .clone();
    runtime
        .submit_command(
            owner.clone(),
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("focus command admitted"));
    runtime
        .pump(FULL_PUMP)
        .unwrap_or_else(|_| unreachable!("focus command settles"));
    assert_eq!(runtime.focus().focused_node(), Some(&owner));

    runtime
        .submit_action(FocusOutOrderAction::Disable)
        .unwrap_or_else(|_| unreachable!("disable action admitted"));
    let first = runtime
        .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("first bounded pump"));
    assert_eq!(first.report().processed_envelopes(), 1);
    assert_eq!(runtime.state().processed, [FocusOutOrderAction::Disable]);
    assert_eq!(runtime.focus().focused_node(), None);

    let second = runtime
        .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("notification output pump"));
    assert_eq!(second.report().processed_envelopes(), 1);
    assert_eq!(
        runtime.state().processed,
        [
            FocusOutOrderAction::Disable,
            FocusOutOrderAction::NotificationOutput
        ],
        "routed FocusOut output is ahead of the application update effect"
    );

    let third = runtime
        .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("application effect pump"));
    assert_eq!(third.report().processed_envelopes(), 1);
    assert_eq!(
        runtime.state().processed,
        [
            FocusOutOrderAction::Disable,
            FocusOutOrderAction::NotificationOutput,
            FocusOutOrderAction::UpdateOutput,
        ]
    );
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ReconciliationFocusAction {
    Noop,
    Retain,
    Reorder,
    Disable,
    ReorderAndDisable,
    Remove,
    Replace,
}

struct ReconciliationFocusState {
    log: Rc<RefCell<Vec<ExternalFocusFact>>>,
    change: ReconciliationFocusAction,
}

#[derive(Debug)]
struct ReplacementFocusWidget;

impl Widget<()> for ReplacementFocusWidget {
    type State = ();

    fn create_state(&self) -> Self::State {}
}

struct ReconciliationFocusApp;

impl UiApp for ReconciliationFocusApp {
    type State = ReconciliationFocusState;
    type Action = ReconciliationFocusAction;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> Element<Self::Action> {
        let enabled = !matches!(
            state.change,
            ReconciliationFocusAction::Disable | ReconciliationFocusAction::ReorderAndDisable
        );
        let target = if state.change == ReconciliationFocusAction::Replace {
            Element::new(ReplacementFocusWidget)
                .key("a")
                .id("reconciliation.a")
                .focusable(true)
        } else {
            Element::new(ExternalFocusWidget::new(
                "a",
                Rc::clone(&state.log),
                enabled,
            ))
            .key("a")
            .id("reconciliation.a")
            .focusable(true)
        };
        let sibling = Element::new(ExternalFocusWidget::new(
            "sibling",
            Rc::clone(&state.log),
            false,
        ))
        .key("sibling")
        .id("reconciliation.sibling");
        let children = match state.change {
            ReconciliationFocusAction::Remove => vec![sibling],
            ReconciliationFocusAction::Reorder | ReconciliationFocusAction::ReorderAndDisable => {
                vec![sibling, target]
            }
            _ => vec![target, sibling],
        };
        container(
            ExternalFocusWidget::new("root", Rc::clone(&state.log), false),
            children,
        )
        .id("reconciliation.root")
        .key("root")
        .into_element()
        .map_action(|()| ReconciliationFocusAction::Noop)
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        if action != ReconciliationFocusAction::Noop {
            state.change = action;
        }
    }
}

fn focused_reconciliation_fixture(
    trace_capacity: usize,
) -> (
    AppRuntime<ReconciliationFocusApp>,
    Rc<RefCell<Vec<ExternalFocusFact>>>,
    MountedNodeId,
) {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = AppRuntime::<ReconciliationFocusApp>::mount_with_config(
        ReconciliationFocusState {
            log: Rc::clone(&log),
            change: ReconciliationFocusAction::Retain,
        },
        runenui_runtime::RuntimeConfig::default()
            .with_trace_config(runenui_runtime::TraceConfig::new(trace_capacity)),
    );
    assert!(
        runtime
            .pump(FULL_PUMP)
            .unwrap_or_else(|_| unreachable!("initial reconciliation"))
            .report()
            .is_quiescent()
    );
    let target = runtime
        .index()
        .nodes()
        .iter()
        .find(|node| {
            node.authored_id()
                .is_some_and(|id| id.as_str() == "reconciliation.a")
        })
        .unwrap_or_else(|| unreachable!("A is mounted"))
        .id()
        .clone();
    runtime
        .submit_command(
            target.clone(),
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("focus request is admitted"));
    assert!(
        runtime
            .pump(FULL_PUMP)
            .unwrap_or_else(|_| unreachable!("initial focus"))
            .report()
            .is_quiescent()
    );
    assert_eq!(runtime.focus().focused_node(), Some(&target));
    log.borrow_mut().clear();
    (runtime, log, target)
}

#[test]
fn reconciled_retention_never_synthesizes_focus_out_even_after_sibling_reorder() {
    for change in [
        ReconciliationFocusAction::Retain,
        ReconciliationFocusAction::Reorder,
    ] {
        let (mut runtime, log, target) = focused_reconciliation_fixture(256);
        let trace_start = runtime.trace().len();
        runtime
            .submit_action(change)
            .unwrap_or_else(|_| unreachable!("retained action is admitted"));
        let zero = runtime
            .pump(PumpBudget::new(0, 0, 0, 0))
            .unwrap_or_else(|_| unreachable!("zero-budget pump"));
        assert_eq!(zero.report().processed_envelopes(), 0);
        assert_eq!(runtime.focus().focused_node(), Some(&target));
        let one = runtime
            .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
            .unwrap_or_else(|_| unreachable!("single-envelope pump"));
        assert_eq!(one.report().processed_envelopes(), 1);
        assert_eq!(runtime.focus().focused_node(), Some(&target));
        assert!(runtime.index().node(&target).is_some());
        assert!(log.borrow().is_empty());
        assert!(!runtime.trace().records().skip(trace_start).any(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::FocusNotificationResolved {
                    kind: FocusEventKind::Out
                }
            )
        }));
    }
}

#[test]
fn sibling_reorder_and_disablement_delivers_on_unchanged_old_route_once() {
    let (mut runtime, log, target) = focused_reconciliation_fixture(256);
    runtime
        .submit_action(ReconciliationFocusAction::ReorderAndDisable)
        .unwrap_or_else(|_| unreachable!("reordered disablement admitted"));
    assert!(
        runtime
            .pump(FULL_PUMP)
            .unwrap_or_else(|_| unreachable!("reordered disablement settles"))
            .report()
            .is_quiescent()
    );
    assert_eq!(runtime.focus().focused_node(), None);
    assert!(runtime.index().node(&target).is_some());
    let log = log.borrow();
    assert_eq!(
        log.iter()
            .map(|event| (event.widget, event.phase, event.kind, event.reason))
            .collect::<Vec<_>>(),
        [
            (
                "root",
                EventPhase::Capture,
                FocusEventKind::Out,
                FocusReason::Disablement
            ),
            (
                "a",
                EventPhase::Target,
                FocusEventKind::Out,
                FocusReason::Disablement
            ),
            (
                "root",
                EventPhase::Bubble,
                FocusEventKind::Out,
                FocusReason::Disablement
            ),
        ]
    );
}

#[test]
fn removal_and_replacement_suppress_post_unmount_focus_out() {
    for change in [
        ReconciliationFocusAction::Remove,
        ReconciliationFocusAction::Replace,
    ] {
        let (mut runtime, log, old) = focused_reconciliation_fixture(256);
        let trace_start = runtime.trace().len();
        runtime
            .submit_action(change)
            .unwrap_or_else(|_| unreachable!("retirement action admitted"));
        assert!(
            runtime
                .pump(FULL_PUMP)
                .unwrap_or_else(|_| unreachable!("retirement settles"))
                .report()
                .is_quiescent()
        );
        assert_eq!(runtime.focus().focused_node(), None);
        assert_eq!(runtime.focus().reason(), Some(FocusReason::Removal));
        assert!(runtime.index().node(&old).is_none());
        assert!(
            log.borrow().is_empty(),
            "no callback may target an unmounted old widget"
        );
        let out = runtime
            .trace()
            .records()
            .skip(trace_start)
            .filter(|record| {
                matches!(
                    record.kind(),
                    TraceRecordKind::FocusNotificationResolved {
                        kind: FocusEventKind::Out
                    }
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(out.len(), 1);
        assert_eq!(
            out[0].context().delivery(),
            Some(TraceDeliveryOutcome::Suppressed)
        );
    }
}

#[test]
fn disabled_trace_does_not_disable_real_focus_out_delivery() {
    let (mut runtime, log, target) = focused_reconciliation_fixture(0);
    runtime
        .submit_action(ReconciliationFocusAction::Disable)
        .unwrap_or_else(|_| unreachable!("disablement admitted"));
    assert!(
        runtime
            .pump(FULL_PUMP)
            .unwrap_or_else(|_| unreachable!("disablement settles"))
            .report()
            .is_quiescent()
    );
    assert!(runtime.index().node(&target).is_some());
    assert_eq!(runtime.focus().focused_node(), None);
    assert_eq!(runtime.trace().len(), 0);
    assert_eq!(
        log.borrow()
            .iter()
            .map(|event| (event.widget, event.phase, event.kind))
            .collect::<Vec<_>>(),
        [
            ("root", EventPhase::Capture, FocusEventKind::Out),
            ("a", EventPhase::Target, FocusEventKind::Out),
            ("root", EventPhase::Bubble, FocusEventKind::Out),
        ]
    );
    log.borrow_mut().clear();
    runtime
        .submit_action(ReconciliationFocusAction::Disable)
        .unwrap_or_else(|_| unreachable!("idempotent disablement admitted"));
    runtime
        .pump(FULL_PUMP)
        .unwrap_or_else(|_| unreachable!("idempotent disablement settles"));
    assert!(log.borrow().is_empty(), "retired focus cannot notify twice");
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ReparentFocusAction {
    Noop,
    Move,
}

struct ReparentFocusState {
    log: Rc<RefCell<Vec<ExternalFocusFact>>>,
    right: bool,
}

struct ReparentFocusApp;

impl UiApp for ReparentFocusApp {
    type State = ReparentFocusState;
    type Action = ReparentFocusAction;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> Element<Self::Action> {
        let target = Element::new(ExternalFocusWidget::new("a", Rc::clone(&state.log), true))
            .id("reparent.a")
            .key("a")
            .focusable(true);
        let (left, right) = if state.right {
            (Vec::new(), vec![target])
        } else {
            (vec![target], Vec::new())
        };
        column(vec![
            container(
                ExternalFocusWidget::new("left", Rc::clone(&state.log), false),
                left,
            )
            .id("reparent.left")
            .key("left")
            .into_element(),
            container(
                ExternalFocusWidget::new("right", Rc::clone(&state.log), false),
                right,
            )
            .id("reparent.right")
            .key("right")
            .into_element(),
        ])
        .id("reparent.root")
        .key("root")
        .into_element()
        .map_action(|()| ReparentFocusAction::Noop)
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        if action == ReparentFocusAction::Move {
            state.right = true;
        }
    }
}

/// Cross-parent keyed moves do not preserve mounted identity in the public
/// reconciliation model. The old route must never be traversed after unmount.
#[test]
fn cross_parent_reparent_remounts_and_suppresses_old_focus_route() {
    let log = Rc::new(RefCell::new(Vec::<ExternalFocusFact>::new()));
    let mut runtime = AppRuntime::<ReparentFocusApp>::mount(ReparentFocusState {
        log: Rc::clone(&log),
        right: false,
    });
    assert!(
        runtime
            .pump(FULL_PUMP)
            .unwrap_or_else(|_| unreachable!("initial mount"))
            .report()
            .is_quiescent()
    );
    let old = runtime
        .index()
        .nodes()
        .iter()
        .find(|node| {
            node.authored_id()
                .is_some_and(|id| id.as_str() == "reparent.a")
        })
        .unwrap_or_else(|| unreachable!("initial A exists"))
        .id()
        .clone();
    runtime
        .submit_command(
            old.clone(),
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("old A accepts focus"));
    runtime
        .pump(FULL_PUMP)
        .unwrap_or_else(|_| unreachable!("focus settles"));
    assert_eq!(runtime.focus().focused_node(), Some(&old));
    log.borrow_mut().clear();
    let trace_start = runtime.trace().len();

    runtime
        .submit_action(ReparentFocusAction::Move)
        .unwrap_or_else(|_| unreachable!("move action accepted"));
    assert!(
        runtime
            .pump(FULL_PUMP)
            .unwrap_or_else(|_| unreachable!("move settles"))
            .report()
            .is_quiescent()
    );
    let new = runtime
        .index()
        .nodes()
        .iter()
        .find(|node| {
            node.authored_id()
                .is_some_and(|id| id.as_str() == "reparent.a")
        })
        .unwrap_or_else(|| unreachable!("new A exists"))
        .id()
        .clone();
    assert_ne!(
        old, new,
        "a cross-parent move creates a new mounted lifetime"
    );
    assert!(runtime.index().node(&old).is_none());
    assert_eq!(runtime.focus().focused_node(), None);
    assert_eq!(runtime.focus().reason(), Some(FocusReason::Removal));
    assert!(
        log.borrow().is_empty(),
        "retired old route cannot receive callbacks"
    );
    let outcomes = runtime
        .trace()
        .records()
        .skip(trace_start)
        .filter(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::FocusNotificationResolved {
                    kind: FocusEventKind::Out
                }
            )
        })
        .map(|record| record.context().delivery())
        .collect::<Vec<_>>();
    assert_eq!(outcomes, [Some(TraceDeliveryOutcome::Suppressed)]);
}

#[test]
fn shutdown_suppresses_focus_out_without_routing_to_unmounted_owners() {
    let (mut runtime, log, old) = focused_reconciliation_fixture(256);
    let trace_start = runtime.trace().len();
    runtime
        .shutdown()
        .unwrap_or_else(|_| unreachable!("shutdown completes"));
    assert_eq!(runtime.focus().focused_node(), None);
    assert!(runtime.index().node(&old).is_none());
    assert!(
        log.borrow().is_empty(),
        "shutdown cannot dispatch focus callbacks"
    );
    let outcomes = runtime
        .trace()
        .records()
        .skip(trace_start)
        .filter(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::FocusNotificationResolved {
                    kind: FocusEventKind::Out
                }
            )
        })
        .map(|record| record.context().delivery())
        .collect::<Vec<_>>();
    assert_eq!(outcomes, [Some(TraceDeliveryOutcome::Suppressed)]);
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum HeldKeyAction {
    Disable,
}

struct HeldKeyState {
    held: Rc<RefCell<Vec<PhysicalKey>>>,
    focus_outs: Rc<RefCell<usize>>,
    unmounts: Rc<RefCell<usize>>,
    enabled: bool,
}

#[derive(Debug)]
struct HeldKeyProbe {
    held: Rc<RefCell<Vec<PhysicalKey>>>,
    focus_outs: Rc<RefCell<usize>>,
    unmounts: Rc<RefCell<usize>>,
    enabled: bool,
}

impl Widget<HeldKeyAction> for HeldKeyProbe {
    type State = Rc<RefCell<Vec<PhysicalKey>>>;

    fn create_state(&self) -> Self::State {
        Rc::clone(&self.held)
    }

    fn activation(&self, _: &Self::State) -> WidgetActivation {
        WidgetActivation::actionable(self.enabled)
    }

    fn event(
        &mut self,
        state: &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, HeldKeyAction>,
    ) -> WidgetEventOutput {
        if context.phase() != EventPhase::Target {
            return WidgetEventOutput::none();
        }
        match event {
            UiEvent::Keyboard(key) if key.phase() == KeyboardPhase::Down && !key.is_repeat() => {
                state.borrow_mut().push(key.physical_key().clone());
                WidgetEventOutput::changed()
            }
            UiEvent::Focus(focus) if focus.kind() == FocusEventKind::Out => {
                state.borrow_mut().clear();
                *self.focus_outs.borrow_mut() += 1;
                WidgetEventOutput::changed()
            }
            _ => WidgetEventOutput::none(),
        }
    }

    fn unmount(&self, _: &mut Self::State, _: &mut runenui_core::WidgetUnmountContext) {
        *self.unmounts.borrow_mut() += 1;
    }
}

struct HeldKeyApp;

impl UiApp for HeldKeyApp {
    type State = HeldKeyState;
    type Action = HeldKeyAction;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> Element<Self::Action> {
        column(vec![
            Element::new(HeldKeyProbe {
                held: Rc::clone(&state.held),
                focus_outs: Rc::clone(&state.focus_outs),
                unmounts: Rc::clone(&state.unmounts),
                enabled: state.enabled,
            })
            .id("held.a")
            .key("held.a")
            .focusable(true),
        ])
        .id("held.root")
        .key("held.root")
        .into_element()
    }

    fn update(state: &mut Self::State, HeldKeyAction::Disable: Self::Action) {
        state.enabled = false;
    }
}

/// A concrete downstream widget retains key state in its mounted `Widget::State`.
/// Compatible disablement must invoke its live `FocusOut` cleanup instead of
/// reporting a trace-only or suppressed departure, without unmounting the widget.
#[test]
fn live_reconciliation_focus_out_clears_widget_held_key_without_unmount() {
    let held = Rc::new(RefCell::new(Vec::<PhysicalKey>::new()));
    let focus_outs = Rc::new(RefCell::new(0_usize));
    let unmounts = Rc::new(RefCell::new(0_usize));
    let mut runtime = AppRuntime::<HeldKeyApp>::mount(HeldKeyState {
        held: Rc::clone(&held),
        focus_outs: Rc::clone(&focus_outs),
        unmounts: Rc::clone(&unmounts),
        enabled: true,
    });
    assert!(
        runtime
            .pump(FULL_PUMP)
            .unwrap_or_else(|_| unreachable!("mount completes"))
            .report()
            .is_quiescent()
    );
    let a = runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id().is_some_and(|id| id.as_str() == "held.a"))
        .unwrap_or_else(|| unreachable!("key owner is mounted"))
        .id()
        .clone();
    runtime
        .submit_command(
            a.clone(),
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("programmatic focus admitted"));
    runtime
        .pump(FULL_PUMP)
        .unwrap_or_else(|_| unreachable!("focus settles"));
    assert_eq!(runtime.focus().focused_node(), Some(&a));

    runtime
        .submit_keyboard(group_character("w"))
        .unwrap_or_else(|_| unreachable!("keyboard Down admitted"));
    runtime
        .pump(FULL_PUMP)
        .unwrap_or_else(|_| unreachable!("key press settles"));
    assert_eq!(
        held.borrow().as_slice(),
        [PhysicalKey::Code(String::from("KeyX"))],
        "mounted widget state owns the held key before focus loss"
    );

    runtime
        .submit_action(HeldKeyAction::Disable)
        .unwrap_or_else(|_| unreachable!("disable action admitted"));
    runtime
        .pump(FULL_PUMP)
        .unwrap_or_else(|_| unreachable!("disablement settles"));
    assert_eq!(runtime.focus().focused_node(), None);
    assert!(runtime.index().node(&a).is_some());
    assert!(
        held.borrow().is_empty(),
        "FocusOut clears widget-local held state"
    );
    assert_eq!(*focus_outs.borrow(), 1);
    assert_eq!(*unmounts.borrow(), 0);

    runtime
        .submit_action(HeldKeyAction::Disable)
        .unwrap_or_else(|_| unreachable!("idempotent disablement admitted"));
    runtime
        .pump(FULL_PUMP)
        .unwrap_or_else(|_| unreachable!("second update settles"));
    assert_eq!(
        *focus_outs.borrow(),
        1,
        "the retired focus lifetime cannot notify twice"
    );
    assert_eq!(*unmounts.borrow(), 0);
}
