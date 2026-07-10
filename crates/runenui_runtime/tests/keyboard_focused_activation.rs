use runenui_core::prelude::{button, column};
use runenui_runtime::prelude::{
    ActivationResult, AppRuntime, Key, KeyModifiers, KeyPhase, KeyboardActivationResult,
    KeyboardEvent, RuntimeNodeId, RuntimeNodeRef, UiApp,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Action {
    Increment,
    Decrement,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct State {
    count: i32,
}

struct ActivationApp;

impl UiApp for ActivationApp {
    type State = State;
    type Action = Action;

    fn root(_state: &Self::State) -> runenui_core::Element<Self::Action> {
        column((
            button("+").id("increment").on_press(Action::Increment),
            button("-").id("decrement").on_press(Action::Decrement),
            button("No action").id("no-action"),
        ))
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            Action::Increment => state.count += 1,
            Action::Decrement => state.count -= 1,
        }
    }
}

const fn pressed_key(key: Key) -> KeyboardEvent {
    KeyboardEvent::new(KeyPhase::Pressed, key, KeyModifiers::NONE, None)
}

const fn released_key(key: Key) -> KeyboardEvent {
    KeyboardEvent::new(KeyPhase::Released, key, KeyModifiers::NONE, None)
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
fn pressed_enter_activates_focused_node() -> Result<(), &'static str> {
    let mut runtime = AppRuntime::<ActivationApp>::mount(State::default());
    let increment = node_id(&runtime, "increment")?;
    assert!(runtime.set_focus(increment));

    let result = runtime.handle_keyboard_activation(&pressed_key(Key::Enter));

    assert_eq!(
        result,
        KeyboardActivationResult::Handled(ActivationResult::Dispatched)
    );
    assert_eq!(runtime.state().count, 1);
    assert_eq!(runtime.focus().focused_node(), None);

    Ok(())
}

#[test]
fn pressed_space_activates_focused_node() -> Result<(), &'static str> {
    let mut runtime = AppRuntime::<ActivationApp>::mount(State::default());
    let decrement = node_id(&runtime, "decrement")?;
    assert!(runtime.set_focus(decrement));

    let result = runtime.handle_keyboard_activation(&pressed_key(Key::Space));

    assert_eq!(
        result,
        KeyboardActivationResult::Handled(ActivationResult::Dispatched)
    );
    assert_eq!(runtime.state().count, -1);
    assert_eq!(runtime.focus().focused_node(), None);

    Ok(())
}

#[test]
fn focused_no_action_button_returns_no_action_without_rebuild() -> Result<(), &'static str> {
    let mut runtime = AppRuntime::<ActivationApp>::mount(State::default());
    let no_action = node_id(&runtime, "no-action")?;
    assert!(runtime.set_focus(no_action));

    let result = runtime.handle_keyboard_activation(&pressed_key(Key::Enter));

    assert_eq!(
        result,
        KeyboardActivationResult::Handled(ActivationResult::NoAction)
    );
    assert_eq!(runtime.state().count, 0);
    assert_eq!(runtime.focus().focused_node(), Some(no_action));

    Ok(())
}

#[test]
fn activation_key_without_focus_reports_no_focused_node() {
    let mut runtime = AppRuntime::<ActivationApp>::mount(State::default());

    let result = runtime.handle_keyboard_activation(&pressed_key(Key::Enter));

    assert_eq!(result, KeyboardActivationResult::NoFocusedNode);
    assert_eq!(runtime.state().count, 0);
    assert_eq!(runtime.focus().focused_node(), None);
}

#[test]
fn released_activation_key_is_ignored() -> Result<(), &'static str> {
    let mut runtime = AppRuntime::<ActivationApp>::mount(State::default());
    let increment = node_id(&runtime, "increment")?;
    assert!(runtime.set_focus(increment));

    let result = runtime.handle_keyboard_activation(&released_key(Key::Enter));

    assert_eq!(result, KeyboardActivationResult::Ignored);
    assert_eq!(runtime.state().count, 0);
    assert_eq!(runtime.focus().focused_node(), Some(increment));

    Ok(())
}

#[test]
fn tab_is_ignored_by_keyboard_activation_policy() -> Result<(), &'static str> {
    let mut runtime = AppRuntime::<ActivationApp>::mount(State::default());
    let increment = node_id(&runtime, "increment")?;
    assert!(runtime.set_focus(increment));

    let result = runtime.handle_keyboard_activation(&pressed_key(Key::Tab));

    assert_eq!(result, KeyboardActivationResult::Ignored);
    assert_eq!(runtime.state().count, 0);
    assert_eq!(runtime.focus().focused_node(), Some(increment));

    Ok(())
}
