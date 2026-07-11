use runenui_core::prelude::{StyleTokens, button, column, text};
use runenui_runtime::prelude::{
    ActivationResult, AppRuntime, InputEvent, InputEventResult, KeyModifiers, LogicalPoint,
    LogicalSize, PointerActivationResult, PointerButton, PointerEvent, PointerFocusResult,
    PointerPhase, RuntimeNodeId, SurfaceBuildContext, UiApp, resolve_pointer_event_target,
    resolve_pointer_input_event_target,
};

#[derive(Clone, Debug, Eq, PartialEq)]
enum CounterAction {
    Increment,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Counter {
    count: i32,
}

struct CounterApp;

impl UiApp for CounterApp {
    type State = Counter;
    type Action = CounterAction;

    fn root(state: &Self::State) -> runenui_core::Element<Self::Action> {
        column((
            text::<CounterAction>(state.count.to_string()).id("counter.value"),
            button("+")
                .id("counter.increment")
                .on_press(CounterAction::Increment),
        ))
        .id("counter.root")
        .gap(8.0)
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            CounterAction::Increment => state.count += 1,
        }
    }
}

fn surface_frame(runtime: &AppRuntime<CounterApp>) -> runenui_runtime::SurfaceFrame {
    let tokens = StyleTokens::new();
    let context = SurfaceBuildContext::new(&tokens);
    runtime
        .publish_surface(LogicalSize::new(200.0, 100.0), &context)
        .into_parts()
        .0
}

const fn pointer_event(position: LogicalPoint, target: Option<RuntimeNodeId>) -> PointerEvent {
    PointerEvent::new(
        PointerPhase::Pressed,
        position,
        Some(PointerButton::Primary),
        KeyModifiers::NONE,
        target,
    )
}

#[test]
fn pointer_event_with_target_replaces_target() {
    let event = pointer_event(LogicalPoint::new(1.0, 1.0), None)
        .with_target(Some(RuntimeNodeId::from_index(7)));

    assert_eq!(event.target(), Some(RuntimeNodeId::from_index(7)));
}

#[test]
fn resolve_pointer_event_target_sets_hit_tested_node() {
    let runtime = AppRuntime::<CounterApp>::mount(Counter { count: 0 });
    let frame = surface_frame(&runtime);
    let event = pointer_event(LogicalPoint::new(1.0, 29.0), None);

    let targeted = resolve_pointer_event_target(&frame, event);

    assert_eq!(targeted.target(), Some(RuntimeNodeId::from_index(2)));
}

#[test]
fn resolve_pointer_event_target_clears_stale_target_on_miss() {
    let runtime = AppRuntime::<CounterApp>::mount(Counter { count: 0 });
    let frame = surface_frame(&runtime);
    let event = pointer_event(
        LogicalPoint::new(400.0, 400.0),
        Some(RuntimeNodeId::from_index(2)),
    );

    let targeted = resolve_pointer_event_target(&frame, event);

    assert_eq!(targeted.target(), None);
}

#[test]
fn resolve_pointer_input_event_target_wraps_targeted_pointer_event() -> Result<(), &'static str> {
    let runtime = AppRuntime::<CounterApp>::mount(Counter { count: 0 });
    let frame = surface_frame(&runtime);
    let event = pointer_event(LogicalPoint::new(1.0, 29.0), None);

    let input = resolve_pointer_input_event_target(&frame, event);

    match input {
        InputEvent::Pointer(pointer) => {
            assert_eq!(pointer.target(), Some(RuntimeNodeId::from_index(2)));
            Ok(())
        }
        InputEvent::Keyboard(_) => Err("expected pointer input event"),
    }
}

#[test]
fn resolved_pointer_input_event_drives_runtime_input_facade() {
    let mut runtime = AppRuntime::<CounterApp>::mount(Counter { count: 0 });
    let frame = surface_frame(&runtime);
    let event = pointer_event(LogicalPoint::new(1.0, 29.0), None);
    let input = resolve_pointer_input_event_target(&frame, event);

    let result = runtime.handle_input_event(&input);

    assert_eq!(
        result,
        InputEventResult::Pointer {
            focus: PointerFocusResult::Moved(RuntimeNodeId::from_index(2)),
            activation: PointerActivationResult::Handled(ActivationResult::Dispatched),
        }
    );
    assert_eq!(runtime.state().count, 1);
}
