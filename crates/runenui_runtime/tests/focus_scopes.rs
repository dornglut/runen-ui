#![allow(refining_impl_trait)]

use std::{cell::RefCell, rc::Rc};

use runenui_core::{
    Element, ElementId, EventContext, FocusBoundaryPolicy, FocusEventKind, FocusReason, FocusScope,
    FocusScopePolicy, InputModality, KeyLocation, KeyModifiers, KeyboardCompositionState,
    KeyboardEvent, KeyboardPhase, LogicalKey, NoHostProtocol, PhysicalKey, SemanticCommand,
    StyleTokens, UiApp, UiEvent, View, Widget, WidgetEventOutput, children, column, row, text,
};
use runenui_runtime::{
    AppRuntime, LogicalSize, PumpBudget, SurfaceBuildContext, TraceDeliveryOutcome,
    TraceEventFamily, TraceFocusBoundaryOutcome, TraceRecord, TraceRecordKind,
    TraceSurfaceSnapshotKind, TraceTarget,
};

#[derive(Clone, Debug, Eq, PartialEq)]
struct FocusObservation {
    observer: &'static str,
    kind: FocusEventKind,
    reason: FocusReason,
    original_target: runenui_core::MountedNodeId,
    related_target: Option<runenui_core::MountedNodeId>,
    phase: runenui_core::EventPhase,
    current_target: runenui_core::MountedNodeId,
}

#[derive(Clone, Debug)]
struct FocusProbe {
    name: &'static str,
    log: Rc<RefCell<Vec<FocusObservation>>>,
}

impl Widget<Action> for FocusProbe {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn event(
        &mut self,
        _state: &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, Action>,
    ) -> WidgetEventOutput {
        if let UiEvent::Focus(event) = event {
            self.log.borrow_mut().push(FocusObservation {
                observer: self.name,
                kind: event.kind(),
                reason: event.reason(),
                original_target: event.target().clone(),
                related_target: context.related_target().cloned(),
                phase: context.phase(),
                current_target: context.current_target().clone(),
            });
        }
        WidgetEventOutput::none()
    }
}

#[derive(Clone, Debug)]
struct State {
    show_first: bool,
    log: Rc<RefCell<Vec<FocusObservation>>>,
}

#[derive(Clone, Debug)]
enum Action {
    HideFirst,
}

struct App;

impl UiApp for App {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        let first = state.show_first.then(|| {
            Element::new(FocusProbe {
                name: "first",
                log: Rc::clone(&state.log),
            })
            .id("first")
            .key("first")
            .focusable(true)
        });
        let second = Element::new(FocusProbe {
            name: "second",
            log: Rc::clone(&state.log),
        })
        .id("second")
        .key("second")
        .focusable(true);
        let scope_children = match first {
            Some(first) => children![first, second],
            None => children![second],
        };
        let scope = column(scope_children)
            .id("scope")
            .key("scope")
            .into_element()
            .focus_scope(FocusScope::new().with_policy(FocusScopePolicy::new(
                FocusBoundaryPolicy::Wrap,
                FocusBoundaryPolicy::Trap,
            )));
        row(children![scope]).id("root").key("root")
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            Action::HideFirst => state.show_first = false,
        }
    }
}

#[derive(Clone, Debug)]
struct EmptyState;

struct EmptyApp;

impl UiApp for EmptyApp {
    type State = EmptyState;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(_state: &Self::State) -> impl View<Self::Action> {
        row(children![text("label")]).id("root").key("root")
    }

    fn update(_state: &mut Self::State, _action: Self::Action) {}
}

fn pump_all<App: UiApp>(runtime: &mut AppRuntime<App>) {
    let report = runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
    assert!(report.is_quiescent());
}

fn publish<App: UiApp>(runtime: &mut AppRuntime<App>) -> runenui_runtime::SurfacePublication {
    let tokens = StyleTokens::default();
    let size = LogicalSize::try_new(320.0, 240.0)
        .unwrap_or_else(|_| unreachable!("positive logical size is valid"));
    runtime.publish_surface(&SurfaceBuildContext::tight(&tokens, size))
}

fn target_by_id(
    publication: &runenui_runtime::SurfacePublication,
    id: &str,
) -> runenui_core::MountedNodeId {
    let id = ElementId::new(id).unwrap_or_else(|_| unreachable!("test id is valid"));
    publication
        .frame()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&id))
        .unwrap_or_else(|| unreachable!("authored test node is published"))
        .id()
        .clone()
}

fn request_focus<App: UiApp>(
    runtime: &mut AppRuntime<App>,
    publication: &runenui_runtime::SurfacePublication,
    target: runenui_core::MountedNodeId,
) {
    runtime
        .submit_resolved_surface_command(
            publication.input_context().clone(),
            target,
            SemanticCommand::RequestFocus,
            runenui_core::CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("the test focus request is accepted"));
    pump_all(runtime);
}

fn keyboard_event(
    phase: KeyboardPhase,
    physical: PhysicalKey,
    logical: LogicalKey,
) -> KeyboardEvent {
    KeyboardEvent::new(
        phase,
        physical,
        logical,
        KeyModifiers::NONE,
        false,
        KeyLocation::Standard,
        KeyboardCompositionState::Inactive,
        None,
    )
}

fn assert_focus_event(
    observation: &FocusObservation,
    observer: &'static str,
    kind: FocusEventKind,
    reason: FocusReason,
    original_target: &runenui_core::MountedNodeId,
    related_target: Option<&runenui_core::MountedNodeId>,
    phase: runenui_core::EventPhase,
    current_target: &runenui_core::MountedNodeId,
) {
    assert_eq!(observation.observer, observer);
    assert_eq!(observation.kind, kind);
    assert_eq!(observation.reason, reason);
    assert_eq!(&observation.original_target, original_target);
    assert_eq!(observation.related_target.as_ref(), related_target);
    assert_eq!(observation.phase, phase);
    assert_eq!(&observation.current_target, current_target);
}

#[test]
fn nested_focus_notifications_use_exact_routes_and_related_targets() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = AppRuntime::<App>::mount(State {
        show_first: true,
        log: Rc::clone(&log),
    });
    let publication = publish(&mut runtime);
    pump_all(&mut runtime);
    let root = target_by_id(&publication, "root");
    let scope = target_by_id(&publication, "scope");
    let first = target_by_id(&publication, "first");
    let second = target_by_id(&publication, "second");

    request_focus(&mut runtime, &publication, first.clone());
    log.borrow_mut().clear();
    request_focus(&mut runtime, &publication, second.clone());

    let observations = log.borrow();
    assert_eq!(observations.len(), 10);
    assert_focus_event(
        &observations[0],
        "first",
        FocusEventKind::Out,
        FocusReason::ProgrammaticRequest,
        &first,
        Some(&second),
        runenui_core::EventPhase::Capture,
        &root,
    );
    assert_focus_event(
        &observations[1],
        "first",
        FocusEventKind::Out,
        FocusReason::ProgrammaticRequest,
        &first,
        Some(&second),
        runenui_core::EventPhase::Capture,
        &scope,
    );
    assert_focus_event(
        &observations[2],
        "first",
        FocusEventKind::Out,
        FocusReason::ProgrammaticRequest,
        &first,
        Some(&second),
        runenui_core::EventPhase::Target,
        &first,
    );
    assert_focus_event(
        &observations[3],
        "first",
        FocusEventKind::Out,
        FocusReason::ProgrammaticRequest,
        &first,
        Some(&second),
        runenui_core::EventPhase::Bubble,
        &scope,
    );
    assert_focus_event(
        &observations[4],
        "first",
        FocusEventKind::Out,
        FocusReason::ProgrammaticRequest,
        &first,
        Some(&second),
        runenui_core::EventPhase::Bubble,
        &root,
    );
    assert_focus_event(
        &observations[5],
        "second",
        FocusEventKind::In,
        FocusReason::ProgrammaticRequest,
        &second,
        Some(&first),
        runenui_core::EventPhase::Capture,
        &root,
    );
    assert_focus_event(
        &observations[6],
        "second",
        FocusEventKind::In,
        FocusReason::ProgrammaticRequest,
        &second,
        Some(&first),
        runenui_core::EventPhase::Capture,
        &scope,
    );
    assert_focus_event(
        &observations[7],
        "second",
        FocusEventKind::In,
        FocusReason::ProgrammaticRequest,
        &second,
        Some(&first),
        runenui_core::EventPhase::Target,
        &second,
    );
    assert_focus_event(
        &observations[8],
        "second",
        FocusEventKind::In,
        FocusReason::ProgrammaticRequest,
        &second,
        Some(&first),
        runenui_core::EventPhase::Bubble,
        &scope,
    );
    assert_focus_event(
        &observations[9],
        "second",
        FocusEventKind::In,
        FocusReason::ProgrammaticRequest,
        &second,
        Some(&first),
        runenui_core::EventPhase::Bubble,
        &root,
    );
}

#[test]
fn tab_wraps_shift_tab_reverses_and_escape_is_not_focus_navigation() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = AppRuntime::<App>::mount(State {
        show_first: true,
        log,
    });
    let publication = publish(&mut runtime);
    pump_all(&mut runtime);
    let first = target_by_id(&publication, "first");
    let second = target_by_id(&publication, "second");
    request_focus(&mut runtime, &publication, first.clone());

    runtime
        .submit_keyboard(keyboard_event(
            KeyboardPhase::Down,
            PhysicalKey::Tab,
            LogicalKey::Tab,
        ))
        .unwrap_or_else(|_| unreachable!("Tab is admitted"));
    pump_all(&mut runtime);
    assert_eq!(runtime.focus().focused_node(), Some(&second));

    runtime
        .submit_keyboard(KeyboardEvent::new(
            KeyboardPhase::Down,
            PhysicalKey::Tab,
            LogicalKey::Tab,
            KeyModifiers::SHIFT,
            false,
            KeyLocation::Standard,
            KeyboardCompositionState::Inactive,
            None,
        ))
        .unwrap_or_else(|_| unreachable!("Shift+Tab is admitted"));
    pump_all(&mut runtime);
    assert_eq!(runtime.focus().focused_node(), Some(&first));

    runtime
        .submit_keyboard(keyboard_event(
            KeyboardPhase::Down,
            PhysicalKey::Escape,
            LogicalKey::Escape,
        ))
        .unwrap_or_else(|_| unreachable!("Escape is admitted"));
    pump_all(&mut runtime);
    assert_eq!(runtime.focus().focused_node(), Some(&first));
}

#[test]
fn keyboard_modality_is_committed_before_keyboard_default_focus_transfer() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = AppRuntime::<App>::mount(State {
        show_first: true,
        log,
    });
    let publication = publish(&mut runtime);
    pump_all(&mut runtime);
    let first = target_by_id(&publication, "first");
    let second = target_by_id(&publication, "second");
    request_focus(&mut runtime, &publication, first.clone());
    assert_eq!(
        runtime.focus().modality(),
        Some(InputModality::Programmatic)
    );

    runtime
        .submit_keyboard(keyboard_event(
            KeyboardPhase::Down,
            PhysicalKey::Tab,
            LogicalKey::Tab,
        ))
        .unwrap_or_else(|_| unreachable!("Tab is admitted"));
    pump_all(&mut runtime);

    assert_eq!(runtime.focus().modality(), Some(InputModality::Keyboard));
    assert_eq!(runtime.focus().focused_node(), Some(&second));
    let trace = runtime.trace().kinds().collect::<Vec<_>>();
    let modality = trace
        .iter()
        .rposition(|kind| matches!(kind, TraceRecordKind::ModalityChanged))
        .unwrap_or_else(|| unreachable!("modality change is traced"));
    let transition = trace
        .iter()
        .rposition(|kind| {
            matches!(
                kind,
                TraceRecordKind::FocusTransitionCommitted {
                    reason: FocusReason::LinearNavigation,
                }
            )
        })
        .unwrap_or_else(|| unreachable!("focus transition is traced"));
    assert!(modality < transition);
}

#[test]
fn scope_wrap_trap_and_restoration_are_deterministic() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = AppRuntime::<App>::mount(State {
        show_first: true,
        log,
    });
    let publication = publish(&mut runtime);
    pump_all(&mut runtime);
    let scope = target_by_id(&publication, "scope");
    let first = target_by_id(&publication, "first");
    let second = target_by_id(&publication, "second");
    request_focus(&mut runtime, &publication, second.clone());

    runtime
        .submit_resolved_surface_command(
            publication.input_context().clone(),
            scope.clone(),
            SemanticCommand::FocusNext,
            runenui_core::CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("focus next is admitted"));
    pump_all(&mut runtime);
    assert_eq!(runtime.focus().focused_node(), Some(&first));
    assert!(runtime.trace().kinds().any(|kind| matches!(
        kind,
        TraceRecordKind::FocusCandidateSelected {
            outcome: TraceFocusBoundaryOutcome::Wrapped,
        }
    )));

    runtime
        .submit_resolved_surface_command(
            publication.input_context().clone(),
            scope.clone(),
            SemanticCommand::FocusRight,
            runenui_core::CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("focus right is admitted"));
    pump_all(&mut runtime);
    assert_eq!(runtime.focus().focused_node(), Some(&first));
    assert!(runtime.trace().kinds().any(|kind| matches!(
        kind,
        TraceRecordKind::FocusCandidateSelected {
            outcome: TraceFocusBoundaryOutcome::Trapped,
        }
    )));

    runtime
        .submit_resolved_surface_command(
            publication.input_context().clone(),
            scope,
            SemanticCommand::RestoreFocus,
            runenui_core::CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("restore focus is admitted"));
    pump_all(&mut runtime);
    assert_eq!(runtime.focus().focused_node(), Some(&first));
    assert!(
        runtime
            .trace()
            .kinds()
            .any(|kind| matches!(kind, TraceRecordKind::FocusRestorationAccepted))
    );
}

fn record_for<'a>(
    records: &[&'a TraceRecord],
    predicate: impl Fn(&TraceRecordKind) -> bool,
) -> &'a TraceRecord {
    records
        .iter()
        .copied()
        .find(|record| predicate(record.kind()))
        .unwrap_or_else(|| unreachable!("the expected focus record is retained"))
}

fn assert_reconciliation_surface(
    record: &TraceRecord,
    publication: &runenui_runtime::SurfacePublication,
) {
    let surface = record
        .context()
        .surface()
        .unwrap_or_else(|| unreachable!("focus cleanup owns displayed-surface identity"));
    let input = publication.input_context();
    assert_eq!(surface.surface_id(), input.surface_id());
    assert_eq!(surface.coordinate_revision(), input.coordinate_revision());
    assert_eq!(surface.hit_test_generation(), input.hit_test_generation());
    assert_eq!(surface.snapshot(), Some(TraceSurfaceSnapshotKind::Current));
}

#[test]
fn focus_is_cleared_and_notification_is_suppressed_when_target_disappears() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = AppRuntime::<App>::mount(State {
        show_first: true,
        log,
    });
    let publication = publish(&mut runtime);
    pump_all(&mut runtime);
    let root = target_by_id(&publication, "root");
    let scope = target_by_id(&publication, "scope");
    let first = target_by_id(&publication, "first");
    request_focus(&mut runtime, &publication, first.clone());
    let trace_start = runtime.trace().len();

    let sequence = runtime
        .submit_action(Action::HideFirst)
        .unwrap_or_else(|_| unreachable!("hide action is admitted"));
    pump_all(&mut runtime);

    assert_eq!(runtime.focus().focused_node(), None);
    let records = runtime
        .trace()
        .records()
        .skip(trace_start)
        .collect::<Vec<_>>();
    let transition = record_for(&records, |kind| {
        matches!(
            kind,
            TraceRecordKind::FocusTransitionCommitted {
                reason: FocusReason::Removal,
            }
        )
    });
    let within = record_for(&records, |kind| {
        matches!(
            kind,
            TraceRecordKind::FocusWithinInvalidated { left, entered: 0 } if *left >= 1
        )
    });
    let resolved = record_for(&records, |kind| {
        matches!(
            kind,
            TraceRecordKind::FocusNotificationResolved {
                kind: FocusEventKind::Out,
            }
        )
    });

    assert_eq!(transition.work_sequence(), Some(sequence));
    assert_eq!(within.work_sequence(), Some(sequence));
    assert_eq!(resolved.work_sequence(), Some(sequence));
    assert_eq!(within.causal_parent(), Some(transition.sequence()));
    assert_eq!(resolved.causal_parent(), Some(within.sequence()));
    assert_eq!(transition.instant(), within.instant());
    assert_eq!(transition.instant(), resolved.instant());
    assert!(transition.instant().is_some());
    assert_eq!(
        transition.reconciliation_before(),
        within.reconciliation_before()
    );
    assert_eq!(
        transition.reconciliation_after(),
        within.reconciliation_after()
    );
    assert_eq!(
        transition.reconciliation_before(),
        resolved.reconciliation_before()
    );
    assert_eq!(
        transition.reconciliation_after(),
        resolved.reconciliation_after()
    );

    let transition_context = transition.context();
    let endpoints = transition_context
        .target_transition()
        .unwrap_or_else(|| unreachable!("focus transition owns exact endpoints"));
    assert_eq!(
        endpoints.previous().map(TraceTarget::mounted_node_id),
        Some(&first)
    );
    assert_eq!(endpoints.current(), None);
    assert_reconciliation_surface(transition, &publication);

    let resolved_context = resolved.context();
    let event = resolved_context
        .event()
        .unwrap_or_else(|| unreachable!("focus resolution owns event classification"));
    assert_eq!(event.family(), TraceEventFamily::Focus);
    assert!(!event.is_cancelable());
    assert_eq!(
        resolved_context.delivery(),
        Some(TraceDeliveryOutcome::Suppressed)
    );
    let route = resolved_context
        .route()
        .unwrap_or_else(|| unreachable!("suppressed focus cleanup retains the old route"));
    assert_eq!(route.targets().len(), 3);
    assert_eq!(route.targets()[0].mounted_node_id(), &root);
    assert_eq!(route.targets()[1].mounted_node_id(), &scope);
    assert_eq!(route.targets()[2].mounted_node_id(), &first);
    assert_eq!(route.related_target(), None);
    let notification_endpoints = resolved_context
        .target_transition()
        .unwrap_or_else(|| unreachable!("focus resolution owns exact endpoints"));
    assert_eq!(
        notification_endpoints
            .previous()
            .map(TraceTarget::mounted_node_id),
        Some(&first)
    );
    assert_eq!(notification_endpoints.current(), None);
    assert_reconciliation_surface(resolved, &publication);
    assert_eq!(
        records
            .iter()
            .filter(|record| {
                matches!(
                    record.kind(),
                    TraceRecordKind::FocusNotificationResolved { .. }
                ) && record.context().delivery() == Some(TraceDeliveryOutcome::Delivered)
            })
            .count(),
        0
    );
}

#[test]
fn empty_scope_does_not_move_focus_and_reports_empty_restoration() {
    let mut runtime = AppRuntime::<EmptyApp>::mount(EmptyState);
    let publication = publish(&mut runtime);
    pump_all(&mut runtime);
    let root = target_by_id(&publication, "root");

    runtime
        .submit_resolved_surface_command(
            publication.input_context().clone(),
            root,
            SemanticCommand::RestoreFocus,
            runenui_core::CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("restore focus is admitted"));
    pump_all(&mut runtime);

    assert_eq!(runtime.focus().focused_node(), None);
    assert!(runtime.trace().kinds().any(|kind| matches!(
        kind,
        TraceRecordKind::FocusCandidateSelected {
            outcome: TraceFocusBoundaryOutcome::Empty,
        }
    )));
}
