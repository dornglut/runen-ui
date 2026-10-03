#![allow(refining_impl_trait)]

use std::{cell::RefCell, rc::Rc};

use runenui_core::{
    Element, EventContext, EventPhase, HitContribution, HitContributionContext, LogicalDelta,
    LogicalLength, LogicalPoint, LogicalRect, NoHostProtocol, PointerButton, PointerButtons,
    PointerCaptureKind, PointerDeviceKind, PointerEvent, PointerId, PointerPhase,
    PresentationOrigin, PresentationRotation, PresentationScale, PresentationTransform,
    PresentationTranslation, StyleEnvironment, UiApp, UiEvent, UnitInterval, View, Widget,
    WidgetEventOutput, WidgetMeasure,
};
use runenui_runtime::{AppRuntime, LogicalSize, PumpBudget, SurfaceBuildContext};

#[derive(Clone, Debug, Eq, PartialEq)]
enum Observation {
    Pointer {
        phase: PointerPhase,
        callback_phase: EventPhase,
        physical_target: bool,
        local_position: Option<[u32; 2]>,
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
    ShiftPresentation,
    MakePresentationSingular,
}

#[derive(Clone)]
struct State {
    observations: Rc<RefCell<Vec<Observation>>>,
    translation: f32,
    singular: bool,
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
                    local_position: context
                        .pointer_local_position()
                        .map(|point| [point.x().to_bits(), point.y().to_bits()]),
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

fn unit(value: f32) -> UnitInterval {
    UnitInterval::new(value).unwrap_or_else(|_| unreachable!("fixture unit is valid"))
}

fn presentation(translation: f32, singular: bool) -> PresentationTransform {
    PresentationTransform::new(
        PresentationTranslation::new(translation, translation)
            .unwrap_or_else(|_| unreachable!("fixture translation is finite")),
        PresentationScale::new(if singular { 0.0 } else { 1.0 }, 1.0)
            .unwrap_or_else(|_| unreachable!("fixture scale is finite")),
        PresentationRotation::ZERO,
        PresentationOrigin::new(unit(0.0), unit(0.0)),
    )
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
        .key("external.pointer")
        .presentation(presentation(state.translation, state.singular))
        .map_action(Action::Child)
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            Action::Child(ChildAction) => {}
            Action::ShiftPresentation => state.translation = 30.0,
            Action::MakePresentationSingular => state.singular = true,
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
        translation: 0.0,
        singular: false,
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
    assert!(observations.iter().any(|observation| matches!(
        observation,
        Observation::Pointer {
            phase: PointerPhase::Move,
            callback_phase: EventPhase::Target,
            physical_target: false,
            local_position: Some([x, y]),
        } if *x == 65.0_f32.to_bits() && *y == 65.0_f32.to_bits()
    )));
    assert_eq!(
        observations
            .iter()
            .filter(|observation| **observation == Observation::LogicalScroll)
            .count(),
        1
    );
}

#[test]
fn pointer_local_position_uses_exact_retained_transform_through_capture_and_fails_closed_when_singular() {
    let observations = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = AppRuntime::<App>::mount(State {
        observations: Rc::clone(&observations),
        translation: 10.0,
        singular: false,
    });
    settle(&mut runtime);
    let style_environment = StyleEnvironment::default();
    let build = SurfaceBuildContext::tight(
        &style_environment,
        LogicalSize::try_new(80.0, 80.0)
            .unwrap_or_else(|_| unreachable!("the surface size is finite")),
    );
    let initial = runtime
        .publish_surface(&build)
        .unwrap_or_else(|_| unreachable!("translated pointer fixture publishes"));
    let retained_context = initial.input_context().clone();
    let down =
        LogicalPoint::new(12.0, 13.0).unwrap_or_else(|_| unreachable!("fixture point is finite"));

    runtime
        .submit_pointer(pointer_event(
            11,
            PointerPhase::Down,
            down,
            retained_context.clone(),
        ))
        .unwrap_or_else(|_| unreachable!("translated pointer down is admitted"));
    settle(&mut runtime);
    assert!(observations.borrow().iter().any(|observation| matches!(
        observation,
        Observation::Pointer {
            phase: PointerPhase::Down,
            callback_phase: EventPhase::Target,
            physical_target: true,
            local_position: Some([x, y]),
        } if *x == 2.0_f32.to_bits() && *y == 3.0_f32.to_bits()
    )));

    observations.borrow_mut().clear();
    runtime
        .submit_action(Action::ShiftPresentation)
        .unwrap_or_else(|_| unreachable!("presentation shift enters the FIFO"));
    settle(&mut runtime);
    let shifted = runtime
        .publish_surface(&build)
        .unwrap_or_else(|_| unreachable!("shifted pointer fixture republishes"));
    assert_ne!(
        shifted.input_context().coordinate_revision(),
        retained_context.coordinate_revision()
    );

    let outside_old_hit =
        LogicalPoint::new(50.0, 50.0).unwrap_or_else(|_| unreachable!("fixture point is finite"));
    runtime
        .submit_pointer(pointer_event(
            11,
            PointerPhase::Move,
            outside_old_hit,
            retained_context,
        ))
        .unwrap_or_else(|_| unreachable!("captured retained-context move is admitted"));
    settle(&mut runtime);
    assert!(observations.borrow().iter().any(|observation| matches!(
        observation,
        Observation::Pointer {
            phase: PointerPhase::Move,
            callback_phase: EventPhase::Target,
            physical_target: false,
            local_position: Some([x, y]),
        } if *x == 40.0_f32.to_bits() && *y == 40.0_f32.to_bits()
    )));

    observations.borrow_mut().clear();
    runtime
        .submit_action(Action::MakePresentationSingular)
        .unwrap_or_else(|_| unreachable!("singular presentation enters the FIFO"));
    settle(&mut runtime);
    let singular = runtime
        .publish_surface(&build)
        .unwrap_or_else(|_| unreachable!("singular pointer fixture republishes"));
    runtime
        .submit_pointer(pointer_event(
            11,
            PointerPhase::Move,
            outside_old_hit,
            singular.input_context().clone(),
        ))
        .unwrap_or_else(|_| unreachable!("captured singular-context move is admitted"));
    settle(&mut runtime);
    assert!(observations.borrow().iter().any(|observation| matches!(
        observation,
        Observation::Pointer {
            phase: PointerPhase::Move,
            callback_phase: EventPhase::Target,
            local_position: None,
            ..
        }
    )));

    runtime
        .submit_pointer(pointer_event(
            11,
            PointerPhase::Up,
            outside_old_hit,
            singular.input_context().clone(),
        ))
        .unwrap_or_else(|_| unreachable!("captured pointer up is admitted"));
    settle(&mut runtime);
}

