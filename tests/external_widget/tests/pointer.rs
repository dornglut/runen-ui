#![allow(refining_impl_trait)]

use std::{cell::RefCell, rc::Rc};

use runenui_core::{
    Element, EventContext, EventPhase, HitContribution, HitContributionContext, LogicalDelta,
    LogicalLength, LogicalPoint, LogicalRect, NoHostProtocol, PointerButton, PointerButtons,
    PointerCaptureKind, PointerDeviceKind, PointerEvent, PointerId, PointerPhase, StyleEnvironment,
    UiApp, UiEvent, View, Widget, WidgetEventOutput, WidgetMeasure,
};
use runenui_runtime::{AppRuntime, LogicalSize, PumpBudget, SurfaceBuildContext};

#[derive(Clone, Debug, Eq, PartialEq)]
enum Observation {
    Pointer {
        phase: PointerPhase,
        callback_phase: EventPhase,
        physical_target: bool,
    },
    Boundary,
    Capture(PointerCaptureKind),
    LogicalScroll,
}

#[derive(Debug)]
struct ChildAction;

#[derive(Debug)]
enum Action {
    Child(ChildAction),
}

#[derive(Clone)]
struct State {
    observations: Rc<RefCell<Vec<Observation>>>,
}

#[derive(Debug)]
struct ExternalPointerWidget {
    observations: Rc<RefCell<Vec<Observation>>>,
}

impl Widget<ChildAction> for ExternalPointerWidget {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn event(
        &mut self,
        _state: &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, ChildAction>,
    ) -> WidgetEventOutput {
        let observation = match event {
            UiEvent::Pointer(pointer) => {
                if pointer.phase() == PointerPhase::Down {
                    context.capture_pointer();
                }
                Observation::Pointer {
                    phase: pointer.phase(),
                    callback_phase: context.phase(),
                    physical_target: context.physical_target().is_some(),
                }
            }
            UiEvent::PointerBoundary(_) => Observation::Boundary,
            UiEvent::PointerCapture(capture) => Observation::Capture(capture.kind()),
            UiEvent::SemanticCommand(command)
                if matches!(
                    command.command(),
                    runenui_core::SemanticCommand::LogicalScroll(_)
                ) =>
            {
                Observation::LogicalScroll
            }
            _ => return WidgetEventOutput::none(),
        };
        self.observations.borrow_mut().push(observation);
        WidgetEventOutput::none()
    }

    fn measure(
        &self,
        _state: &Self::State,
        _input: runenui_core::WidgetMeasureInput,
    ) -> WidgetMeasure {
        WidgetMeasure::measured(LogicalLength::from(24_u16), LogicalLength::from(24_u16))
    }

    fn hit_test(&self, _state: &Self::State, context: HitContributionContext) -> HitContribution {
        let size = context.local_size();
        let rect = LogicalRect::try_new(0.0, 0.0, size.width(), size.height())
            .unwrap_or_else(|_| unreachable!("validated local size yields a valid hit rectangle"));
        HitContribution::single_rect(rect)
    }
}

struct App;

impl UiApp for App {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        Element::new(ExternalPointerWidget {
            observations: Rc::clone(&state.observations),
        })
        .id("external.pointer")
        .map_action(Action::Child)
    }

    fn update(_state: &mut Self::State, action: Self::Action) {
        match action {
            Action::Child(ChildAction) => {}
        }
    }
}

fn pointer_event(
    id: u64,
    phase: PointerPhase,
    point: LogicalPoint,
    context: runenui_core::SurfaceInputContext,
) -> PointerEvent {
    let pointer_id =
        PointerId::new(id).unwrap_or_else(|| unreachable!("test pointer identities are non-zero"));
    let mut event = PointerEvent::new(pointer_id, PointerDeviceKind::Mouse, phase, point, context);
    if matches!(phase, PointerPhase::Down | PointerPhase::Up) {
        event = event.with_changed_button(PointerButton::Primary);
    }
    if matches!(phase, PointerPhase::Down | PointerPhase::Move) {
        event = event.with_buttons(PointerButtons::new([PointerButton::Primary]));
    }
    if phase == PointerPhase::Wheel {
        event = event.with_scroll_delta(
            LogicalDelta::new(0.0, 3.0)
                .unwrap_or_else(|_| unreachable!("the logical delta is finite")),
        );
    }
    event
}

fn settle(runtime: &mut AppRuntime<App>) {
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
fn downstream_widget_uses_public_pointer_capture_boundary_and_wheel_protocol() {
    let observations = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = AppRuntime::<App>::mount(State {
        observations: Rc::clone(&observations),
    });
    settle(&mut runtime);
    let style_environment = StyleEnvironment::default();
    let publication = runtime
        .publish_surface(&SurfaceBuildContext::tight(
            &style_environment,
            LogicalSize::try_new(64.0, 64.0)
                .unwrap_or_else(|_| unreachable!("the surface size is finite")),
        ))
        .unwrap_or_else(|_| unreachable!("pointer conformance publication is admitted"));
    let context = publication.input_context().clone();
    let bounds = publication.frame().nodes()[0].bounds();
    let inside = LogicalPoint::new(bounds.x() + 1.0, bounds.y() + 1.0)
        .unwrap_or_else(|_| unreachable!("published bounds are finite"));
    let outside = LogicalPoint::new(65.0, 65.0)
        .unwrap_or_else(|_| unreachable!("the outside point is finite"));

    for event in [
        pointer_event(1, PointerPhase::Down, inside, context.clone()),
        pointer_event(1, PointerPhase::Move, outside, context.clone()),
        pointer_event(1, PointerPhase::Up, outside, context.clone()),
        pointer_event(2, PointerPhase::Wheel, inside, context),
    ] {
        runtime
            .submit_pointer(event)
            .unwrap_or_else(|_| unreachable!("the displayed pointer event is accepted"));
        settle(&mut runtime);
    }

    let observations = observations.borrow();
    assert!(observations.contains(&Observation::Boundary));
    assert!(observations.contains(&Observation::Capture(PointerCaptureKind::Gained)));
    assert!(observations.contains(&Observation::Capture(PointerCaptureKind::Lost)));
    assert!(observations.contains(&Observation::Pointer {
        phase: PointerPhase::Move,
        callback_phase: EventPhase::Target,
        physical_target: false,
    }));
    assert_eq!(
        observations
            .iter()
            .filter(|observation| **observation == Observation::LogicalScroll)
            .count(),
        1
    );
}
