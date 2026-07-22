#![allow(refining_impl_trait)]

use std::{cell::RefCell, rc::Rc};

use runenui_core::{
    Element, ElementId, EventContext, LogicalLength, LogicalPoint, NoHostProtocol, PointerButton,
    PointerButtons, PointerCaptureKind, PointerDeviceKind, PointerEvent, PointerId, PointerPhase,
    StyleTokens, SurfaceInputContext, UiApp, UiEvent, View, Widget, WidgetActivation,
    WidgetEventOutput, WidgetMeasure,
};
use runenui_runtime::{
    AppRuntime, LogicalSize, PumpBudget, SurfaceBuildContext, SurfacePublication, TraceRecordKind,
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
}

struct Harness {
    runtime: AppRuntime<App>,
    context: SurfaceInputContext,
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
                TraceRecordKind::PointerIntegrityCleanupCommitted { .. }
                    | TraceRecordKind::PointerCaptureNotificationSuppressed { .. }
                    | TraceRecordKind::PointerStreamClosed { .. }
                    | TraceRecordKind::RuntimeShutdown { .. }
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(records.len(), 6);
    assert!(matches!(
        records[0].kind(),
        TraceRecordKind::PointerIntegrityCleanupCommitted {
            pointer_id,
            pressed: true,
            capture: true,
            physical_path: true,
        } if pointer_id == &captured
    ));
    assert!(matches!(
        records[1].kind(),
        TraceRecordKind::PointerCaptureNotificationSuppressed {
            pointer_id,
            kind: PointerCaptureKind::Lost,
        } if pointer_id == &captured
    ));
    assert!(matches!(
        records[2].kind(),
        TraceRecordKind::PointerStreamClosed { pointer_id } if pointer_id == &captured
    ));
    assert!(matches!(
        records[3].kind(),
        TraceRecordKind::PointerIntegrityCleanupCommitted {
            pointer_id,
            pressed: false,
            capture: false,
            physical_path: true,
        } if pointer_id == &hovered
    ));
    assert!(matches!(
        records[4].kind(),
        TraceRecordKind::PointerStreamClosed { pointer_id } if pointer_id == &hovered
    ));
    assert!(matches!(
        records[5].kind(),
        TraceRecordKind::RuntimeShutdown { .. }
    ));
    for pair in records.windows(2) {
        assert_eq!(pair[1].causal_parent(), Some(pair[0].sequence()));
    }

    let trace_len = harness.runtime.trace().len();
    assert!(harness.runtime.shutdown().already_complete());
    assert_eq!(harness.runtime.trace().len(), trace_len);
}
