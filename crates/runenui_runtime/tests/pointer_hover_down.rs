#![allow(refining_impl_trait)]

use std::{
    cell::RefCell,
    rc::Rc,
};

use runenui_core::{
    Element, EventContext, LogicalLength, LogicalPoint, NoHostProtocol, PointerButton,
    PointerButtons, PointerDeviceKind, PointerEvent, PointerId, PointerPhase, StyleTokens, UiApp,
    UiEvent, View, Widget, WidgetEventOutput, WidgetMeasure,
};
use runenui_runtime::{AppRuntime, LogicalSize, PumpBudget, SurfaceBuildContext};

#[derive(Clone)]
struct State {
    phases: Rc<RefCell<Vec<PointerPhase>>>,
}

#[derive(Debug)]
enum Action {
    Observed(PointerPhase),
}

struct App;

impl UiApp for App {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(_state: &Self::State) -> impl View<Self::Action> {
        Element::new(ProbeWidget)
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            Action::Observed(phase) => state.phases.borrow_mut().push(phase),
        }
    }
}

#[derive(Debug)]
struct ProbeWidget;

impl Widget<Action> for ProbeWidget {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn event(
        &mut self,
        _state: &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, Action>,
    ) -> WidgetEventOutput {
        if let UiEvent::Pointer(pointer) = event {
            context.emit(Action::Observed(pointer.phase()));
        }
        WidgetEventOutput::none()
    }

    fn measure(&self, _state: &Self::State) -> WidgetMeasure {
        WidgetMeasure::Fixed {
            width: LogicalLength::new(32.0).unwrap_or_default(),
            height: LogicalLength::new(32.0).unwrap_or_default(),
        }
    }
}

fn pump_all(runtime: &mut AppRuntime<App>) {
    let report = runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
    assert!(report.is_quiescent());
}

#[test]
fn hover_stream_accepts_first_button_down_and_rejects_repeated_button_down() {
    let phases = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = AppRuntime::<App>::mount(State {
        phases: Rc::clone(&phases),
    });
    let tokens = StyleTokens::default();
    let size = LogicalSize::try_new(64.0, 64.0)
        .unwrap_or_else(|_| unreachable!("the test surface size is finite"));
    let publication = runtime.publish_surface(&SurfaceBuildContext::tight(&tokens, size));
    let node = publication
        .frame()
        .nodes()
        .first()
        .unwrap_or_else(|| unreachable!("the root is published"));
    let bounds = node.bounds();
    let point = LogicalPoint::new(bounds.x() + 1.0, bounds.y() + 1.0)
        .unwrap_or_else(|_| unreachable!("published bounds are finite"));
    let context = publication.input_context().clone();
    let pointer_id =
        PointerId::new(1).unwrap_or_else(|| unreachable!("pointer identity is non-zero"));

    runtime
        .submit_pointer(PointerEvent::new(
            pointer_id,
            PointerDeviceKind::Mouse,
            PointerPhase::Move,
            point,
            context.clone(),
        ))
        .unwrap_or_else(|_| unreachable!("hover ingress is accepted"));
    pump_all(&mut runtime);

    let down = PointerEvent::new(
        pointer_id,
        PointerDeviceKind::Mouse,
        PointerPhase::Down,
        point,
        context,
    )
    .with_buttons(PointerButtons::new([PointerButton::Primary]))
    .with_changed_button(PointerButton::Primary);

    runtime
        .submit_pointer(down.clone())
        .unwrap_or_else(|_| unreachable!("the first button-down is accepted"));
    pump_all(&mut runtime);

    assert_eq!(
        phases.borrow().as_slice(),
        [PointerPhase::Move, PointerPhase::Down]
    );

    runtime
        .submit_pointer(down)
        .unwrap_or_else(|_| unreachable!("processing owns duplicate-down rejection"));
    pump_all(&mut runtime);

    assert_eq!(
        phases.borrow().as_slice(),
        [PointerPhase::Move, PointerPhase::Down]
    );
}
