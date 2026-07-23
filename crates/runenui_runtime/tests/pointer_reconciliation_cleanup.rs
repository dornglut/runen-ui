#![allow(refining_impl_trait)]

use std::{cell::RefCell, rc::Rc};

use runenui_core::{
    Element, ElementId, EventContext, LogicalLength, LogicalPoint, NoHostProtocol, PointerButton,
    PointerButtons, PointerCaptureKind, PointerDeviceKind, PointerEvent, PointerId, PointerPhase,
    StyleTokens, SurfaceInputContext, UiApp, UiEvent, View, Widget, WidgetActivation,
    WidgetEventOutput, WidgetMeasure, children, row,
};
use runenui_runtime::{
    AppRuntime, LogicalSize, PumpBudget, SurfaceBuildContext, SurfacePublication, TraceRecordKind,
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

    harness
        .runtime
        .submit_action(Action::Hide)
        .unwrap_or_else(|_| unreachable!("the hide action is accepted"));
    pump_all(&mut harness.runtime);

    assert!(harness.callbacks.borrow().is_empty());
    let lifecycle = harness
        .runtime
        .trace()
        .records()
        .skip(trace_start)
        .filter(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::PointerIntegrityCleanupCommitted { .. }
                    | TraceRecordKind::PointerCaptureNotificationSuppressed { .. }
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(lifecycle.len(), 3);
    assert!(matches!(
        lifecycle[0].kind(),
        TraceRecordKind::PointerIntegrityCleanupCommitted {
            pointer_id,
            pressed: true,
            capture: true,
            physical_path: true,
        } if pointer_id == &captured
    ));
    assert!(matches!(
        lifecycle[1].kind(),
        TraceRecordKind::PointerCaptureNotificationSuppressed {
            pointer_id,
            kind: PointerCaptureKind::Lost,
        } if pointer_id == &captured
    ));
    assert!(matches!(
        lifecycle[2].kind(),
        TraceRecordKind::PointerIntegrityCleanupCommitted {
            pointer_id,
            pressed: false,
            capture: false,
            physical_path: true,
        } if pointer_id == &hovered
    ));
    assert_eq!(lifecycle[1].causal_parent(), Some(lifecycle[0].sequence()));
    assert_eq!(lifecycle[2].causal_parent(), Some(lifecycle[1].sequence()));

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
