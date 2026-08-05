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
    AppRuntime, LogicalSize, MountedNodeId, PumpBudget, SurfaceBuildContext, TraceEventFamily,
    TracePointerCaptureRequestKind, TracePointerCaptureRequestRejection, TraceRecord,
    TraceRecordKind, TraceSurfaceSnapshotKind, TraceTarget, WorkSequence,
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
    release_after_transfer: bool,
    capture_root_on_down: bool,
}

struct App;

impl UiApp for App {
    type State = State;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        container(
            InvalidReleaseRoot {
                capture_on_down: state.capture_root_on_down,
                captures: Rc::clone(&state.captures),
            },
            children![
                Element::new(CaptureProbe {
                    name: "left",
                    transfer_on_enter: false,
                    release_after_transfer: false,
                    captures: Rc::clone(&state.captures),
                })
                .id("left")
                .key("left"),
                Element::new(CaptureProbe {
                    name: "right",
                    transfer_on_enter: true,
                    release_after_transfer: state.release_after_transfer,
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
struct InvalidReleaseRoot {
    capture_on_down: bool,
    captures: Rc<RefCell<Vec<CaptureObservation>>>,
}

impl Widget<()> for InvalidReleaseRoot {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn event(
        &mut self,
        _state: &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, ()>,
    ) -> WidgetEventOutput {
        match event {
            UiEvent::Pointer(pointer) if pointer.phase() == PointerPhase::Move => {
                context.release_pointer_capture();
            }
            UiEvent::Pointer(pointer)
                if self.capture_on_down && pointer.phase() == PointerPhase::Down =>
            {
                context.capture_pointer();
            }
            UiEvent::PointerCapture(capture) => {
                self.captures.borrow_mut().push(CaptureObservation {
                    widget: "root",
                    kind: capture.kind(),
                    related: capture.related_owner().cloned(),
                });
            }
            _ => {}
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
    release_after_transfer: bool,
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
                if self.release_after_transfer {
                    context.release_pointer_capture();
                }
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
    root: MountedNodeId,
    left: MountedNodeId,
    right: MountedNodeId,
    left_point: LogicalPoint,
    right_point: LogicalPoint,
    captures: Rc<RefCell<Vec<CaptureObservation>>>,
}

fn harness(release_after_transfer: bool, capture_root_on_down: bool) -> Harness {
    let captures = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = AppRuntime::<App>::mount(State {
        captures: Rc::clone(&captures),
        release_after_transfer,
        capture_root_on_down,
    });
    let tokens = StyleTokens::default();
    let size = LogicalSize::try_new(96.0, 48.0)
        .unwrap_or_else(|_| unreachable!("the test surface size is finite"));
    let publication = runtime.publish_surface(&SurfaceBuildContext::tight(&tokens, size));
    let root_authored =
        ElementId::new("root").unwrap_or_else(|_| unreachable!("the test id is valid"));
    let left_authored =
        ElementId::new("left").unwrap_or_else(|_| unreachable!("the test id is valid"));
    let right_authored =
        ElementId::new("right").unwrap_or_else(|_| unreachable!("the test id is valid"));
    let root_node = publication
        .frame()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&root_authored))
        .unwrap_or_else(|| unreachable!("the root node is published"));
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
        root: root_node.id().clone(),
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

fn submit_and_pump(runtime: &mut AppRuntime<App>, event: PointerEvent) -> WorkSequence {
    let submission = runtime
        .submit_pointer(event)
        .unwrap_or_else(|_| unreachable!("the pointer event is accepted"));
    let sequence = submission.sequence();
    let report = runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
    assert!(report.is_quiescent());
    sequence
}

fn assert_release_not_owner_rejection(
    record: &TraceRecord,
    sequence: WorkSequence,
    harness: &Harness,
) {
    assert!(matches!(
        record.kind(),
        TraceRecordKind::PointerCaptureRequestRejected {
            request: TracePointerCaptureRequestKind::Release,
            outcome: TracePointerCaptureRequestRejection::ReleaseNotOwner,
        }
    ));
    assert_eq!(record.work_sequence(), Some(sequence));
    assert!(record.instant().is_some());
    assert_eq!(
        record.target().map(TraceTarget::mounted_node_id),
        Some(&harness.root)
    );

    let context = record.context();
    let event = context
        .event()
        .unwrap_or_else(|| unreachable!("rejection owns capture event classification"));
    assert_eq!(event.family(), TraceEventFamily::PointerCapture);
    assert!(!event.is_cancelable());
    let pointer = context
        .pointer()
        .unwrap_or_else(|| unreachable!("rejection owns submitted pointer identity"));
    assert_eq!(pointer.pointer_id().get(), 61);
    assert_eq!(pointer.device_id(), None);
    assert_eq!(pointer.device_kind(), PointerDeviceKind::Mouse);
    assert_eq!(pointer.phase(), Some(PointerPhase::Move));
    let surface = context
        .surface()
        .unwrap_or_else(|| unreachable!("rejection owns accepted surface identity"));
    assert_eq!(surface.surface_id(), harness.context.surface_id());
    assert_eq!(
        surface.coordinate_revision(),
        harness.context.coordinate_revision()
    );
    assert_eq!(
        surface.hit_test_generation(),
        harness.context.hit_test_generation()
    );
    assert_eq!(surface.snapshot(), Some(TraceSurfaceSnapshotKind::Current));
    let path = context
        .physical_path()
        .unwrap_or_else(|| unreachable!("rejection owns the exact physical path"));
    assert_eq!(path.targets().len(), 2);
    assert_eq!(path.targets()[0].mounted_node_id(), &harness.root);
    assert_eq!(path.targets()[1].mounted_node_id(), &harness.right);
    let transition = context
        .target_transition()
        .unwrap_or_else(|| unreachable!("rejection owns prior and requested capture endpoints"));
    assert_eq!(
        transition.previous().map(TraceTarget::mounted_node_id),
        Some(&harness.right)
    );
    assert_eq!(transition.current(), None);
    assert_eq!(context.route(), None);
    assert_eq!(context.delivery(), None);
}

#[test]
fn invalid_later_releases_do_not_erase_an_earlier_valid_transfer() {
    let mut harness = harness(false, false);
    submit_and_pump(
        &mut harness.runtime,
        pointer_event(&harness.context, harness.left_point, PointerPhase::Down),
    );
    harness.captures.borrow_mut().clear();

    let sequence = submit_and_pump(
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
    let rejections = harness
        .runtime
        .trace()
        .records()
        .filter(|record| {
            record.work_sequence() == Some(sequence)
                && matches!(
                    record.kind(),
                    TraceRecordKind::PointerCaptureRequestRejected { .. }
                )
        })
        .collect::<Vec<_>>();
    assert_eq!(rejections.len(), 2);
    assert_release_not_owner_rejection(rejections[0], sequence, &harness);
    assert_release_not_owner_rejection(rejections[1], sequence, &harness);
    assert_eq!(rejections[0].instant(), rejections[1].instant());
    assert_eq!(
        rejections[1].causal_parent(),
        Some(rejections[0].sequence())
    );
}

#[test]
fn one_callback_preserves_capture_then_release_in_staging_order() {
    let mut harness = harness(true, false);
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
        [CaptureObservation {
            widget: "left",
            kind: PointerCaptureKind::Lost,
            related: None,
        }]
    );
}

#[test]
fn down_default_is_the_final_capture_request_after_explicit_staging() {
    let mut harness = harness(false, true);

    submit_and_pump(
        &mut harness.runtime,
        pointer_event(&harness.context, harness.left_point, PointerPhase::Down),
    );

    assert_eq!(
        harness.captures.borrow().as_slice(),
        [CaptureObservation {
            widget: "left",
            kind: PointerCaptureKind::Gained,
            related: None,
        }]
    );
    assert!(
        !harness
            .runtime
            .trace()
            .kinds()
            .any(|kind| matches!(kind, TraceRecordKind::PointerCaptureRequestRejected { .. }))
    );
}
