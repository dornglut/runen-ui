#![allow(refining_impl_trait)]

use core::num::NonZeroUsize;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use runenui_core::{
    Element, ElementId, EventContext, LogicalLength, LogicalPoint, NoHostProtocol, PointerButton,
    PointerButtons, PointerCaptureKind, PointerDeviceKind, PointerEvent, PointerId, PointerPhase,
    StyleTokens, SurfaceInputContext, UiApp, UiEvent, View, Widget, WidgetActivation,
    WidgetActivationContext, WidgetActivationOutput, WidgetEventOutput, WidgetMeasure, children,
    row,
};
use runenui_runtime::{
    AppRuntime, LogicalSize, PumpBudget, RuntimeConfig, SurfaceBuildContext, SurfacePublication,
    TraceDeliveryOutcome, TraceEventFamily, TracePointerRejection, TraceRecord, TraceRecordKind,
    TraceTarget,
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

    fn measure(&self, _state: &Self::State) -> WidgetMeasure {
        WidgetMeasure::Fixed {
            width: LogicalLength::new(32.0).unwrap_or_default(),
            height: LogicalLength::new(32.0).unwrap_or_default(),
        }
    }
}

struct Harness {
    runtime: AppRuntime<App>,
    context: SurfaceInputContext,
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
    let authored =
        ElementId::new("target").unwrap_or_else(|_| unreachable!("the test id is valid"));
    let node = publication
        .frame()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored))
        .unwrap_or_else(|| unreachable!("the target is published"));
    let bounds = node.bounds();
    let point = LogicalPoint::new(
        bounds.x() + bounds.width() / 2.0,
        bounds.y() + bounds.height() / 2.0,
    )
    .unwrap_or_else(|_| unreachable!("published bounds are finite"));
    pump_all(&mut runtime);
    Harness {
        runtime,
        context: publication.input_context().clone(),
        target: node.id().clone(),
        point,
        observations,
        activations,
    }
}

fn publish(runtime: &mut AppRuntime<App>) -> SurfacePublication {
    let tokens = StyleTokens::default();
    let size = LogicalSize::try_new(64.0, 64.0)
        .unwrap_or_else(|_| unreachable!("the test surface size is finite"));
    runtime.publish_surface(&SurfaceBuildContext::tight(&tokens, size))
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

fn assert_retired_up_cleanup(cleanup: &TraceRecord, harness: &Harness) {
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
    assert_eq!(surface.surface_id(), harness.context.surface_id());
    assert_eq!(
        surface.coordinate_revision(),
        harness.context.coordinate_revision()
    );
    assert_eq!(
        surface.hit_test_generation(),
        harness.context.hit_test_generation()
    );
    assert_eq!(surface.snapshot(), None);
    let path = context
        .physical_path()
        .unwrap_or_else(|| unreachable!("cleanup owns the prior physical path"));
    assert_eq!(path.targets().len(), 1);
    assert_eq!(path.targets()[0].mounted_node_id(), &harness.target);
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

fn assert_retired_up_capture_loss(capture: &TraceRecord, harness: &Harness) {
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
    assert_eq!(surface.surface_id(), harness.context.surface_id());
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
    assert_eq!(path.targets().len(), 1);
    assert_eq!(path.targets()[0].mounted_node_id(), &harness.target);
    let transition = context
        .target_transition()
        .unwrap_or_else(|| unreachable!("capture loss owns exact capture endpoints"));
    assert_eq!(
        transition.previous().map(TraceTarget::mounted_node_id),
        Some(&harness.target)
    );
    assert_eq!(transition.current(), None);
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
    let rejected = records
        .iter()
        .rev()
        .copied()
        .find(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::PointerIngressRejected {
                    pointer_id,
                    phase: PointerPhase::Up,
                    outcome: TracePointerRejection::RetiredGeneration,
                } if pointer_id.get() == 71
            )
        })
        .unwrap_or_else(|| unreachable!("retired up is diagnosed"));
    let cleanup = records
        .iter()
        .rev()
        .copied()
        .find(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::PointerIntegrityCleanupCommitted
            ) && record
                .context()
                .pointer()
                .is_some_and(|pointer| pointer.pointer_id().get() == 71)
        })
        .unwrap_or_else(|| unreachable!("retired up commits exact interaction cleanup"));
    let closed = records
        .iter()
        .rev()
        .copied()
        .find(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::PointerStreamClosed { pointer_id } if pointer_id.get() == 71
            )
        })
        .unwrap_or_else(|| unreachable!("retired up closes the stream"));
    let capture_lost = records
        .iter()
        .rev()
        .copied()
        .find(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::PointerCaptureNotificationResolved {
                    kind: PointerCaptureKind::Lost,
                }
            ) && record
                .context()
                .pointer()
                .is_some_and(|pointer| pointer.pointer_id().get() == 71)
        })
        .unwrap_or_else(|| unreachable!("live capture loss is resolved"));

    assert_retired_up_cleanup(cleanup, &harness);
    assert_retired_up_capture_loss(capture_lost, &harness);
    assert_eq!(cleanup.causal_parent(), Some(rejected.sequence()));
    assert_eq!(closed.causal_parent(), Some(cleanup.sequence()));
    assert_causal_ancestor(&records, capture_lost, closed);
    assert_eq!(cleanup.instant(), capture_lost.instant());
    assert!(cleanup.instant().is_some());

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
