#![allow(refining_impl_trait)]

use std::{cell::RefCell, rc::Rc};

use runenui_core::{
    Element, EventContext, LogicalLength, LogicalPoint, MountedNodeId, NoHostProtocol,
    PointerButton, PointerButtons, PointerDeviceKind, PointerEvent, PointerId, PointerPhase,
    StyleTokens, SurfaceInputContext, UiApp, UiEvent, View, Widget, WidgetActivation,
    WidgetEventOutput, WidgetMeasure, children, row,
};
use runenui_runtime::{AppRuntime, LogicalSize, PumpBudget, SurfaceBuildContext, TraceRecordKind};

#[derive(Clone, Debug, Eq, PartialEq)]
struct Observation {
    widget: &'static str,
    pointer_id: PointerId,
    phase: PointerPhase,
    current_target: MountedNodeId,
    physical_target: Option<MountedNodeId>,
}

#[derive(Clone)]
struct State {
    observations: Rc<RefCell<Vec<Observation>>>,
}

struct App;

impl UiApp for App {
    type State = State;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        row(children![
            Element::new(Probe {
                name: "left",
                observations: Rc::clone(&state.observations),
            })
            .id("left")
            .key("left"),
            Element::new(Probe {
                name: "right",
                observations: Rc::clone(&state.observations),
            })
            .id("right")
            .key("right"),
        ])
        .key("root")
    }

    fn update(_state: &mut Self::State, _action: Self::Action) {}
}

#[derive(Debug)]
struct Probe {
    name: &'static str,
    observations: Rc<RefCell<Vec<Observation>>>,
}

impl Widget<()> for Probe {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn event(
        &mut self,
        _state: &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, ()>,
    ) -> WidgetEventOutput {
        if let UiEvent::Pointer(pointer) = event {
            self.observations.borrow_mut().push(Observation {
                widget: self.name,
                pointer_id: pointer.pointer_id(),
                phase: pointer.phase(),
                current_target: context.current_target().clone(),
                physical_target: context.physical_target().cloned(),
            });
        }
        WidgetEventOutput::none()
    }

    fn activation(&self, _state: &Self::State) -> WidgetActivation {
        WidgetActivation::actionable(true)
    }

    fn measure(&self, _state: &Self::State) -> WidgetMeasure {
        WidgetMeasure::Fixed {
            width: LogicalLength::new(24.0).unwrap_or_default(),
            height: LogicalLength::new(24.0).unwrap_or_default(),
        }
    }
}

struct Harness {
    runtime: AppRuntime<App>,
    context: SurfaceInputContext,
    left_id: MountedNodeId,
    right_id: MountedNodeId,
    left_point: LogicalPoint,
    right_point: LogicalPoint,
    observations: Rc<RefCell<Vec<Observation>>>,
}

fn harness() -> Harness {
    let observations = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = AppRuntime::<App>::mount(State {
        observations: Rc::clone(&observations),
    });
    let tokens = StyleTokens::default();
    let size = LogicalSize::try_new(64.0, 32.0)
        .unwrap_or_else(|_| unreachable!("the test surface size is finite"));
    let publication = runtime.publish_surface(&SurfaceBuildContext::tight(&tokens, size));
    let left = publication
        .frame()
        .nodes()
        .iter()
        .find(|node| node.authored_id().is_some_and(|id| id.as_str() == "left"))
        .unwrap_or_else(|| unreachable!("the left probe is published"));
    let right = publication
        .frame()
        .nodes()
        .iter()
        .find(|node| node.authored_id().is_some_and(|id| id.as_str() == "right"))
        .unwrap_or_else(|| unreachable!("the right probe is published"));
    let left_bounds = left.bounds();
    let right_bounds = right.bounds();
    Harness {
        runtime,
        context: publication.input_context().clone(),
        left_id: left.id().clone(),
        right_id: right.id().clone(),
        left_point: LogicalPoint::new(left_bounds.x() + 1.0, left_bounds.y() + 1.0)
            .unwrap_or_else(|_| unreachable!("the left point is finite")),
        right_point: LogicalPoint::new(right_bounds.x() + 1.0, right_bounds.y() + 1.0)
            .unwrap_or_else(|_| unreachable!("the right point is finite")),
        observations,
    }
}

fn pointer(value: u64) -> PointerId {
    PointerId::new(value).unwrap_or_else(|| unreachable!("the pointer id is non-zero"))
}

fn pointer_event(
    context: &SurfaceInputContext,
    pointer_id: u64,
    phase: PointerPhase,
    position: LogicalPoint,
    primary_pressed: bool,
) -> PointerEvent {
    let buttons = if primary_pressed {
        PointerButtons::new([PointerButton::Primary])
    } else {
        PointerButtons::default()
    };
    let event = PointerEvent::new(
        pointer(pointer_id),
        PointerDeviceKind::Touch,
        phase,
        position,
        context.clone(),
    )
    .with_buttons(buttons);
    match phase {
        PointerPhase::Down | PointerPhase::Up => event.with_changed_button(PointerButton::Primary),
        _ => event,
    }
}

fn submit_and_pump(runtime: &mut AppRuntime<App>, event: PointerEvent) {
    runtime
        .submit_pointer(event)
        .unwrap_or_else(|_| unreachable!("the pointer event is accepted"));
    assert!(
        runtime
            .pump(PumpBudget::new(
                usize::MAX,
                usize::MAX,
                usize::MAX,
                usize::MAX,
            ))
            .is_quiescent()
    );
}

fn establish_captured_streams(harness: &mut Harness) {
    submit_and_pump(
        &mut harness.runtime,
        pointer_event(
            &harness.context,
            91,
            PointerPhase::Down,
            harness.left_point,
            true,
        ),
    );
    submit_and_pump(
        &mut harness.runtime,
        pointer_event(
            &harness.context,
            92,
            PointerPhase::Down,
            harness.right_point,
            true,
        ),
    );
    harness.observations.borrow_mut().clear();
}

fn assert_crossed_capture_routes(harness: &mut Harness) {
    submit_and_pump(
        &mut harness.runtime,
        pointer_event(
            &harness.context,
            91,
            PointerPhase::Move,
            harness.right_point,
            true,
        ),
    );
    submit_and_pump(
        &mut harness.runtime,
        pointer_event(
            &harness.context,
            92,
            PointerPhase::Move,
            harness.left_point,
            true,
        ),
    );
    assert_eq!(
        harness.observations.borrow().as_slice(),
        [
            Observation {
                widget: "left",
                pointer_id: pointer(91),
                phase: PointerPhase::Move,
                current_target: harness.left_id.clone(),
                physical_target: Some(harness.right_id.clone()),
            },
            Observation {
                widget: "right",
                pointer_id: pointer(92),
                phase: PointerPhase::Move,
                current_target: harness.right_id.clone(),
                physical_target: Some(harness.left_id.clone()),
            },
        ]
    );
    harness.observations.borrow_mut().clear();
}

fn cancel_first_stream(harness: &mut Harness) {
    submit_and_pump(
        &mut harness.runtime,
        pointer_event(
            &harness.context,
            91,
            PointerPhase::Cancel,
            harness.right_point,
            false,
        ),
    );
    assert_eq!(
        harness.observations.borrow().as_slice(),
        [Observation {
            widget: "left",
            pointer_id: pointer(91),
            phase: PointerPhase::Cancel,
            current_target: harness.left_id.clone(),
            physical_target: Some(harness.right_id.clone()),
        }]
    );
    harness.observations.borrow_mut().clear();
}

fn assert_second_stream_survives(harness: &mut Harness) {
    submit_and_pump(
        &mut harness.runtime,
        pointer_event(
            &harness.context,
            92,
            PointerPhase::Move,
            harness.left_point,
            true,
        ),
    );
    assert_eq!(
        harness.observations.borrow().as_slice(),
        [Observation {
            widget: "right",
            pointer_id: pointer(92),
            phase: PointerPhase::Move,
            current_target: harness.right_id.clone(),
            physical_target: Some(harness.left_id.clone()),
        }]
    );
}

fn close_second_and_assert_trace(harness: &mut Harness) {
    submit_and_pump(
        &mut harness.runtime,
        pointer_event(
            &harness.context,
            92,
            PointerPhase::Cancel,
            harness.left_point,
            false,
        ),
    );
    assert_eq!(
        harness
            .runtime
            .trace()
            .kinds()
            .filter(|kind| matches!(kind, TraceRecordKind::PointerStreamClosed { .. }))
            .count(),
        2
    );
    for expected in [91, 92] {
        assert!(harness.runtime.trace().kinds().any(|kind| matches!(
            kind,
            TraceRecordKind::PointerStreamClosed { pointer_id }
                if pointer_id.get() == expected
        )));
    }
}

#[test]
fn simultaneous_captured_streams_route_and_terminate_independently() {
    let mut harness = harness();
    establish_captured_streams(&mut harness);
    assert_crossed_capture_routes(&mut harness);
    cancel_first_stream(&mut harness);
    assert_second_stream_survives(&mut harness);
    close_second_and_assert_trace(&mut harness);
}
