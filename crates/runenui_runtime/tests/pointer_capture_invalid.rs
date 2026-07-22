#![allow(refining_impl_trait)]

use std::{cell::RefCell, rc::Rc};

use runenui_core::{
    Axis, ChildLayout, ChildLayoutWidget, Element, ElementId, EventContext, LogicalLength,
    LogicalPoint, NoHostProtocol, PointerBoundaryKind, PointerButton, PointerButtons,
    PointerCaptureKind, PointerDeviceKind, PointerEvent, PointerId, PointerPhase, StyleTokens,
    SurfaceInputContext, UiApp, UiEvent, View, Widget, WidgetActivation, WidgetEventOutput,
    WidgetMeasure, children, container,
};
use runenui_runtime::{
    AppRuntime, LogicalSize, MountedNodeId, PumpBudget, SurfaceBuildContext,
    TracePointerCaptureRequestRejection, TraceRecordKind,
};

#[derive(Clone, Debug, Eq, PartialEq)]
struct CaptureObservation {
    widget: &'static str,
    kind: PointerCaptureKind,
    related: Option<MountedNodeId>,
}

#[derive(Clone)]
struct State {
    captures: Rc<RefCell<Vec<CaptureObservation>>>,
}

struct App;

impl UiApp for App {
    type State = State;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        container(
            InvalidReleaseRoot,
            children![
                Element::new(CaptureProbe {
                    name: "left",
                    transfer_on_enter: false,
                    captures: Rc::clone(&state.captures),
                })
                .id("left")
                .key("left"),
                Element::new(CaptureProbe {
                    name: "right",
                    transfer_on_enter: true,
                    captures: Rc::clone(&state.captures),
                })
                .id("right")
                .key("right"),
            ],
        )
        .id("root")
        .key("root")
    }

    fn update(_state: &mut Self::State, _action: Self::Action) {}
}

#[derive(Debug)]
struct InvalidReleaseRoot;

impl Widget<()> for InvalidReleaseRoot {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn event(
        &mut self,
        _state: &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, ()>,
    ) -> WidgetEventOutput {
        if matches!(event, UiEvent::Pointer(pointer) if pointer.phase() == PointerPhase::Move) {
            context.release_pointer_capture();
        }
        WidgetEventOutput::none()
    }
}

impl ChildLayoutWidget<()> for InvalidReleaseRoot {
    fn child_layout(&self, _state: &Self::State) -> ChildLayout {
        ChildLayout::Linear {
            axis: Axis::Horizontal,
        }
    }
}

#[derive(Debug)]
struct CaptureProbe {
    name: &'static str,
    transfer_on_enter: bool,
    captures: Rc<RefCell<Vec<CaptureObservation>>>,
}

impl Widget<()> for CaptureProbe {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn event(
        &mut self,
        _state: &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, ()>,
    ) -> WidgetEventOutput {
        match event {
            UiEvent::PointerBoundary(boundary)
                if self.transfer_on_enter && boundary.kind() == PointerBoundaryKind::Enter =>
            {
                context.capture_pointer();
            }
            UiEvent::PointerCapture(capture) => {
                self.captures.borrow_mut().push(CaptureObservation {
                    widget: self.name,
                    kind: capture.kind(),
                    related: capture.related_owner().cloned(),
                });
            }
            _ => {}
        }
        WidgetEventOutput::none()
    }

    fn activation(&self, _state: &Self::State) -> WidgetActivation {
        WidgetActivation::actionable(true)
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
    left: MountedNodeId,
    right: MountedNodeId,
    left_point: LogicalPoint,
    right_point: LogicalPoint,
    captures: Rc<RefCell<Vec<CaptureObservation>>>,
}

fn harness() -> Harness {
    let captures = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = AppRuntime::<App>::mount(State {
        captures: Rc::clone(&captures),
    });
    let tokens = StyleTokens::default();
    let size = LogicalSize::try_new(96.0, 48.0)
        .unwrap_or_else(|_| unreachable!("the test surface size is finite"));
    let publication = runtime.publish_surface(&SurfaceBuildContext::tight(&tokens, size));
    let left_authored =
        ElementId::new("left").unwrap_or_else(|_| unreachable!("the test id is valid"));
    let right_authored =
        ElementId::new("right").unwrap_or_else(|_| unreachable!("the test id is valid"));
    let left_node = publication
        .frame()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&left_authored))
        .unwrap_or_else(|| unreachable!("the left node is published"));
    let right_node = publication
        .frame()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&right_authored))
        .unwrap_or_else(|| unreachable!("the right node is published"));
    let left_bounds = left_node.bounds();
    let right_bounds = right_node.bounds();
    let left_point = LogicalPoint::new(
        left_bounds.x() + left_bounds.width() / 2.0,
        left_bounds.y() + left_bounds.height() / 2.0,
    )
    .unwrap_or_else(|_| unreachable!("published bounds are finite"));
    let right_point = LogicalPoint::new(
        right_bounds.x() + right_bounds.width() / 2.0,
        right_bounds.y() + right_bounds.height() / 2.0,
    )
    .unwrap_or_else(|_| unreachable!("published bounds are finite"));
    Harness {
        runtime,
        context: publication.input_context().clone(),
        left: left_node.id().clone(),
        right: right_node.id().clone(),
        left_point,
        right_point,
        captures,
    }
}

fn pointer_event(
    context: &SurfaceInputContext,
    point: LogicalPoint,
    phase: PointerPhase,
) -> PointerEvent {
    let pointer_id =
        PointerId::new(61).unwrap_or_else(|| unreachable!("the pointer id is non-zero"));
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
        PointerPhase::Move => event.with_buttons(PointerButtons::new([PointerButton::Primary])),
        _ => event,
    }
}

fn submit_and_pump(runtime: &mut AppRuntime<App>, event: PointerEvent) {
    runtime
        .submit_pointer(event)
        .unwrap_or_else(|_| unreachable!("the pointer event is accepted"));
    let report = runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
    assert!(report.is_quiescent());
}

#[test]
fn invalid_later_releases_do_not_erase_an_earlier_valid_transfer() {
    let mut harness = harness();
    submit_and_pump(
        &mut harness.runtime,
        pointer_event(&harness.context, harness.left_point, PointerPhase::Down),
    );
    harness.captures.borrow_mut().clear();

    submit_and_pump(
        &mut harness.runtime,
        pointer_event(&harness.context, harness.right_point, PointerPhase::Move),
    );

    assert_eq!(
        harness.captures.borrow().as_slice(),
        [
            CaptureObservation {
                widget: "left",
                kind: PointerCaptureKind::Lost,
                related: Some(harness.right.clone()),
            },
            CaptureObservation {
                widget: "right",
                kind: PointerCaptureKind::Gained,
                related: Some(harness.left.clone()),
            },
        ]
    );
    assert_eq!(
        harness
            .runtime
            .trace()
            .kinds()
            .filter(|kind| matches!(
                kind,
                TraceRecordKind::PointerCaptureRequestRejected {
                    pointer_id,
                    outcome: TracePointerCaptureRequestRejection::ReleaseNotOwner,
                } if pointer_id.get() == 61
            ))
            .count(),
        2
    );
}
