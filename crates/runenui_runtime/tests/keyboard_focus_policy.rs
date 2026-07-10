use runenui_core::prelude::{button, column, row, text};
use runenui_runtime::prelude::{
    AppRuntime, Key, KeyModifiers, KeyPhase, KeyboardEvent, KeyboardFocusResult, RuntimeNodeId,
    RuntimeNodeRef, UiApp,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Action {
    First,
    Second,
    Disabled,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct State;

struct MixedFocusApp;

impl UiApp for MixedFocusApp {
    type State = State;
    type Action = Action;

    fn root(_state: &Self::State) -> runenui_core::Element<Self::Action> {
        column((
            text("Title"),
            button("First").id("first").on_press(Action::First),
            row((
                button("Disabled")
                    .id("disabled")
                    .on_press(Action::Disabled)
                    .disabled(),
                button("Second").id("second").on_press(Action::Second),
            )),
            button("No action").id("no-action"),
        ))
    }

    fn update(_state: &mut Self::State, _action: Self::Action) {}
}

struct NoFocusableApp;

impl UiApp for NoFocusableApp {
    type State = State;
    type Action = Action;

    fn root(_state: &Self::State) -> runenui_core::Element<Self::Action> {
        column((
            text("Title"),
            button("Disabled")
                .id("disabled")
                .on_press(Action::Disabled)
                .disabled(),
        ))
    }

    fn update(_state: &mut Self::State, _action: Self::Action) {}
}

const fn pressed_key(key: Key, modifiers: KeyModifiers) -> KeyboardEvent {
    KeyboardEvent::new(KeyPhase::Pressed, key, modifiers, None)
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
fn pressed_tab_focuses_first_node_when_unfocused() -> Result<(), &'static str> {
    let mut runtime = AppRuntime::<MixedFocusApp>::mount(State);
    let first = node_id(&runtime, "first")?;
    let event = pressed_key(Key::Tab, KeyModifiers::NONE);

    let result = runtime.handle_keyboard_focus(&event);

    assert_eq!(result, KeyboardFocusResult::Moved(first));
    assert_eq!(runtime.focus().focused_node(), Some(first));

    Ok(())
}

#[test]
fn pressed_tab_moves_to_next_focusable_node_and_wraps() -> Result<(), &'static str> {
    let mut runtime = AppRuntime::<MixedFocusApp>::mount(State);
    let first = node_id(&runtime, "first")?;
    let second = node_id(&runtime, "second")?;
    let no_action = node_id(&runtime, "no-action")?;
    let event = pressed_key(Key::Tab, KeyModifiers::NONE);

    assert_eq!(
        runtime.handle_keyboard_focus(&event),
        KeyboardFocusResult::Moved(first)
    );
    assert_eq!(
        runtime.handle_keyboard_focus(&event),
        KeyboardFocusResult::Moved(second)
    );
    assert_eq!(
        runtime.handle_keyboard_focus(&event),
        KeyboardFocusResult::Moved(no_action)
    );
    assert_eq!(
        runtime.handle_keyboard_focus(&event),
        KeyboardFocusResult::Moved(first)
    );

    Ok(())
}

#[test]
fn pressed_shift_tab_focuses_last_node_when_unfocused() -> Result<(), &'static str> {
    let mut runtime = AppRuntime::<MixedFocusApp>::mount(State);
    let no_action = node_id(&runtime, "no-action")?;
    let event = pressed_key(Key::Tab, KeyModifiers::NONE.with_shift());

    let result = runtime.handle_keyboard_focus(&event);

    assert_eq!(result, KeyboardFocusResult::Moved(no_action));
    assert_eq!(runtime.focus().focused_node(), Some(no_action));

    Ok(())
}

#[test]
fn pressed_shift_tab_moves_to_previous_focusable_node_and_wraps() -> Result<(), &'static str> {
    let mut runtime = AppRuntime::<MixedFocusApp>::mount(State);
    let first = node_id(&runtime, "first")?;
    let second = node_id(&runtime, "second")?;
    let no_action = node_id(&runtime, "no-action")?;
    let event = pressed_key(Key::Tab, KeyModifiers::NONE.with_shift());

    assert_eq!(
        runtime.handle_keyboard_focus(&event),
        KeyboardFocusResult::Moved(no_action)
    );
    assert_eq!(
        runtime.handle_keyboard_focus(&event),
        KeyboardFocusResult::Moved(second)
    );
    assert_eq!(
        runtime.handle_keyboard_focus(&event),
        KeyboardFocusResult::Moved(first)
    );
    assert_eq!(
        runtime.handle_keyboard_focus(&event),
        KeyboardFocusResult::Moved(no_action)
    );

    Ok(())
}

#[test]
fn non_tab_pressed_key_is_ignored() -> Result<(), &'static str> {
    let mut runtime = AppRuntime::<MixedFocusApp>::mount(State);
    let first = node_id(&runtime, "first")?;
    assert!(runtime.set_focus(first));

    let result = runtime.handle_keyboard_focus(&pressed_key(Key::Enter, KeyModifiers::NONE));

    assert_eq!(result, KeyboardFocusResult::Ignored);
    assert_eq!(runtime.focus().focused_node(), Some(first));

    Ok(())
}

#[test]
fn released_tab_is_ignored() -> Result<(), &'static str> {
    let mut runtime = AppRuntime::<MixedFocusApp>::mount(State);
    let first = node_id(&runtime, "first")?;
    assert!(runtime.set_focus(first));

    let result = runtime.handle_keyboard_focus(&released_key(Key::Tab));

    assert_eq!(result, KeyboardFocusResult::Ignored);
    assert_eq!(runtime.focus().focused_node(), Some(first));

    Ok(())
}

#[test]
fn tab_reports_no_focusable_node_when_tree_has_none() {
    let mut runtime = AppRuntime::<NoFocusableApp>::mount(State);
    let event = pressed_key(Key::Tab, KeyModifiers::NONE);

    let result = runtime.handle_keyboard_focus(&event);

    assert_eq!(result, KeyboardFocusResult::NoFocusableNode);
    assert_eq!(runtime.focus().focused_node(), None);
}
