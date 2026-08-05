#![allow(refining_impl_trait)]

use std::{cell::RefCell, rc::Rc};

use runenui_core::{
    Element, ElementId, EventContext, LogicalLength, LogicalPoint, NoHostProtocol, PointerButton,
    PointerButtons, PointerCaptureKind, PointerDeviceKind, PointerEvent, PointerId, PointerPhase,
    StyleTokens, SurfaceInputContext, UiApp, UiEvent, View, Widget, WidgetActivation,
    WidgetEventOutput, WidgetMeasure, children, row,
};
use runenui_runtime::{
    AppRuntime, LogicalSize, PumpBudget, SurfaceBuildContext, SurfacePublication,
    TraceDeliveryOutcome, TraceEventFamily, TraceRecord, TraceRecordKind,
    TraceSurfaceSnapshotKind, TraceTarget, WorkSequence,
};

#[derive(Clone)]
struct State {
    visible: bool,
    callbacks: Rc<RefCell<Vec<PointerId>>>,
}

#[derive(Clone, Copy)]
enum Action {
    Hide,
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
                    callbacks: Rc::clone(&state.callbacks),
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
        }
    }
}

#[derive(Debug)]
struct Probe {
    callbacks: Rc<RefCell<Vec<PointerId>>>,
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
            UiEvent::Pointer(pointer) => self.callbacks.borrow_mut().push(pointer.pointer_id()),
            UiEvent::PointerCapture(capture) => {
                self.callbacks.borrow_mut().push(capture.pointer_id());
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
    root: runenui_core::MountedNodeId,
    target: runenui_core::MountedNodeId,
    point: LogicalPoint,
    callbacks: Rc<RefCell<Vec<PointerId>>>,
}

fn harness() -> Harness {
    let callbacks = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = AppRuntime::<App>::mount(State {
        visible: true,
        callbacks: Rc::clone(&callbacks),
    });
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
        callbacks,
    }
}

fn publish(runtime: &mut AppRuntime<App>) -> SurfacePublication {
    let tokens = StyleTokens::default();
    let size = LogicalSize::try_new(64.0, 64.0)
        .unwrap_or_else(|_| unreachable!("the test surface size is finite"));
    runtime.publish_surface(&SurfaceBuildContext::tight(&tokens, size))
}

fn pointer(value: u64) -> PointerId {
    PointerId::new(value).unwrap_or_else(|| unreachable!("the pointer id is non-zero"))
}

fn pointer_event(
    pointer_id: PointerId,
    phase: PointerPhase,
    context: &SurfaceInputContext,
    point: LogicalPoint,
) -> PointerEvent {
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

fn mandatory_record<'a>(
    records: &[&'a TraceRecord],
    sequence: WorkSequence,
    predicate: impl Fn(&TraceRecordKind) -> bool,
) -> &'a TraceRecord {
    records
        .iter()
        .copied()
        .find(|record| record.work_sequence() == Some(sequence) && predicate(record.kind()))
        .unwrap_or_else(|| unreachable!("the mandatory reconciliation fact is retained"))
}

fn assert_requested_surface(record: &TraceRecord, harness: &Harness) {
    let surface = record
        .context()
        .surface()
        .unwrap_or_else(|| unreachable!("reconciliation cleanup owns surface identity"));
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
}

fn assert_physical_path(record: &TraceRecord, harness: &Harness) {
    let path = record
        .context()
        .physical_path()
        .unwrap_or_else(|| unreachable!("cleanup owns the pre-cleanup physical path"));
    assert_eq!(path.targets().len(), 2);
    assert_eq!(path.targets()[0].mounted_node_id(), &harness.root);
    assert_eq!(path.targets()[1].mounted_node_id(), &harness.target);
}

fn assert_captured_cleanup(record: &TraceRecord, harness: &Harness, captured: PointerId) {
    assert!(matches!(
        record.kind(),
        TraceRecordKind::PointerIntegrityCleanupCommitted
    ));
    let context = record.context();
    assert_eq!(context.event(), None);
    let pointer = context
        .pointer()
        .unwrap_or_else(|| unreachable!("cleanup owns pointer-stream identity"));
    assert_eq!(pointer.pointer_id(), &captured);
    assert_eq!(pointer.device_id(), None);
    assert_eq!(pointer.device_kind(), PointerDeviceKind::Mouse);
    assert_eq!(pointer.phase(), None);
    assert_requested_surface(record, harness);
    assert_physical_path(record, harness);
    let cleanup = context
        .pointer_cleanup()
        .unwrap_or_else(|| unreachable!("cleanup owns exact owner transitions"));
    let pressed = cleanup
        .pressed_owner()
        .unwrap_or_else(|| unreachable!("pressed ownership is cleared"));
    assert_eq!(
        pressed.previous().map(TraceTarget::mounted_node_id),
        Some(&harness.target)
    );
    assert_eq!(pressed.current(), None);
    let capture = cleanup
        .capture_owner()
        .unwrap_or_else(|| unreachable!("capture ownership is cleared"));
    assert_eq!(
        capture.previous().map(TraceTarget::mounted_node_id),
        Some(&harness.target)
    );
    assert_eq!(capture.current(), None);
    assert!(cleanup.physical_path_cleared());
}

fn assert_suppressed_capture_loss(record: &TraceRecord, harness: &Harness, captured: PointerId) {
    assert!(matches!(
        record.kind(),
        TraceRecordKind::PointerCaptureNotificationResolved {
            kind: PointerCaptureKind::Lost,
        }
    ));
    assert_eq!(
        record.target().map(TraceTarget::mounted_node_id),
        Some(&harness.target)
    );
    let context = record.context();
    let event = context
        .event()
        .unwrap_or_else(|| unreachable!("capture loss owns event classification"));
    assert_eq!(event.family(), TraceEventFamily::PointerCapture);
    assert!(!event.is_cancelable());
    assert_eq!(context.delivery(), Some(TraceDeliveryOutcome::Suppressed));
    let pointer = context
        .pointer()
        .unwrap_or_else(|| unreachable!("capture loss owns pointer-stream identity"));
    assert_eq!(pointer.pointer_id(), &captured);
    assert_eq!(pointer.device_id(), None);
    assert_eq!(pointer.device_kind(), PointerDeviceKind::Mouse);
    assert_eq!(pointer.phase(), None);
    assert_requested_surface(record, harness);
    let route = context
        .route()
        .unwrap_or_else(|| unreachable!("capture loss owns its target-only route"));
    assert_eq!(route.targets().len(), 1);
    assert_eq!(route.targets()[0].mounted_node_id(), &harness.target);
    assert_eq!(route.related_target(), None);
    let path = context
        .physical_path()
        .unwrap_or_else(|| unreachable!("capture loss owns the post-clear physical path"));
    assert!(path.targets().is_empty());
    let transition = context
        .target_transition()
        .unwrap_or_else(|| unreachable!("capture loss owns exact capture endpoints"));
    assert_eq!(
        transition.previous().map(TraceTarget::mounted_node_id),
        Some(&harness.target)
    );
    assert_eq!(transition.current(), None);
}

fn assert_hovered_cleanup(record: &TraceRecord, harness: &Harness, hovered: PointerId) {
    assert!(matches!(
        record.kind(),
        TraceRecordKind::PointerIntegrityCleanupCommitted
    ));
    let context = record.context();
    let pointer = context
        .pointer()
        .unwrap_or_else(|| unreachable!("cleanup owns pointer-stream identity"));
    assert_eq!(pointer.pointer_id(), &hovered);
    assert_eq!(pointer.device_id(), None);
    assert_eq!(pointer.device_kind(), PointerDeviceKind::Mouse);
    assert_eq!(pointer.phase(), None);
    assert_requested_surface(record, harness);
    assert_physical_path(record, harness);
    let cleanup = context
        .pointer_cleanup()
        .unwrap_or_else(|| unreachable!("cleanup owns exact path outcome"));
    assert_eq!(cleanup.pressed_owner(), None);
    assert_eq!(cleanup.capture_owner(), None);
    assert!(cleanup.physical_path_cleared());
}

#[test]
fn removal_cleans_streams_in_registration_order_and_suppresses_removed_capture_loss() {
    let mut harness = harness();
    let captured = pointer(9);
    let hovered = pointer(2);
    submit_and_pump(
        &mut harness.runtime,
        pointer_event(
            captured,
            PointerPhase::Down,
            &harness.context,
            harness.point,
        ),
    );
    submit_and_pump(
        &mut harness.runtime,
        pointer_event(hovered, PointerPhase::Move, &harness.context, harness.point),
    );
    harness.callbacks.borrow_mut().clear();
    let trace_start = harness.runtime.trace().len();

    let submission = harness
        .runtime
        .submit_action(Action::Hide)
        .unwrap_or_else(|_| unreachable!("the hide action is accepted"));
    let sequence = submission.sequence();
    pump_all(&mut harness.runtime);

    assert!(harness.callbacks.borrow().is_empty());
    let records = harness
        .runtime
        .trace()
        .records()
        .skip(trace_start)
        .collect::<Vec<_>>();
    let captured_cleanup = mandatory_record(&records, sequence, |kind| {
        matches!(kind, TraceRecordKind::PointerIntegrityCleanupCommitted)
    });
    let suppressed_loss = mandatory_record(&records, sequence, |kind| {
        matches!(
            kind,
            TraceRecordKind::PointerCaptureNotificationResolved {
                kind: PointerCaptureKind::Lost,
            }
        )
    });
    let hovered_cleanup = records
        .iter()
        .copied()
        .filter(|record| {
            record.work_sequence() == Some(sequence)
                && matches!(
                    record.kind(),
                    TraceRecordKind::PointerIntegrityCleanupCommitted
                )
        })
        .nth(1)
        .unwrap_or_else(|| unreachable!("the hovered stream cleanup is retained"));

    assert_captured_cleanup(captured_cleanup, &harness, captured);
    assert_suppressed_capture_loss(suppressed_loss, &harness, captured);
    assert_hovered_cleanup(hovered_cleanup, &harness, hovered);
    assert_eq!(captured_cleanup.instant(), suppressed_loss.instant());
    assert_eq!(captured_cleanup.instant(), hovered_cleanup.instant());
    assert!(captured_cleanup.instant().is_some());
    assert_eq!(
        suppressed_loss.causal_parent(),
        Some(captured_cleanup.sequence())
    );
    assert_eq!(
        hovered_cleanup.causal_parent(),
        Some(suppressed_loss.sequence())
    );
    assert_eq!(
        records
            .iter()
            .filter(|record| {
                matches!(
                    record.kind(),
                    TraceRecordKind::PointerCaptureNotificationResolved { .. }
                )
            })
            .count(),
        1
    );

    submit_and_pump(
        &mut harness.runtime,
        pointer_event(
            captured,
            PointerPhase::Cancel,
            &harness.context,
            harness.point,
        ),
    );
    submit_and_pump(
        &mut harness.runtime,
        pointer_event(
            hovered,
            PointerPhase::Cancel,
            &harness.context,
            harness.point,
        ),
    );
    assert!(harness.callbacks.borrow().is_empty());
    assert_eq!(
        harness
            .runtime
            .trace()
            .kinds()
            .filter(|kind| matches!(kind, TraceRecordKind::PointerStreamClosed { .. }))
            .count(),
        2
    );
}
