#![allow(refining_impl_trait)]

use std::{cell::RefCell, rc::Rc};

use runenui_core::{
    Element, ElementId, EventContext, HitContribution, HitContributionContext, LogicalLength,
    LogicalPoint, LogicalRect, NoHostProtocol, PointerButton, PointerButtons, PointerCaptureKind,
    PointerDeviceKind, PointerEvent, PointerId, PointerPhase, StyleEnvironment, SurfaceInputContext,
    UiApp, UiEvent, View, Widget, WidgetActivation, WidgetEventOutput, WidgetMeasure,
};
use runenui_runtime::{
    AppRuntime, FocusEventKind, FocusReason, LogicalSize, PumpBudget, SurfaceBuildContext,
    SurfacePublication, TraceDeliveryOutcome, TraceEventFamily, TraceFocusRecordRole, TraceRecord,
    TraceRecordKind, TraceTarget,
};

#[derive(Clone)]
struct State {
    callbacks: Rc<RefCell<Vec<PointerId>>>,
}

struct App;

impl UiApp for App {
    type State = State;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        Element::new(Probe {
            callbacks: Rc::clone(&state.callbacks),
        })
        .id("target")
        .key("target")
    }

    fn update(_state: &mut Self::State, _action: Self::Action) {}
}

#[derive(Debug)]
struct Probe {
    callbacks: Rc<RefCell<Vec<PointerId>>>,
}

impl Widget<()> for Probe {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn event(
        &mut self,
        _state: &mut Self::State,
        event: &UiEvent,
        _context: &mut EventContext<'_, ()>,
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
    target: runenui_core::MountedNodeId,
    point: LogicalPoint,
    callbacks: Rc<RefCell<Vec<PointerId>>>,
}

fn harness() -> Harness {
    let callbacks = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = AppRuntime::<App>::mount(State {
        callbacks: Rc::clone(&callbacks),
    });
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
        callbacks,
    }
}

fn publish(runtime: &mut AppRuntime<App>) -> SurfacePublication {
    let style_environment = StyleEnvironment::default();
    let size = LogicalSize::try_new(64.0, 64.0)
        .unwrap_or_else(|_| unreachable!("the test surface size is finite"));
    runtime
        .publish_surface(&SurfaceBuildContext::tight(&style_environment, size))
        .unwrap_or_else(|_| unreachable!("the test surface publication is admitted"))
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

fn assert_focus_shutdown_chain(records: &[&TraceRecord], harness: &Harness) {
    let transition = records[5];
    assert!(matches!(
        transition.kind(),
        TraceRecordKind::FocusTransitionCommitted {
            reason: FocusReason::Shutdown,
        }
    ));
    assert_eq!(
        transition.context().focus_record_role(),
        Some(TraceFocusRecordRole::Transition)
    );
    let transition_endpoints = transition
        .context()
        .target_transition()
        .unwrap_or_else(|| unreachable!("shutdown focus transition owns exact endpoints"));
    assert_eq!(
        transition_endpoints
            .previous()
            .map(TraceTarget::mounted_node_id),
        Some(&harness.target)
    );
    assert_eq!(transition_endpoints.current(), None);

    assert!(matches!(
        records[6].kind(),
        TraceRecordKind::FocusWithinInvalidated { entered: 0, .. }
    ));

    let notification = records[7];
    assert!(matches!(
        notification.kind(),
        TraceRecordKind::FocusNotificationResolved {
            kind: FocusEventKind::Out,
        }
    ));
    let context = notification.context();
    assert_eq!(
        context.focus_record_role(),
        Some(TraceFocusRecordRole::Notification)
    );
    let event = context
        .event()
        .unwrap_or_else(|| unreachable!("shutdown focus resolution owns event classification"));
    assert_eq!(event.family(), TraceEventFamily::Focus);
    assert!(!event.is_cancelable());
    assert_eq!(context.delivery(), Some(TraceDeliveryOutcome::Suppressed));
    let route = context
        .route()
        .unwrap_or_else(|| unreachable!("shutdown focus resolution retains the old route"));
    assert_eq!(route.targets().len(), 1);
    assert_eq!(route.targets()[0].mounted_node_id(), &harness.target);
    assert_eq!(route.related_target(), None);
    let notification_endpoints = context
        .target_transition()
        .unwrap_or_else(|| unreachable!("shutdown focus resolution owns exact endpoints"));
    assert_eq!(
        notification_endpoints
            .previous()
            .map(TraceTarget::mounted_node_id),
        Some(&harness.target)
    );
    assert_eq!(notification_endpoints.current(), None);

    assert!(matches!(
        records[8].kind(),
        TraceRecordKind::RuntimeShutdown { .. }
    ));
    assert!(transition.instant().is_some());
    assert_eq!(transition.instant(), notification.instant());
    assert_eq!(transition.instant(), records[8].instant());
}

fn submit_and_pump(runtime: &mut AppRuntime<App>, event: PointerEvent) {
    runtime
        .submit_pointer(event)
        .unwrap_or_else(|_| unreachable!("the pointer event is accepted"));
    pump_all(runtime);
}

fn assert_surface_is_requested(record: &TraceRecord, harness: &Harness) {
    let surface = record
        .context()
        .surface()
        .unwrap_or_else(|| unreachable!("shutdown cleanup owns requested surface identity"));
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

fn assert_captured_cleanup(record: &TraceRecord, harness: &Harness, captured: PointerId) {
    assert!(matches!(
        record.kind(),
        TraceRecordKind::PointerIntegrityCleanupCommitted
    ));
    assert_eq!(record.work_sequence(), None);
    let context = record.context();
    assert_eq!(context.event(), None);
    let pointer = context
        .pointer()
        .unwrap_or_else(|| unreachable!("shutdown cleanup owns pointer-stream identity"));
    assert_eq!(pointer.pointer_id(), &captured);
    assert_eq!(pointer.device_id(), None);
    assert_eq!(pointer.device_kind(), PointerDeviceKind::Mouse);
    assert_eq!(pointer.phase(), None);
    assert_surface_is_requested(record, harness);
    let path = context
        .physical_path()
        .unwrap_or_else(|| unreachable!("shutdown cleanup owns the original physical path"));
    assert_eq!(path.targets().len(), 1);
    assert_eq!(path.targets()[0].mounted_node_id(), &harness.target);
    let cleanup = context
        .pointer_cleanup()
        .unwrap_or_else(|| unreachable!("shutdown cleanup owns exact cleanup facts"));
    let pressed = cleanup
        .pressed_owner()
        .unwrap_or_else(|| unreachable!("pressed authority is cleared"));
    assert_eq!(
        pressed.previous().map(TraceTarget::mounted_node_id),
        Some(&harness.target)
    );
    assert_eq!(pressed.current(), None);
    let capture = cleanup
        .capture_owner()
        .unwrap_or_else(|| unreachable!("capture authority is cleared"));
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
    assert_eq!(record.work_sequence(), None);
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
    assert_surface_is_requested(record, harness);
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
        .unwrap_or_else(|| unreachable!("shutdown cleanup owns pointer-stream identity"));
    assert_eq!(pointer.pointer_id(), &hovered);
    assert_eq!(pointer.phase(), None);
    assert_surface_is_requested(record, harness);
    let path = context
        .physical_path()
        .unwrap_or_else(|| unreachable!("shutdown cleanup owns the original physical path"));
    assert_eq!(path.targets().len(), 1);
    assert_eq!(path.targets()[0].mounted_node_id(), &harness.target);
    let cleanup = context
        .pointer_cleanup()
        .unwrap_or_else(|| unreachable!("shutdown cleanup owns exact cleanup facts"));
    assert_eq!(cleanup.pressed_owner(), None);
    assert_eq!(cleanup.capture_owner(), None);
    assert!(cleanup.physical_path_cleared());
}

#[test]
fn shutdown_drains_pointer_streams_in_registration_order_without_callbacks() {
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

    let report = harness.runtime.shutdown();

    assert!(!report.already_complete());
    assert!(harness.callbacks.borrow().is_empty());
    let records = harness
        .runtime
        .trace()
        .records()
        .skip(trace_start)
        .filter(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::PointerIntegrityCleanupCommitted
                    | TraceRecordKind::PointerCaptureNotificationResolved {
                        kind: PointerCaptureKind::Lost,
                    }
                    | TraceRecordKind::PointerStreamClosed { .. }
                    | TraceRecordKind::FocusTransitionCommitted { .. }
                    | TraceRecordKind::FocusWithinInvalidated { .. }
                    | TraceRecordKind::FocusNotificationResolved {
                        kind: FocusEventKind::Out,
                    }
                    | TraceRecordKind::RuntimeShutdown { .. }
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(records.len(), 9);
    assert_captured_cleanup(records[0], &harness, captured);
    assert_suppressed_capture_loss(records[1], &harness, captured);
    assert_eq!(records[0].instant(), records[1].instant());
    assert!(records[0].instant().is_some());
    assert!(matches!(
        records[2].kind(),
        TraceRecordKind::PointerStreamClosed { pointer_id } if pointer_id == &captured
    ));
    assert_hovered_cleanup(records[3], &harness, hovered);
    assert_eq!(records[0].instant(), records[3].instant());
    assert!(matches!(
        records[4].kind(),
        TraceRecordKind::PointerStreamClosed { pointer_id } if pointer_id == &hovered
    ));
    assert_focus_shutdown_chain(&records, &harness);
    for pair in records.windows(2) {
        assert_eq!(pair[1].causal_parent(), Some(pair[0].sequence()));
    }

    let trace_len = harness.runtime.trace().len();
    assert!(harness.runtime.shutdown().already_complete());
    assert_eq!(harness.runtime.trace().len(), trace_len);
}
