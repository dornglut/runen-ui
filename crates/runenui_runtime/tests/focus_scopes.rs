#![allow(refining_impl_trait)]

use std::{cell::RefCell, rc::Rc};

use runenui_core::{
    Element, ElementId, EventContext, FocusBoundaryPolicy, FocusEventKind, FocusReason, FocusScope,
    FocusScopePolicy, InputModality, KeyLocation, KeyModifiers, KeyboardCompositionState,
    KeyboardEvent, KeyboardPhase, LogicalKey, NoHostProtocol, PhysicalKey, SemanticCommand,
    StyleTokens, UiApp, UiEvent, View, Widget, WidgetEventOutput, children, column, row, text,
};
use runenui_runtime::{
    AppRuntime, LogicalSize, PumpBudget, SurfaceBuildContext, SurfacePublication,
    TraceDeliveryOutcome, TraceEventFamily, TraceFocusBoundaryOutcome, TraceRecord,
    TraceRecordKind, TraceSurfaceSnapshotKind, TraceTarget, WorkSequence,
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

struct FocusFixture {
    runtime: AppRuntime<App>,
    publication: SurfacePublication,
    log: Rc<RefCell<Vec<FocusObservation>>>,
    root: runenui_core::MountedNodeId,
    scope: runenui_core::MountedNodeId,
    first: runenui_core::MountedNodeId,
    second: runenui_core::MountedNodeId,
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

fn publish<App: UiApp>(runtime: &mut AppRuntime<App>) -> SurfacePublication {
    let tokens = StyleTokens::default();
    let size = LogicalSize::try_new(320.0, 240.0)
        .unwrap_or_else(|_| unreachable!("positive logical size is valid"));
    runtime.publish_surface(&SurfaceBuildContext::tight(&tokens, size))
}

fn target_by_id(publication: &SurfacePublication, id: &str) -> runenui_core::MountedNodeId {
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

fn focus_fixture() -> FocusFixture {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = AppRuntime::<App>::mount(State {
        show_first: true,
        log: Rc::clone(&log),
    });
    let publication = publish(&mut runtime);
    pump_all(&mut runtime);
    FocusFixture {
        root: target_by_id(&publication, "root"),
        scope: target_by_id(&publication, "scope"),
        first: target_by_id(&publication, "first"),
        second: target_by_id(&publication, "second"),
        runtime,
        publication,
        log,
    }
}

fn request_focus<App: UiApp>(
    runtime: &mut AppRuntime<App>,
    publication: &SurfacePublication,
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

const fn keyboard_event(
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

#[derive(Clone, Copy)]
struct FocusEventExpectation<'a> {
    observer: &'static str,
    kind: FocusEventKind,
    reason: FocusReason,
    original_target: &'a runenui_core::MountedNodeId,
    related_target: Option<&'a runenui_core::MountedNodeId>,
    phase: runenui_core::EventPhase,
    current_target: &'a runenui_core::MountedNodeId,
}

fn assert_focus_event(observation: &FocusObservation, expected: FocusEventExpectation<'_>) {
    assert_eq!(observation.observer, expected.observer);
    assert_eq!(observation.kind, expected.kind);
    assert_eq!(observation.reason, expected.reason);
    assert_eq!(&observation.original_target, expected.original_target);
    assert_eq!(observation.related_target.as_ref(), expected.related_target);
    assert_eq!(observation.phase, expected.phase);
    assert_eq!(&observation.current_target, expected.current_target);
}

#[derive(Clone, Copy)]
struct FocusRouteExpectation<'a> {
    kind: FocusEventKind,
    root: &'a runenui_core::MountedNodeId,
    scope: &'a runenui_core::MountedNodeId,
    target: &'a runenui_core::MountedNodeId,
    related: &'a runenui_core::MountedNodeId,
    previous: &'a runenui_core::MountedNodeId,
    current: &'a runenui_core::MountedNodeId,
}

fn assert_delivered_focus_record(record: &TraceRecord, expected: FocusRouteExpectation<'_>) {
    assert!(matches!(
        record.kind(),
        TraceRecordKind::FocusNotificationResolved { kind } if *kind == expected.kind
    ));
    let context = record.context();
    let event = context
        .event()
        .unwrap_or_else(|| unreachable!("delivered focus notification owns event classification"));
    assert_eq!(event.family(), TraceEventFamily::Focus);
    assert!(!event.is_cancelable());
    assert_eq!(context.delivery(), Some(TraceDeliveryOutcome::Delivered));
    let route = context
        .route()
        .unwrap_or_else(|| unreachable!("delivered focus notification owns its exact route"));
    assert_eq!(route.targets().len(), 3);
    assert_eq!(route.targets()[0].mounted_node_id(), expected.root);
    assert_eq!(route.targets()[1].mounted_node_id(), expected.scope);
    assert_eq!(route.targets()[2].mounted_node_id(), expected.target);
    assert_eq!(
        route.related_target().map(TraceTarget::mounted_node_id),
        Some(expected.related)
    );
    let endpoints = context
        .target_transition()
        .unwrap_or_else(|| unreachable!("delivered focus notification owns exact endpoints"));
    assert_eq!(
        endpoints.previous().map(TraceTarget::mounted_node_id),
        Some(expected.previous)
    );
    assert_eq!(
        endpoints.current().map(TraceTarget::mounted_node_id),
        Some(expected.current)
    );
}

fn descends_from(
    records: &[&TraceRecord],
    descendant: &TraceRecord,
    ancestor: &TraceRecord,
) -> bool {
    let mut parent = descendant.causal_parent();
    for _ in 0..records.len() {
        let Some(sequence) = parent else {
            return false;
        };
        if sequence == ancestor.sequence() {
            return true;
        }
        parent = records
            .iter()
            .find(|record| record.sequence() == sequence)
            .and_then(|record| record.causal_parent());
    }
    false
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

struct FocusTransferRecords<'a> {
    transition: &'a TraceRecord,
    within: &'a TraceRecord,
    out: &'a TraceRecord,
    input: &'a TraceRecord,
}

fn focus_transfer_records<'a>(records: &[&'a TraceRecord]) -> FocusTransferRecords<'a> {
    FocusTransferRecords {
        transition: record_for(records, |kind| {
            matches!(
                kind,
                TraceRecordKind::FocusTransitionCommitted {
                    reason: FocusReason::ProgrammaticRequest,
                }
            )
        }),
        within: record_for(records, |kind| {
            matches!(
                kind,
                TraceRecordKind::FocusWithinInvalidated {
                    left: 1,
                    entered: 1,
                }
            )
        }),
        out: record_for(records, |kind| {
            matches!(
                kind,
                TraceRecordKind::FocusNotificationResolved {
                    kind: FocusEventKind::Out,
                }
            )
        }),
        input: record_for(records, |kind| {
            matches!(
                kind,
                TraceRecordKind::FocusNotificationResolved {
                    kind: FocusEventKind::In,
                }
            )
        }),
    }
}

fn assert_transfer_lineage(records: &[&TraceRecord], facts: &FocusTransferRecords<'_>) {
    assert_eq!(facts.within.causal_parent(), Some(facts.transition.sequence()));
    assert!(descends_from(records, facts.out, facts.within));
    assert!(descends_from(records, facts.input, facts.out));
    assert_eq!(facts.transition.work_sequence(), facts.within.work_sequence());
    assert_eq!(facts.transition.work_sequence(), facts.out.work_sequence());
    assert_eq!(facts.transition.work_sequence(), facts.input.work_sequence());
    assert_eq!(facts.transition.instant(), facts.within.instant());
    assert_eq!(facts.transition.instant(), facts.out.instant());
    assert_eq!(facts.transition.instant(), facts.input.instant());
    assert!(facts.transition.instant().is_some());
}

fn assert_reconciliation_surface(record: &TraceRecord, publication: &SurfacePublication) {
    let surface = record
        .context()
        .surface()
        .unwrap_or_else(|| unreachable!("focus record owns displayed-surface identity"));
    let input = publication.input_context();
    assert_eq!(surface.surface_id(), input.surface_id());
    assert_eq!(surface.coordinate_revision(), input.coordinate_revision());
    assert_eq!(surface.hit_test_generation(), input.hit_test_generation());
    assert_eq!(surface.snapshot(), Some(TraceSurfaceSnapshotKind::Current));
}

fn assert_transfer_context(fixture: &FocusFixture, facts: &FocusTransferRecords<'_>) {
    let endpoints = facts
        .transition
        .context()
        .target_transition()
        .unwrap_or_else(|| unreachable!("focus transition owns exact endpoints"));
    assert_eq!(
        endpoints.previous().map(TraceTarget::mounted_node_id),
        Some(&fixture.first)
    );
    assert_eq!(
        endpoints.current().map(TraceTarget::mounted_node_id),
        Some(&fixture.second)
    );
    assert_reconciliation_surface(facts.transition, &fixture.publication);
    assert_delivered_focus_record(
        facts.out,
        FocusRouteExpectation {
            kind: FocusEventKind::Out,
            root: &fixture.root,
            scope: &fixture.scope,
            target: &fixture.first,
            related: &fixture.second,
            previous: &fixture.first,
            current: &fixture.second,
        },
    );
    assert_delivered_focus_record(
        facts.input,
        FocusRouteExpectation {
            kind: FocusEventKind::In,
            root: &fixture.root,
            scope: &fixture.scope,
            target: &fixture.second,
            related: &fixture.first,
            previous: &fixture.first,
            current: &fixture.second,
        },
    );
    assert_reconciliation_surface(facts.out, &fixture.publication);
    assert_reconciliation_surface(facts.input, &fixture.publication);
}

#[test]
fn nested_focus_notifications_use_exact_routes_and_related_targets() {
    let mut fixture = focus_fixture();
    request_focus(
        &mut fixture.runtime,
        &fixture.publication,
        fixture.first.clone(),
    );
    fixture.log.borrow_mut().clear();
    let trace_start = fixture.runtime.trace().len();
    request_focus(
        &mut fixture.runtime,
        &fixture.publication,
        fixture.second.clone(),
    );

    let observations = fixture.log.borrow();
    assert_eq!(observations.len(), 2);
    assert_focus_event(
        &observations[0],
        FocusEventExpectation {
            observer: "first",
            kind: FocusEventKind::Out,
            reason: FocusReason::ProgrammaticRequest,
            original_target: &fixture.first,
            related_target: Some(&fixture.second),
            phase: runenui_core::EventPhase::Target,
            current_target: &fixture.first,
        },
    );
    assert_focus_event(
        &observations[1],
        FocusEventExpectation {
            observer: "second",
            kind: FocusEventKind::In,
            reason: FocusReason::ProgrammaticRequest,
            original_target: &fixture.second,
            related_target: Some(&fixture.first),
            phase: runenui_core::EventPhase::Target,
            current_target: &fixture.second,
        },
    );
    drop(observations);

    let records = fixture
        .runtime
        .trace()
        .records()
        .skip(trace_start)
        .collect::<Vec<_>>();
    let facts = focus_transfer_records(&records);
    assert_transfer_lineage(&records, &facts);
    assert_transfer_context(&fixture, &facts);
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
    request_focus(&mut runtime, &publication, first);
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
    request_focus(&mut runtime, &publication, second);

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

struct FocusCleanupRecords<'a> {
    transition: &'a TraceRecord,
    within: &'a TraceRecord,
    resolved: &'a TraceRecord,
}

fn focus_cleanup_records<'a>(records: &[&'a TraceRecord]) -> FocusCleanupRecords<'a> {
    FocusCleanupRecords {
        transition: record_for(records, |kind| {
            matches!(
                kind,
                TraceRecordKind::FocusTransitionCommitted {
                    reason: FocusReason::Removal,
                }
            )
        }),
        within: record_for(records, |kind| {
            matches!(
                kind,
                TraceRecordKind::FocusWithinInvalidated { left, entered: 0 } if *left >= 1
            )
        }),
        resolved: record_for(records, |kind| {
            matches!(
                kind,
                TraceRecordKind::FocusNotificationResolved {
                    kind: FocusEventKind::Out,
                }
            )
        }),
    }
}

fn assert_cleanup_lineage(facts: &FocusCleanupRecords<'_>, sequence: WorkSequence) {
    assert_eq!(facts.transition.work_sequence(), Some(sequence));
    assert_eq!(facts.within.work_sequence(), Some(sequence));
    assert_eq!(facts.resolved.work_sequence(), Some(sequence));
    assert_eq!(facts.within.causal_parent(), Some(facts.transition.sequence()));
    assert_eq!(facts.resolved.causal_parent(), Some(facts.within.sequence()));
    assert_eq!(facts.transition.instant(), facts.within.instant());
    assert_eq!(facts.transition.instant(), facts.resolved.instant());
    assert!(facts.transition.instant().is_some());
    assert_eq!(
        facts.transition.reconciliation_before(),
        facts.within.reconciliation_before()
    );
    assert_eq!(
        facts.transition.reconciliation_after(),
        facts.within.reconciliation_after()
    );
    assert_eq!(
        facts.transition.reconciliation_before(),
        facts.resolved.reconciliation_before()
    );
    assert_eq!(
        facts.transition.reconciliation_after(),
        facts.resolved.reconciliation_after()
    );
}

fn assert_cleanup_context(fixture: &FocusFixture, facts: &FocusCleanupRecords<'_>) {
    let endpoints = facts
        .transition
        .context()
        .target_transition()
        .unwrap_or_else(|| unreachable!("focus transition owns exact endpoints"));
    assert_eq!(
        endpoints.previous().map(TraceTarget::mounted_node_id),
        Some(&fixture.first)
    );
    assert_eq!(endpoints.current(), None);
    assert_reconciliation_surface(facts.transition, &fixture.publication);

    let context = facts.resolved.context();
    let event = context
        .event()
        .unwrap_or_else(|| unreachable!("focus resolution owns event classification"));
    assert_eq!(event.family(), TraceEventFamily::Focus);
    assert!(!event.is_cancelable());
    assert_eq!(context.delivery(), Some(TraceDeliveryOutcome::Suppressed));
    let route = context
        .route()
        .unwrap_or_else(|| unreachable!("suppressed focus cleanup retains the old route"));
    assert_eq!(route.targets().len(), 3);
    assert_eq!(route.targets()[0].mounted_node_id(), &fixture.root);
    assert_eq!(route.targets()[1].mounted_node_id(), &fixture.scope);
    assert_eq!(route.targets()[2].mounted_node_id(), &fixture.first);
    assert_eq!(route.related_target(), None);
    let notification_endpoints = context
        .target_transition()
        .unwrap_or_else(|| unreachable!("focus resolution owns exact endpoints"));
    assert_eq!(
        notification_endpoints
            .previous()
            .map(TraceTarget::mounted_node_id),
        Some(&fixture.first)
    );
    assert_eq!(notification_endpoints.current(), None);
    assert_reconciliation_surface(facts.resolved, &fixture.publication);
}

#[test]
fn focus_is_cleared_and_notification_is_suppressed_when_target_disappears() {
    let mut fixture = focus_fixture();
    request_focus(
        &mut fixture.runtime,
        &fixture.publication,
        fixture.first.clone(),
    );
    let trace_start = fixture.runtime.trace().len();
    let sequence = fixture
        .runtime
        .submit_action(Action::HideFirst)
        .unwrap_or_else(|_| unreachable!("hide action is admitted"));
    pump_all(&mut fixture.runtime);

    assert_eq!(fixture.runtime.focus().focused_node(), None);
    let records = fixture
        .runtime
        .trace()
        .records()
        .skip(trace_start)
        .collect::<Vec<_>>();
    let facts = focus_cleanup_records(&records);
    assert_cleanup_lineage(&facts, sequence);
    assert_cleanup_context(&fixture, &facts);
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
