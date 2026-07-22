#![allow(refining_impl_trait)]

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use runenui_core::{
    Element, EventContext, LogicalLength, LogicalPoint, NoHostProtocol, PointerButton,
    PointerButtons, PointerCaptureKind, PointerDeviceKind, PointerEvent, PointerId, PointerPhase,
    StyleTokens, SurfaceInputContext, UiApp, UiEvent, View, Widget, WidgetActivation,
    WidgetActivationContext, WidgetActivationOutput, WidgetEventOutput, WidgetMeasure,
};
use runenui_runtime::{
    AppRuntime, LogicalSize, PumpBudget, SurfaceBuildContext, TracePointerRejection,
    TraceRecordKind,
};

#[derive(Clone)]
struct State {
    callbacks: Rc<RefCell<Vec<PointerPhase>>>,
    activations: Rc<Cell<usize>>,
}

#[derive(Clone, Copy)]
enum Action {
    Activated,
}

struct App;

impl UiApp for App {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        Element::new(Probe {
            callbacks: Rc::clone(&state.callbacks),
        })
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            Action::Activated => state.activations.set(state.activations.get() + 1),
        }
    }
}

#[derive(Debug)]
struct Probe {
    callbacks: Rc<RefCell<Vec<PointerPhase>>>,
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
        if let UiEvent::Pointer(pointer) = event {
            self.callbacks.borrow_mut().push(pointer.phase());
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

fn pointer_event(
    phase: PointerPhase,
    context: &SurfaceInputContext,
    point: LogicalPoint,
) -> PointerEvent {
    let pointer_id =
        PointerId::new(79).unwrap_or_else(|| unreachable!("the pointer id is non-zero"));
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

fn pump_all(runtime: &mut AppRuntime<App>) {
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

#[test]
fn missing_context_up_commits_integrity_only_cleanup_with_causal_trace() {
    let callbacks = Rc::new(RefCell::new(Vec::new()));
    let activations = Rc::new(Cell::new(0));
    let mut runtime = AppRuntime::<App>::mount(State {
        callbacks: Rc::clone(&callbacks),
        activations: Rc::clone(&activations),
    });
    let tokens = StyleTokens::default();
    let size = LogicalSize::try_new(64.0, 64.0)
        .unwrap_or_else(|_| unreachable!("the test surface size is finite"));
    let publication = runtime.publish_surface(&SurfaceBuildContext::tight(&tokens, size));
    let bounds = publication
        .frame()
        .nodes()
        .first()
        .unwrap_or_else(|| unreachable!("the root is published"))
        .bounds();
    let point = LogicalPoint::new(bounds.x() + 1.0, bounds.y() + 1.0)
        .unwrap_or_else(|_| unreachable!("published bounds are finite"));
    let current = publication.input_context().clone();

    runtime
        .submit_pointer(pointer_event(PointerPhase::Down, &current, point))
        .unwrap_or_else(|_| unreachable!("the down event is accepted"));
    pump_all(&mut runtime);
    callbacks.borrow_mut().clear();
    activations.set(0);

    let missing = runtime.__surface_context_for_test(
        0,
        1,
        current.coordinate_revision(),
        current.hit_test_generation() + 100,
    );
    runtime
        .submit_pointer(pointer_event(PointerPhase::Up, &missing, point))
        .unwrap_or_else(|_| unreachable!("the up event is accepted before processing"));
    pump_all(&mut runtime);

    assert!(callbacks.borrow().is_empty());
    assert_eq!(activations.get(), 0);
    let records = runtime.trace().records().collect::<Vec<_>>();
    let rejected = records
        .iter()
        .rev()
        .find(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::PointerIngressRejected {
                    pointer_id,
                    phase: PointerPhase::Up,
                    outcome: TracePointerRejection::MissingGeneration,
                } if pointer_id.get() == 79
            )
        })
        .unwrap_or_else(|| unreachable!("missing-generation up is diagnosed"));
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
                } if pointer_id.get() == 79
            )
        })
        .unwrap_or_else(|| unreachable!("missing-generation up commits exact cleanup"));
    let suppressed = records
        .iter()
        .rev()
        .find(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::PointerCaptureNotificationSuppressed {
                    pointer_id,
                    kind: PointerCaptureKind::Lost,
                } if pointer_id.get() == 79
            )
        })
        .unwrap_or_else(|| unreachable!("unavailable capture loss is diagnosed"));
    let closed = records
        .iter()
        .rev()
        .find(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::PointerStreamClosed { pointer_id } if pointer_id.get() == 79
            )
        })
        .unwrap_or_else(|| unreachable!("missing-generation up closes the stream"));

    assert_eq!(cleanup.causal_parent(), Some(rejected.sequence()));
    assert_eq!(suppressed.causal_parent(), Some(cleanup.sequence()));
    assert_eq!(closed.causal_parent(), Some(suppressed.sequence()));
}
