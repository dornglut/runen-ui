use runenui_core::prelude::{button, column, row, text};
use runenui_runtime::prelude::{
    ActivationResult, AppRuntime, InputEvent, InputEventResult, Key, KeyModifiers, KeyPhase,
    KeyboardActivationResult, KeyboardEvent, KeyboardFocusResult, LogicalPoint,
    PointerActivationResult, PointerButton, PointerEvent, PointerFocusResult, PointerPhase,
    RuntimeNodeId, RuntimeNodeRef, UiApp,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Action {
    Increment,
    Disabled,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct State {
    count: i32,
}

struct MixedInputApp;

impl UiApp for MixedInputApp {
    type State = State;
    type Action = Action;

    fn root(_state: &Self::State) -> runenui_core::Element<Self::Action> {
        column((
            text("Title").id("title"),
            button("Increment")
                .id("increment")
                .on_press(Action::Increment),
            row((
                button("Disabled")
                    .id("disabled")
                    .on_press(Action::Disabled)
                    .disabled(),
                button("No action").id("no-action"),
            )),
        ))
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            Action::Increment => state.count += 1,
            Action::Disabled => state.count = -1,
        }
    }
}

const fn pointer_event(
    phase: PointerPhase,
    button: Option<PointerButton>,
    target: Option<RuntimeNodeId>,
) -> InputEvent {
    InputEvent::Pointer(PointerEvent::new(
        phase,
        LogicalPoint::new(10.0, 20.0),
        button,
        KeyModifiers::NONE,
        target,
    ))
}

const fn primary_press(target: Option<RuntimeNodeId>) -> InputEvent {
    pointer_event(PointerPhase::Pressed, Some(PointerButton::Primary), target)
}

const fn key_event(phase: KeyPhase, key: Key, modifiers: KeyModifiers) -> InputEvent {
    InputEvent::Keyboard(KeyboardEvent::new(phase, key, modifiers, None))
}

const fn pressed_key(key: Key, modifiers: KeyModifiers) -> InputEvent {
    key_event(KeyPhase::Pressed, key, modifiers)
}

fn node_id<App>(runtime: &AppRuntime<App>, id: &str) -> Result<RuntimeNodeId, &'static str>
where
    App: UiApp,
{
    runtime
        .index()
        .node_by_authored_id(id)
        .map(RuntimeNodeRef::id)
        .ok_or("expected authored node")
}

#[test]
fn pointer_primary_press_focuses_then_activates_target() -> Result<(), &'static str> {
    let mut runtime = AppRuntime::<MixedInputApp>::mount(State::default());
    let increment = node_id(&runtime, "increment")?;

    let result = runtime.handle_input_event(&primary_press(Some(increment)));

    assert_eq!(
        result,
        InputEventResult::Pointer {
            focus: PointerFocusResult::Moved(increment),
            activation: PointerActivationResult::Handled(ActivationResult::Dispatched),
        }
    );
    assert_eq!(runtime.state().count, 1);
    assert_eq!(runtime.focus().focused_node(), None);

    Ok(())
}

#[test]
fn pointer_primary_press_on_no_action_target_keeps_focus_and_reports_no_action()
-> Result<(), &'static str> {
    let mut runtime = AppRuntime::<MixedInputApp>::mount(State::default());
    let no_action = node_id(&runtime, "no-action")?;

    let result = runtime.handle_input_event(&primary_press(Some(no_action)));

    assert_eq!(
        result,
        InputEventResult::Pointer {
            focus: PointerFocusResult::Moved(no_action),
            activation: PointerActivationResult::Handled(ActivationResult::NoAction),
        }
    );
    assert_eq!(runtime.state().count, 0);
    assert_eq!(runtime.focus().focused_node(), Some(no_action));

    Ok(())
}

#[test]
fn pointer_primary_press_on_disabled_target_reports_focus_and_activation_details()
-> Result<(), &'static str> {
    let mut runtime = AppRuntime::<MixedInputApp>::mount(State::default());
    let disabled = node_id(&runtime, "disabled")?;

    let result = runtime.handle_input_event(&primary_press(Some(disabled)));

    assert_eq!(
        result,
        InputEventResult::Pointer {
            focus: PointerFocusResult::NotFocusable,
            activation: PointerActivationResult::Handled(ActivationResult::Disabled),
        }
    );
    assert_eq!(runtime.state().count, 0);
    assert_eq!(runtime.focus().focused_node(), None);

    Ok(())
}

#[test]
fn pointer_non_primary_or_non_pressed_events_are_ignored() -> Result<(), &'static str> {
    let mut runtime = AppRuntime::<MixedInputApp>::mount(State::default());
    let increment = node_id(&runtime, "increment")?;

    assert_eq!(
        runtime.handle_input_event(&pointer_event(
            PointerPhase::Pressed,
            Some(PointerButton::Secondary),
            Some(increment),
        )),
        InputEventResult::Ignored
    );
    assert_eq!(
        runtime.handle_input_event(&pointer_event(PointerPhase::Moved, None, Some(increment))),
        InputEventResult::Ignored
    );

    Ok(())
}

#[test]
fn keyboard_tab_routes_to_focus_policy() -> Result<(), &'static str> {
    let mut runtime = AppRuntime::<MixedInputApp>::mount(State::default());
    let increment = node_id(&runtime, "increment")?;

    let result = runtime.handle_input_event(&pressed_key(Key::Tab, KeyModifiers::NONE));

    assert_eq!(
        result,
        InputEventResult::KeyboardFocus(KeyboardFocusResult::Moved(increment))
    );
    assert_eq!(runtime.focus().focused_node(), Some(increment));
    assert_eq!(runtime.state().count, 0);

    Ok(())
}

#[test]
fn keyboard_enter_routes_to_activation_policy() -> Result<(), &'static str> {
    let mut runtime = AppRuntime::<MixedInputApp>::mount(State::default());
    let increment = node_id(&runtime, "increment")?;
    assert!(runtime.set_focus(increment));

    let result = runtime.handle_input_event(&pressed_key(Key::Enter, KeyModifiers::NONE));

    assert_eq!(
        result,
        InputEventResult::KeyboardActivation(KeyboardActivationResult::Handled(
            ActivationResult::Dispatched,
        ))
    );
    assert_eq!(runtime.state().count, 1);
    assert_eq!(runtime.focus().focused_node(), None);

    Ok(())
}

#[test]
fn keyboard_space_without_focus_reports_no_focused_node() {
    let mut runtime = AppRuntime::<MixedInputApp>::mount(State::default());

    let result = runtime.handle_input_event(&pressed_key(Key::Space, KeyModifiers::NONE));

    assert_eq!(
        result,
        InputEventResult::KeyboardActivation(KeyboardActivationResult::NoFocusedNode)
    );
    assert_eq!(runtime.state().count, 0);
}

#[test]
fn keyboard_non_runtime_policy_key_is_ignored() {
    let mut runtime = AppRuntime::<MixedInputApp>::mount(State::default());

    let result = runtime.handle_input_event(&pressed_key(Key::Character('a'), KeyModifiers::NONE));

    assert_eq!(result, InputEventResult::Ignored);
    assert_eq!(runtime.state().count, 0);
}
