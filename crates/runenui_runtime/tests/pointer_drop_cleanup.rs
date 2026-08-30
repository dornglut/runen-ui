#![allow(refining_impl_trait)]

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use runenui_core::{
    Element, ElementId, EventContext, LogicalLength, LogicalPoint, NoHostProtocol, PointerButton,
    PointerButtons, PointerDeviceKind, PointerEvent, PointerId, PointerPhase, StyleEnvironment,
    UiApp, UiEvent, View, Widget, WidgetActivation, WidgetEventOutput, WidgetMeasure,
    WidgetUnmountContext,
};
use runenui_runtime::{AppRuntime, LogicalSize, PumpBudget, SurfaceBuildContext};

#[derive(Clone)]
struct State {
    callbacks: Rc<RefCell<Vec<PointerPhase>>>,
    unmounts: Rc<Cell<usize>>,
}

struct App;

impl UiApp for App {
    type State = State;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        Element::new(Probe {
            callbacks: Rc::clone(&state.callbacks),
            unmounts: Rc::clone(&state.unmounts),
        })
        .id("target")
        .key("target")
    }

    fn update(_state: &mut Self::State, _action: Self::Action) {}
}

#[derive(Debug)]
struct Probe {
    callbacks: Rc<RefCell<Vec<PointerPhase>>>,
    unmounts: Rc<Cell<usize>>,
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
        if let UiEvent::Pointer(pointer) = event {
            self.callbacks.borrow_mut().push(pointer.phase());
        }
        WidgetEventOutput::none()
    }

    fn unmount(&self, _state: &mut Self::State, _context: &mut WidgetUnmountContext) {
        self.unmounts.set(self.unmounts.get() + 1);
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

#[test]
fn dropping_runtime_closes_active_pointer_before_one_widget_unmount_without_callbacks() {
    let callbacks = Rc::new(RefCell::new(Vec::new()));
    let unmounts = Rc::new(Cell::new(0));
    let mut runtime = AppRuntime::<App>::mount(State {
        callbacks: Rc::clone(&callbacks),
        unmounts: Rc::clone(&unmounts),
    });
    let style_environment = StyleEnvironment::default();
    let size = LogicalSize::try_new(64.0, 64.0)
        .unwrap_or_else(|_| unreachable!("the test surface size is finite"));
    let publication = runtime
        .publish_surface(&SurfaceBuildContext::tight(&style_environment, size))
        .unwrap_or_else(|_| unreachable!("the test surface publication is admitted"));
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
    let pointer_id =
        PointerId::new(81).unwrap_or_else(|| unreachable!("the pointer id is non-zero"));
    let event = PointerEvent::new(
        pointer_id,
        PointerDeviceKind::Mouse,
        PointerPhase::Down,
        point,
        publication.input_context().clone(),
    )
    .with_buttons(PointerButtons::new([PointerButton::Primary]))
    .with_changed_button(PointerButton::Primary);
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
    callbacks.borrow_mut().clear();

    drop(runtime);

    assert!(callbacks.borrow().is_empty());
    assert_eq!(unmounts.get(), 1);
}
