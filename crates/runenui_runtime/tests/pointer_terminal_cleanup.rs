#![allow(refining_impl_trait)]

use core::num::NonZeroUsize;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use runenui_core::{
    Element, ElementId, EventContext, HitContribution, HitContributionContext, LogicalLength,
    LogicalPoint, LogicalRect, NoHostProtocol, PointerButton, PointerButtons, PointerCaptureKind,
    PointerDeviceKind, PointerEvent, PointerId, PointerPhase, StyleEnvironment,
    SurfaceInputContext, UiApp, UiEvent, View, Widget, WidgetActivation, WidgetActivationContext,
    WidgetActivationOutput, WidgetEventOutput, WidgetMeasure, children, row,
};
use runenui_runtime::{
    AppRuntime, LogicalSize, PumpBudget, RuntimeConfig, SurfaceBuildContext, SurfacePublication,
    TraceDeliveryOutcome, TraceEventFamily, TracePointerRejection, TraceRecord, TraceRecordKind,
    TraceSurfaceContext, TraceTarget,
};

#[derive(Clone, Debug, Eq, PartialEq)]
enum Observation {
    Pointer(PointerPhase),
    Capture(PointerCaptureKind),
}

#[derive(Clone)]
struct State {
    visible: bool,
    observations: Rc<RefCell<Vec<Observation>>>,
    activations: Rc<Cell<usize>>,
}

#[derive(Clone, Copy)]
enum Action {
    Hide,
    Activated,
}

struct App;

impl UiApp for App {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        let children = if state.visible {
            children![
                Element::new(Probe {
                    observations: Rc::clone(&state.observations),
                })
                .id("target")
                .key("target"),
            ]
        } else {
            children![]
        };
        row(children).id("root").key("root")
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            Action::Hide => state.visible = false,
            Action::Activated => state.activations.set(state.activations.get() + 1),
        }
    }
}

#[derive(Debug)]
struct Probe {
    observations: Rc<RefCell<Vec<Observation>>>,
}

impl Widget<Action> for Probe {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn event(
        &mut self,
        _state: &mut Self::State,
        event: &UiEvent,
        _context: &mut EventContext<'_, Action>,
    ) -> WidgetEventOutput {
        match event {
            UiEvent::Pointer(pointer) => self
                .observations
                .borrow_mut()
                .push(Observation::Pointer(pointer.phase())),
            UiEvent::PointerCapture(capture) => self
                .observations
                .borrow_mut()
                .push(Observation::Capture(capture.kind())),
            _ => {}
        }
        WidgetEventOutput::none()
    }

    fn activation(&self, _state: &Self::State) -> WidgetActivation {
        WidgetActivation::actionable(true)
    }

    fn activate(
        &mut self,
        _state: &mut Self::State,
        _context: &mut WidgetActivationContext<Action>,
    ) -> WidgetActivationOutput<Action> {
        WidgetActivationOutput::action(Action::Activated)
    }

    fn measure(
        &self,
        _state: &Self::State,
        _input: runenui_core::WidgetMeasureInput,
    ) -> WidgetMeasure {
        WidgetMeasure::measured(
            LogicalLength::new(32.0).unwrap_or_default(),
            LogicalLength::new(32.0).unwrap_or_default(),
        )
    }

    fn hit_test(&self, _state: &Self::State, context: HitContributionContext) -> HitContribution {
        let size = context.local_size();
        let rect = LogicalRect::try_new(0.0, 0.0, size.width(), size.height())
            .unwrap_or_else(|_| unreachable!("validated local size yields a valid hit rectangle"));
        HitContribution::single_rect(rect)
    }
}

struct Harness {
    runtime: AppRuntime<App>,
    context: SurfaceInputContext,
    root: runenui_core::MountedNodeId,
    target: runenui_core::MountedNodeId,
    point: LogicalPoint,
    observations: Rc<RefCell<Vec<Observation>>>,
    activations: Rc<Cell<usize>>,
}

fn harness(config: RuntimeConfig) -> Harness {
    let observations = Rc::new(RefCell::new(Vec::new()));
    let activations = Rc::new(Cell::new(0));
    let mut runtime = AppRuntime::<App>::mount_with_config(
        State {
            visible: true,
            observations: Rc::clone(&observations),
            activations: Rc::clone(&activations),
        },
        config,
    );
    let publication = publish(&mut runtime);
    let root_authored =
        ElementId::new("root").unwrap_or_else(|_| unreachable!("the test id is valid"));
    let target_authored =
        ElementId::new("target").unwrap_or_else(|_| unreachable!("the test id is valid"));
    let root = publication
        .frame()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&root_authored))
        .unwrap_or_else(|| unreachable!("the root is published"));
    let target = publication
        .frame()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&target_authored))
        .unwrap_or_else(|| unreachable!("the target is published"));
    let bounds = target.bounds();
    let point = LogicalPoint::new(
        bounds.x() + bounds.width() / 2.0,
        bounds.y() + bounds.height() / 2.0,
    )
    .unwrap_or_else(|_| unreachable!("published bounds are finite"));
    pump_all(&mut runtime);
    Harness {
        runtime,
        context: publication.input_context().clone(),
        root: root.id().clone(),
        target: target.id().clone(),
        point,
        observations,
        activations,
    }
}

fn publish(runtime: &mut AppRuntime<App>) -> SurfacePublication {
    let environment = StyleEnvironment::default();
    let size = LogicalSize::try_new(64.0, 64.0)
        .unwrap_or_else(|_| unreachable!("the test surface size is finite"));
    runtime
        .publish_surface(&SurfaceBuildContext::tight(&environment, size))
        .unwrap_or_else(|_| unreachable!("the test surface publication is admitted"))
}

fn pointer_event(
    pointer_id: u64,
    phase: PointerPhase,
    context: &SurfaceInputContext,
    point: LogicalPoint,
) -> PointerEvent {
    let pointer_id =
        PointerId::new(pointer_id).unwrap_or_else(|| unreachable!("the pointer id is non-zero"));
    let event = PointerEvent::new(
        pointer_id,
        PointerDeviceKind::Mouse,
        phase,
        point,
        context.clone(),
    );
    match phase {
        PointerPhase::Down => event
            .with_buttons(PointerButtons::new([PointerButton::Primary]))
            .with_changed_button(PointerButton::Primary),
        PointerPhase::Up => event.with_changed_button(PointerButton::Primary),
        _ => event,
    }
}

const fn full_budget() -> PumpBudget {
    PumpBudget::new(usize::MAX, usize::MAX, usize::MAX, usize::MAX)
}

fn pump_all(runtime: &mut AppRuntime<App>) {
    assert!(runtime.pump(full_budget()).is_quiescent());
}

fn submit_and_pump(runtime: &mut AppRuntime<App>, event: PointerEvent) {
    runtime
        .submit_pointer(event)
        .unwrap_or_else(|_| unreachable!("the pointer event is accepted"));
    pump_all(runtime);
}

fn assert_causal_ancestor(
    records: &[&TraceRecord],
    descendant: &TraceRecord,
    ancestor: &TraceRecord,
) {
    let mut parent = descendant.causal_parent();
    while parent != Some(ancestor.sequence()) {
        let sequence =
            parent.unwrap_or_else(|| unreachable!("capture loss must descend from stream closure"));
        parent = records
            .iter()
            .copied()
            .find(|record| record.sequence() == sequence)
            .unwrap_or_else(|| unreachable!("every retained parent is present in this trace"))
            .causal_parent();
    }
}

fn assert_retired_up_cleanup(
    cleanup: &TraceRecord,
    harness: &Harness,
    stream_surface: &SurfaceInputContext,
) {
    assert!(matches!(
        cleanup.kind(),
        TraceRecordKind::PointerIntegrityCleanupCommitted
    ));
    let context = cleanup.context();
    assert_eq!(context.event(), None);
    let pointer = context
        .pointer()
        .unwrap_or_else(|| unreachable!("cleanup owns pointer-stream identity"));
    assert_eq!(pointer.pointer_id().get(), 71);
    assert_eq!(pointer.device_id(), None);
    assert_eq!(pointer.device_kind(), PointerDeviceKind::Mouse);
    assert_eq!(pointer.phase(), None);
    let surface = context
        .surface()
        .unwrap_or_else(|| unreachable!("cleanup owns prior stream surface identity"));
    assert_eq!(surface.surface_id(), stream_surface.surface_id());
    assert_eq!(
        surface.coordinate_revision(),
        stream_surface.coordinate_revision()
    );
    assert_eq!(
        surface.hit_test_generation(),
        stream_surface.hit_test_generation()
    );
    assert_eq!(surface.snapshot(), None);
    let path = context
        .physical_path()
        .unwrap_or_else(|| unreachable!("cleanup owns the prior physical path"));
    assert_eq!(path.targets().len(), 2);
    assert_eq!(path.targets()[0].mounted_node_id(), &harness.root);
    assert_eq!(path.targets()[1].mounted_node_id(), &harness.target);
    let facts = context
        .pointer_cleanup()
        .unwrap_or_else(|| unreachable!("cleanup owns exact interaction transitions"));
    let pressed = facts
        .pressed_owner()
        .unwrap_or_else(|| unreachable!("pressed ownership is cleared"));
    assert_eq!(
        pressed.previous().map(TraceTarget::mounted_node_id),
        Some(&harness.target)
    );
    assert_eq!(pressed.current(), None);
    let capture = facts
        .capture_owner()
        .unwrap_or_else(|| unreachable!("capture ownership is cleared"));
    assert_eq!(
        capture.previous().map(TraceTarget::mounted_node_id),
        Some(&harness.target)
    );
    assert_eq!(capture.current(), None);
    assert!(facts.physical_path_cleared());
    assert_eq!(context.route(), None);
    assert_eq!(context.delivery(), None);
}

fn assert_retired_up_capture_loss(
    capture: &TraceRecord,
    harness: &Harness,
    stream_surface: &SurfaceInputContext,
) {
    assert!(matches!(
        capture.kind(),
        TraceRecordKind::PointerCaptureNotificationResolved {
            kind: PointerCaptureKind::Lost,
        }
    ));
    assert_eq!(
        capture.target().map(TraceTarget::mounted_node_id),
        Some(&harness.target)
    );
    let context = capture.context();
    let event = context
        .event()
        .unwrap_or_else(|| unreachable!("capture loss owns event classification"));
    assert_eq!(event.family(), TraceEventFamily::PointerCapture);
    assert!(!event.is_cancelable());
    assert_eq!(context.delivery(), Some(TraceDeliveryOutcome::Delivered));
    let pointer = context
        .pointer()
        .unwrap_or_else(|| unreachable!("capture loss owns pointer-stream identity"));
    assert_eq!(pointer.pointer_id().get(), 71);
    assert_eq!(pointer.device_id(), None);
    assert_eq!(pointer.device_kind(), PointerDeviceKind::Mouse);
    assert_eq!(pointer.phase(), None);
    let surface = context
        .surface()
        .unwrap_or_else(|| unreachable!("capture loss owns prior stream surface identity"));
    assert_eq!(surface.surface_id(), stream_surface.surface_id());
    assert_eq!(
        surface.coordinate_revision(),
        stream_surface.coordinate_revision()
    );
    assert_eq!(
        surface.hit_test_generation(),
        stream_surface.hit_test_generation()
    );
    assert_eq!(surface.snapshot(), None);
    let route = context
        .route()
        .unwrap_or_else(|| unreachable!("capture loss owns its target-only route"));
    assert_eq!(route.targets().len(), 1);
    assert_eq!(route.targets()[0].mounted_node_id(), &harness.target);
    assert_eq!(route.related_target(), None);
    let path = context
        .physical_path()
        .unwrap_or_else(|| unreachable!("capture loss owns the prior physical path"));
    assert_eq!(path.targets().len(), 2);
    assert_eq!(path.targets()[0].mounted_node_id(), &harness.root);
    assert_eq!(path.targets()[1].mounted_node_id(), &harness.target);
    let transition = context
        .target_transition()
        .unwrap_or_else(|| unreachable!("capture loss owns exact capture endpoints"));
    assert_eq!(
        transition.previous().map(TraceTarget::mounted_node_id),
        Some(&harness.target)
    );
    assert_eq!(transition.current(), None);
}

struct RetiredUpRecords<'a> {
    rejected: &'a TraceRecord,
    cleanup: &'a TraceRecord,
    closed: &'a TraceRecord,
    capture_lost: &'a TraceRecord,
}

fn retired_up_records<'a>(records: &[&'a TraceRecord]) -> RetiredUpRecords<'a> {
    let find = |predicate: &dyn Fn(&TraceRecord) -> bool, message| {
        records
            .iter()
            .rev()
            .copied()
            .find(|record| predicate(record))
            .unwrap_or_else(|| unreachable!("{message}"))
    };
    let rejected = find(
        &|record| {
            matches!(
                record.kind(),
                TraceRecordKind::PointerIngressRejected {
                    pointer_id,
                    phase: PointerPhase::Up,
                    outcome: TracePointerRejection::RetiredGeneration,
                } if pointer_id.get() == 71
            )
        },
        "retired up is diagnosed",
    );
    let cleanup = find(
        &|record| {
            matches!(
                record.kind(),
                TraceRecordKind::PointerIntegrityCleanupCommitted
            ) && record
                .context()
                .pointer()
                .is_some_and(|pointer| pointer.pointer_id().get() == 71)
        },
        "retired up commits exact interaction cleanup",
    );
    let closed = find(
        &|record| {
            matches!(
                record.kind(),
                TraceRecordKind::PointerStreamClosed { pointer_id } if pointer_id.get() == 71
            )
        },
        "retired up closes the stream",
    );
    let capture_lost = find(
        &|record| {
            matches!(
                record.kind(),
                TraceRecordKind::PointerCaptureNotificationResolved {
                    kind: PointerCaptureKind::Lost,
                }
            ) && record
                .context()
                .pointer()
                .is_some_and(|pointer| pointer.pointer_id().get() == 71)
        },
        "live capture loss is resolved",
    );
    RetiredUpRecords {
        rejected,
        cleanup,
        closed,
        capture_lost,
    }
}

#[test]
fn retired_context_up_closes_without_ordinary_route_or_activation_and_notifies_capture_loss() {
    let retention =
        NonZeroUsize::new(1).unwrap_or_else(|| unreachable!("the test retention is non-zero"));
    let config = RuntimeConfig::default().with_surface_snapshot_retention(retention);
    let mut harness = harness(config);
    submit_and_pump(
        &mut harness.runtime,
        pointer_event(71, PointerPhase::Down, &harness.context, harness.point),
    );
    let current = publish(&mut harness.runtime);
    pump_all(&mut harness.runtime);
    assert_ne!(
        harness.context.hit_test_generation(),
        current.input_context().hit_test_generation()
    );
    harness.observations.borrow_mut().clear();
    harness.activations.set(0);

    submit_and_pump(
        &mut harness.runtime,
        pointer_event(71, PointerPhase::Up, &harness.context, harness.point),
    );

    assert_eq!(
        harness.observations.borrow().as_slice(),
        [Observation::Capture(PointerCaptureKind::Lost)]
    );
    assert_eq!(harness.activations.get(), 0);
    let records = harness.runtime.trace().records().collect::<Vec<_>>();
    let retained = retired_up_records(&records);

    assert_retired_up_cleanup(retained.cleanup, &harness, current.input_context());
    assert_retired_up_capture_loss(retained.capture_lost, &harness, current.input_context());
    assert_ne!(
        retained
            .cleanup
            .context()
            .surface()
            .map(TraceSurfaceContext::hit_test_generation),
        Some(harness.context.hit_test_generation())
    );
    assert_eq!(
        retained.cleanup.causal_parent(),
        Some(retained.rejected.sequence())
    );
    assert_eq!(
        retained.closed.causal_parent(),
        Some(retained.cleanup.sequence())
    );
    assert_causal_ancestor(&records, retained.capture_lost, retained.closed);
    assert_eq!(retained.cleanup.instant(), retained.capture_lost.instant());
    assert!(retained.cleanup.instant().is_some());

    submit_and_pump(
        &mut harness.runtime,
        pointer_event(
            71,
            PointerPhase::Cancel,
            current.input_context(),
            harness.point,
        ),
    );
    assert!(harness.runtime.trace().kinds().any(|kind| matches!(
        kind,
        TraceRecordKind::PointerIngressRejected {
            pointer_id,
            phase: PointerPhase::Cancel,
            outcome: TracePointerRejection::MissingStream,
        } if pointer_id.get() == 71
    )));
}

#[test]
fn retired_context_cancel_diagnoses_geometry_but_routes_cleanup_and_closes() {
    let retention =
        NonZeroUsize::new(1).unwrap_or_else(|| unreachable!("the test retention is non-zero"));
    let config = RuntimeConfig::default().with_surface_snapshot_retention(retention);
    let mut harness = harness(config);
    submit_and_pump(
        &mut harness.runtime,
        pointer_event(73, PointerPhase::Down, &harness.context, harness.point),
    );
    let _current = publish(&mut harness.runtime);
    pump_all(&mut harness.runtime);
    harness.observations.borrow_mut().clear();
    harness.activations.set(0);

    submit_and_pump(
        &mut harness.runtime,
        pointer_event(73, PointerPhase::Cancel, &harness.context, harness.point),
    );

    assert_eq!(
        harness.observations.borrow().as_slice(),
        [
            Observation::Pointer(PointerPhase::Cancel),
            Observation::Capture(PointerCaptureKind::Lost),
        ]
    );
    assert_eq!(harness.activations.get(), 0);
    assert!(harness.runtime.trace().kinds().any(|kind| matches!(
        kind,
        TraceRecordKind::PointerContextUnavailable {
            pointer_id,
            outcome: TracePointerRejection::RetiredGeneration,
        } if pointer_id.get() == 73
    )));
    assert!(harness.runtime.trace().kinds().any(|kind| matches!(
        kind,
        TraceRecordKind::PointerStreamClosed { pointer_id } if pointer_id.get() == 73
    )));
}

#[test]
fn cancel_closes_hover_stream_after_target_removal_without_stale_callback() {
    let mut harness = harness(RuntimeConfig::default());
    submit_and_pump(
        &mut harness.runtime,
        pointer_event(73, PointerPhase::Move, &harness.context, harness.point),
    );
    harness.observations.borrow_mut().clear();
    harness
        .runtime
        .submit_action(Action::Hide)
        .unwrap_or_else(|_| unreachable!("the hide action is accepted"));
    pump_all(&mut harness.runtime);

    submit_and_pump(
        &mut harness.runtime,
        pointer_event(73, PointerPhase::Cancel, &harness.context, harness.point),
    );

    assert!(harness.observations.borrow().is_empty());
    assert!(harness.runtime.trace().kinds().any(|kind| matches!(
        kind,
        TraceRecordKind::PointerStreamClosed { pointer_id } if pointer_id.get() == 73
    )));
    submit_and_pump(
        &mut harness.runtime,
        pointer_event(73, PointerPhase::Cancel, &harness.context, harness.point),
    );
    assert!(harness.runtime.trace().kinds().any(|kind| matches!(
        kind,
        TraceRecordKind::PointerIngressRejected {
            pointer_id,
            phase: PointerPhase::Cancel,
            outcome: TracePointerRejection::MissingStream,
        } if pointer_id.get() == 73
    )));
}
