#![allow(refining_impl_trait)]

use std::{cell::RefCell, rc::Rc};

use runenui_core::{
    Element, ElementId, EventContext, EventPhase, LogicalLength, LogicalPoint, NoHostProtocol,
    PointerBoundaryKind, PointerButton, PointerButtons, PointerCaptureKind, PointerDeviceKind,
    PointerEvent, PointerId, PointerPhase, StyleTokens, SurfaceInputContext, UiApp, UiEvent, View,
    Widget, WidgetActivation, WidgetEventOutput, WidgetMeasure, children, row,
};
use runenui_runtime::{AppRuntime, LogicalSize, MountedNodeId, PumpBudget, SurfaceBuildContext};

#[derive(Clone, Debug, Eq, PartialEq)]
enum CallbackObservation {
    Boundary {
        widget: &'static str,
        kind: PointerBoundaryKind,
    },
    Pointer {
        widget: &'static str,
        phase: PointerPhase,
        routed_target: MountedNodeId,
        physical_target: Option<MountedNodeId>,
    },
    Capture {
        widget: &'static str,
        kind: PointerCaptureKind,
        phase: EventPhase,
        original: MountedNodeId,
        current: MountedNodeId,
        related: Option<MountedNodeId>,
        event_related: Option<MountedNodeId>,
        cancelable: bool,
    },
}

#[derive(Debug)]
enum Action {
    Boundary(&'static str),
    Pointer(&'static str),
    Capture(&'static str),
}

#[derive(Clone)]
struct State {
    callbacks: Rc<RefCell<Vec<CallbackObservation>>>,
    updates: Rc<RefCell<Vec<&'static str>>>,
}

struct App;

impl UiApp for App {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        row(children![
            Element::new(CaptureProbe {
                name: "left",
                transfer_on_enter: false,
                callbacks: Rc::clone(&state.callbacks),
            })
            .id("left")
            .key("left"),
            Element::new(CaptureProbe {
                name: "right",
                transfer_on_enter: true,
                callbacks: Rc::clone(&state.callbacks),
            })
            .id("right")
            .key("right"),
        ])
        .id("root")
        .key("root")
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        state.updates.borrow_mut().push(match action {
            Action::Boundary(value) | Action::Pointer(value) | Action::Capture(value) => value,
        });
    }
}

#[derive(Debug)]
struct CaptureProbe {
    name: &'static str,
    transfer_on_enter: bool,
    callbacks: Rc<RefCell<Vec<CallbackObservation>>>,
}

impl Widget<Action> for CaptureProbe {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn event(
        &mut self,
        _state: &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, Action>,
    ) -> WidgetEventOutput {
        match event {
            UiEvent::PointerBoundary(boundary) => {
                self.callbacks
                    .borrow_mut()
                    .push(CallbackObservation::Boundary {
                        widget: self.name,
                        kind: boundary.kind(),
                    });
                context.emit(Action::Boundary(match boundary.kind() {
                    PointerBoundaryKind::Enter => match self.name {
                        "left" => "boundary-left-enter",
                        "right" => "boundary-right-enter",
                        _ => unreachable!("the test has two probes"),
                    },
                    PointerBoundaryKind::Leave => match self.name {
                        "left" => "boundary-left-leave",
                        "right" => "boundary-right-leave",
                        _ => unreachable!("the test has two probes"),
                    },
                    _ => unreachable!("the current boundary kinds are covered"),
                }));
                if self.transfer_on_enter && boundary.kind() == PointerBoundaryKind::Enter {
                    context.capture_pointer();
                }
            }
            UiEvent::Pointer(pointer) if pointer.phase() == PointerPhase::Move => {
                self.callbacks
                    .borrow_mut()
                    .push(CallbackObservation::Pointer {
                        widget: self.name,
                        phase: pointer.phase(),
                        routed_target: context.original_target().clone(),
                        physical_target: context.physical_target().cloned(),
                    });
                context.emit(Action::Pointer(match self.name {
                    "left" => "pointer-left",
                    "right" => "pointer-right",
                    _ => unreachable!("the test has two probes"),
                }));
            }
            UiEvent::PointerCapture(capture) => {
                self.callbacks
                    .borrow_mut()
                    .push(CallbackObservation::Capture {
                        widget: self.name,
                        kind: capture.kind(),
                        phase: context.phase(),
                        original: context.original_target().clone(),
                        current: context.current_target().clone(),
                        related: context.related_target().cloned(),
                        event_related: capture.related_owner().cloned(),
                        cancelable: context.default_is_cancelable(),
                    });
                context.emit(Action::Capture(match (self.name, capture.kind()) {
                    ("left", PointerCaptureKind::Gained) => "capture-left-gained",
                    ("left", PointerCaptureKind::Lost) => "capture-left-lost",
                    ("right", PointerCaptureKind::Gained) => "capture-right-gained",
                    ("right", PointerCaptureKind::Lost) => "capture-right-lost",
                    _ => unreachable!("the current capture kinds and probes are covered"),
                }));
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
    callbacks: Rc<RefCell<Vec<CallbackObservation>>>,
    updates: Rc<RefCell<Vec<&'static str>>>,
}

fn harness() -> Harness {
    let callbacks = Rc::new(RefCell::new(Vec::new()));
    let updates = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = AppRuntime::<App>::mount(State {
        callbacks: Rc::clone(&callbacks),
        updates: Rc::clone(&updates),
    });
    let tokens = StyleTokens::default();
    let size = LogicalSize::try_new(96.0, 48.0)
        .unwrap_or_else(|_| unreachable!("the test surface size is finite"));
    let publication = runtime
        .publish_surface(&SurfaceBuildContext::tight(&tokens, size))
        .unwrap_or_else(|_| unreachable!("the test surface publication is admitted"));
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
        callbacks,
        updates,
    }
}

fn pointer_event(
    context: &SurfaceInputContext,
    point: LogicalPoint,
    phase: PointerPhase,
) -> PointerEvent {
    let pointer_id =
        PointerId::new(51).unwrap_or_else(|| unreachable!("the pointer id is non-zero"));
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
        PointerPhase::Up => event.with_changed_button(PointerButton::Primary),
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
fn capture_transfer_preserves_physical_target_and_loses_before_it_gains() {
    let mut harness = harness();
    submit_and_pump(
        &mut harness.runtime,
        pointer_event(&harness.context, harness.left_point, PointerPhase::Down),
    );
    assert!(harness.callbacks.borrow().iter().any(|observation| {
        matches!(
            observation,
            CallbackObservation::Capture {
                widget: "left",
                kind: PointerCaptureKind::Gained,
                phase: EventPhase::Target,
                original,
                current,
                related: None,
                event_related: None,
                cancelable: false,
            } if original == &harness.left && current == &harness.left
        )
    }));

    harness.callbacks.borrow_mut().clear();
    harness.updates.borrow_mut().clear();
    submit_and_pump(
        &mut harness.runtime,
        pointer_event(&harness.context, harness.right_point, PointerPhase::Move),
    );

    assert_eq!(
        harness.callbacks.borrow().as_slice(),
        [
            CallbackObservation::Boundary {
                widget: "left",
                kind: PointerBoundaryKind::Leave,
            },
            CallbackObservation::Boundary {
                widget: "right",
                kind: PointerBoundaryKind::Enter,
            },
            CallbackObservation::Pointer {
                widget: "left",
                phase: PointerPhase::Move,
                routed_target: harness.left.clone(),
                physical_target: Some(harness.right.clone()),
            },
            CallbackObservation::Capture {
                widget: "left",
                kind: PointerCaptureKind::Lost,
                phase: EventPhase::Target,
                original: harness.left.clone(),
                current: harness.left.clone(),
                related: Some(harness.right.clone()),
                event_related: Some(harness.right.clone()),
                cancelable: false,
            },
            CallbackObservation::Capture {
                widget: "right",
                kind: PointerCaptureKind::Gained,
                phase: EventPhase::Target,
                original: harness.right.clone(),
                current: harness.right.clone(),
                related: Some(harness.left.clone()),
                event_related: Some(harness.left.clone()),
                cancelable: false,
            },
        ]
    );
    assert_eq!(
        harness.updates.borrow().as_slice(),
        [
            "boundary-left-leave",
            "boundary-right-enter",
            "capture-left-lost",
            "capture-right-gained",
            "pointer-left",
        ]
    );
}

#[test]
fn pointer_up_closes_capture_with_one_target_only_lost_notification() {
    let mut harness = harness();
    submit_and_pump(
        &mut harness.runtime,
        pointer_event(&harness.context, harness.left_point, PointerPhase::Down),
    );
    harness.callbacks.borrow_mut().clear();
    submit_and_pump(
        &mut harness.runtime,
        pointer_event(&harness.context, harness.left_point, PointerPhase::Up),
    );

    let capture_callbacks = harness
        .callbacks
        .borrow()
        .iter()
        .filter(|observation| matches!(observation, CallbackObservation::Capture { .. }))
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(
        capture_callbacks,
        [CallbackObservation::Capture {
            widget: "left",
            kind: PointerCaptureKind::Lost,
            phase: EventPhase::Target,
            original: harness.left.clone(),
            current: harness.left.clone(),
            related: None,
            event_related: None,
            cancelable: false,
        }]
    );
}
