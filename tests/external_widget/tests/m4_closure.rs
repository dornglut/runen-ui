#![allow(refining_impl_trait)]

use std::{cell::RefCell, rc::Rc};

use runenui_core::{
    ChildBearingWidget, CommandOrigin, CompositionEvent, Element, EventContext, EventPhase,
    FocusEventKind, HitContribution, HitContributionContext, LogicalLength, LogicalPoint,
    LogicalRect, NoHostProtocol, PointerButton, PointerButtons, PointerCaptureKind,
    PointerDeviceKind, PointerEvent, PointerId, PointerPhase, SemanticCommand, StyleEnvironment,
    UiApp, UiEvent, View, Widget, WidgetActivation, WidgetActivationContext,
    WidgetActivationOutput, WidgetEventOutput, WidgetInvalidation, WidgetMeasure, WidgetTextInput,
    container,
};
use runenui_runtime::{
    AppRuntime, LogicalSize, MountedNodeId, PumpBudget, SurfaceBuildContext, TraceRecordKind,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ChildAction {
    Activated,
    Auxiliary,
    WorkCompleted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Action {
    Child(ChildAction),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RoutedKind {
    Activate,
    CompositionStart,
    CompositionUpdate,
    CompositionEnd,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Fact {
    Routed {
        actor: &'static str,
        phase: EventPhase,
        kind: RoutedKind,
    },
    Focus(FocusEventKind),
    Capture(PointerCaptureKind),
    CapturedMoveOutside,
}

struct State {
    facts: Rc<RefCell<Vec<Fact>>>,
    actions: Vec<ChildAction>,
}

#[derive(Debug)]
struct ClosureAncestor {
    facts: Rc<RefCell<Vec<Fact>>>,
}

impl Widget<Action> for ClosureAncestor {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn event(
        &mut self,
        (): &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, Action>,
    ) -> WidgetEventOutput {
        if let Some(kind) = routed_kind(event) {
            self.facts.borrow_mut().push(Fact::Routed {
                actor: "ancestor",
                phase: context.phase(),
                kind,
            });
        }
        WidgetEventOutput::none()
    }
}

impl ChildBearingWidget<Action> for ClosureAncestor {}

#[derive(Debug)]
struct ClosureWidget {
    facts: Rc<RefCell<Vec<Fact>>>,
}

impl Widget<ChildAction> for ClosureWidget {
    type State = usize;

    fn create_state(&self) -> Self::State {
        0
    }

    fn event(
        &mut self,
        _state: &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, ChildAction>,
    ) -> WidgetEventOutput {
        if let Some(kind) = routed_kind(event) {
            self.facts.borrow_mut().push(Fact::Routed {
                actor: "child",
                phase: context.phase(),
                kind,
            });
        }
        match event {
            UiEvent::Pointer(pointer)
                if context.phase() == EventPhase::Target
                    && pointer.phase() == PointerPhase::Down =>
            {
                context.capture_pointer();
            }
            UiEvent::Pointer(pointer)
                if context.phase() == EventPhase::Target
                    && pointer.phase() == PointerPhase::Move
                    && context.physical_target().is_none() =>
            {
                self.facts.borrow_mut().push(Fact::CapturedMoveOutside);
            }
            UiEvent::PointerCapture(capture) => {
                self.facts.borrow_mut().push(Fact::Capture(capture.kind()));
            }
            UiEvent::Focus(focus) if context.phase() == EventPhase::Target => {
                self.facts.borrow_mut().push(Fact::Focus(focus.kind()));
            }
            _ => {}
        }
        WidgetEventOutput::none()
    }

    fn activation(&self, _state: &Self::State) -> WidgetActivation {
        WidgetActivation::actionable(true)
    }

    fn text_input(&self, _state: &Self::State) -> WidgetTextInput {
        WidgetTextInput::new(true, true)
    }

    fn activate(
        &mut self,
        state: &mut Self::State,
        context: &mut WidgetActivationContext<ChildAction>,
    ) -> WidgetActivationOutput<ChildAction> {
        *state += 1;
        context.invalidate(WidgetInvalidation::PAINT);
        context.emit(ChildAction::Auxiliary);
        context.local_task(async { Some(ChildAction::WorkCompleted) });
        WidgetActivationOutput::changed_with_action(ChildAction::Activated)
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
        container(
            ClosureAncestor {
                facts: Rc::clone(&state.facts),
            },
            vec![
                Element::new(ClosureWidget {
                    facts: Rc::clone(&state.facts),
                })
                .id("external.m4")
                .key("closure")
                .focusable(true)
                .map_action(Action::Child),
            ],
        )
        .id("external.m4.root")
        .key("root")
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            Action::Child(action) => state.actions.push(action),
        }
    }
}

fn routed_kind(event: &UiEvent) -> Option<RoutedKind> {
    match event {
        UiEvent::SemanticCommand(command) if command.command() == SemanticCommand::Activate => {
            Some(RoutedKind::Activate)
        }
        UiEvent::Composition(CompositionEvent::Start(_)) => Some(RoutedKind::CompositionStart),
        UiEvent::Composition(CompositionEvent::Update(_)) => Some(RoutedKind::CompositionUpdate),
        UiEvent::Composition(CompositionEvent::End(_)) => Some(RoutedKind::CompositionEnd),
        _ => None,
    }
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

fn target(runtime: &mut AppRuntime<App>) -> MountedNodeId {
    runtime
        .index()
        .nodes()
        .iter()
        .find(|node| {
            node.authored_id()
                .is_some_and(|id| id.as_str() == "external.m4")
        })
        .unwrap_or_else(|| unreachable!("downstream closure target is mounted"))
        .id()
        .clone()
}

fn pointer_event(
    phase: PointerPhase,
    point: LogicalPoint,
    context: runenui_core::SurfaceInputContext,
) -> PointerEvent {
    let id = PointerId::new(1).unwrap_or_else(|| unreachable!("pointer identity is non-zero"));
    let mut event = PointerEvent::new(id, PointerDeviceKind::Mouse, phase, point, context);
    if matches!(phase, PointerPhase::Down | PointerPhase::Up) {
        event = event.with_changed_button(PointerButton::Primary);
    }
    if matches!(phase, PointerPhase::Down | PointerPhase::Move) {
        event = event.with_buttons(PointerButtons::new([PointerButton::Primary]));
    }
    event
}

fn assert_route(facts: &[Fact], kind: RoutedKind) {
    let route = facts
        .iter()
        .filter_map(|fact| match fact {
            Fact::Routed {
                actor,
                phase,
                kind: actual,
            } if *actual == kind => Some((*actor, *phase)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        route,
        [
            ("ancestor", EventPhase::Capture),
            ("child", EventPhase::Target),
            ("ancestor", EventPhase::Bubble),
        ]
    );
}

fn exercise_composition(runtime: &mut AppRuntime<App>) {
    let start = runtime
        .start_composition(None)
        .unwrap_or_else(|_| unreachable!("composition start is accepted"));
    runtime
        .submit_composition_update(start.generation().clone(), String::from("pré"), None)
        .unwrap_or_else(|_| unreachable!("composition update is accepted"));
    runtime
        .submit_composition_end(start.generation().clone())
        .unwrap_or_else(|_| unreachable!("composition end is accepted"));
    settle(runtime);
}

fn exercise_capture(runtime: &mut AppRuntime<App>) {
    let style_environment = StyleEnvironment::default();
    let publication = runtime
        .publish_surface(&SurfaceBuildContext::tight(
            &style_environment,
            LogicalSize::try_new(64.0, 64.0)
                .unwrap_or_else(|_| unreachable!("surface size is finite")),
        ))
        .unwrap_or_else(|_| unreachable!("M4 closure publication is admitted"));
    let context = publication.input_context().clone();
    let node = publication
        .frame()
        .nodes()
        .iter()
        .find(|node| {
            node.authored_id()
                .is_some_and(|id| id.as_str() == "external.m4")
        })
        .unwrap_or_else(|| unreachable!("closure widget is published"));
    let bounds = node.bounds();
    let inside = LogicalPoint::new(bounds.x() + 1.0, bounds.y() + 1.0)
        .unwrap_or_else(|_| unreachable!("published point is finite"));
    let outside =
        LogicalPoint::new(65.0, 65.0).unwrap_or_else(|_| unreachable!("outside point is finite"));

    for event in [
        pointer_event(PointerPhase::Down, inside, context.clone()),
        pointer_event(PointerPhase::Move, outside, context.clone()),
        pointer_event(PointerPhase::Up, outside, context),
    ] {
        runtime
            .submit_pointer(event)
            .unwrap_or_else(|_| unreachable!("displayed pointer event is accepted"));
        settle(runtime);
    }
}

#[test]
fn m4_close_02_downstream_widget_composes_public_m4_protocols() {
    let facts = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = AppRuntime::<App>::mount(State {
        facts: Rc::clone(&facts),
        actions: Vec::new(),
    });
    settle(&mut runtime);
    let target = target(&mut runtime);

    runtime
        .submit_command(
            target.clone(),
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("public focus command is accepted"));
    settle(&mut runtime);
    assert_eq!(runtime.focus().focused_node(), Some(&target));
    assert!(facts.borrow().contains(&Fact::Focus(FocusEventKind::In)));

    facts.borrow_mut().clear();
    exercise_composition(&mut runtime);
    {
        let facts = facts.borrow();
        assert_route(&facts, RoutedKind::CompositionStart);
        assert_route(&facts, RoutedKind::CompositionUpdate);
        assert_route(&facts, RoutedKind::CompositionEnd);
    }

    facts.borrow_mut().clear();
    exercise_capture(&mut runtime);
    assert!(
        facts
            .borrow()
            .contains(&Fact::Capture(PointerCaptureKind::Gained))
    );
    assert!(
        facts
            .borrow()
            .contains(&Fact::Capture(PointerCaptureKind::Lost))
    );
    assert!(facts.borrow().contains(&Fact::CapturedMoveOutside));

    facts.borrow_mut().clear();
    let trace_start = runtime.trace().len();
    runtime
        .submit_command(
            target,
            SemanticCommand::Activate,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("public activation command is accepted"));
    settle(&mut runtime);
    assert_route(&facts.borrow(), RoutedKind::Activate);
    assert_eq!(
        runtime.state().actions,
        [
            ChildAction::Activated,
            ChildAction::Auxiliary,
            ChildAction::WorkCompleted,
        ]
    );

    let trace = runtime
        .trace()
        .records()
        .skip(trace_start)
        .collect::<Vec<_>>();
    assert!(trace.iter().any(|record| matches!(
        record.kind(),
        TraceRecordKind::SemanticDefaultApplied {
            command: SemanticCommand::Activate
        }
    )));
    assert!(
        trace
            .iter()
            .any(|record| matches!(record.kind(), TraceRecordKind::LocalWorkReady))
    );
    assert_eq!(
        trace
            .iter()
            .filter(|record| matches!(record.kind(), TraceRecordKind::ApplicationStateUpdated))
            .count(),
        3,
        "primary, auxiliary, and owner-local work outputs must all reach application update"
    );
}
