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
    TracePointerRejection, TraceRecordKind,
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
        .find(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::PointerIntegrityCleanupCommitted {
                    pointer_id,
                    pressed: true,
                    capture: true,
                    physical_path: true,
                } if pointer_id.get() == 71
            )
        })
        .unwrap_or_else(|| unreachable!("retired up commits exact interaction cleanup"));
    let capture_lost = records
        .iter()
        .rev()
        .find(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::PointerCaptureTransitionQueued {
                    pointer_id,
                    kind: PointerCaptureKind::Lost,
                } if pointer_id.get() == 71
            )
        })
        .unwrap_or_else(|| unreachable!("live capture loss is queued"));
    let closed = records
        .iter()
        .rev()
        .find(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::PointerStreamClosed { pointer_id } if pointer_id.get() == 71
            )
        })
        .unwrap_or_else(|| unreachable!("retired up closes the stream"));
    assert_eq!(cleanup.causal_parent(), Some(rejected.sequence()));
    assert_eq!(capture_lost.causal_parent(), Some(cleanup.sequence()));
    assert_eq!(closed.causal_parent(), Some(capture_lost.sequence()));

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
