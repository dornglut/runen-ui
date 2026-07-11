use runenui_core::{Element, ElementId, IntoElement, button, children, column, text};
use runenui_runtime::{
    ActivationResult, AppRuntime, InputEvent, InputEventResult, Key, KeyModifiers, KeyPhase,
    KeyboardActivationResult, KeyboardEvent, KeyboardFocusResult, LogicalPoint,
    PointerActivationResult, PointerButton, PointerEvent, PointerFocusResult, PointerPhase,
    RuntimeNodeRef, UiApp,
};

#[derive(Clone, Copy)]
enum Action {
    Hit,
}
struct App;
impl UiApp for App {
    type State = usize;
    type Action = Action;
    fn root(_: &usize) -> Element<Action> {
        column(children![
            text("Title").id("title"),
            button("First").id("first").on_press(Action::Hit),
            button("Disabled")
                .id("disabled")
                .on_press(Action::Hit)
                .disabled(),
            button("Second").id("second").on_press(Action::Hit),
        ])
        .into_element()
    }
    fn update(state: &mut usize, _: Action) {
        *state += 1;
    }
}

fn node(
    runtime: &AppRuntime<App>,
    id: &str,
) -> Result<runenui_runtime::RuntimeNodeId, &'static str> {
    let id = ElementId::new(id).map_err(|_| "id")?;
    runtime
        .index()
        .node_by_authored_id(&id)
        .map(RuntimeNodeRef::id)
        .ok_or("node")
}

#[test]
fn focus_keyboard_and_pointer_policies_regress() -> Result<(), &'static str> {
    let mut runtime = AppRuntime::<App>::mount(0);
    let first = node(&runtime, "first")?;
    assert_eq!(
        runtime.handle_keyboard_focus(&KeyboardEvent::new(
            KeyPhase::Pressed,
            Key::Tab,
            KeyModifiers::NONE,
            None
        )),
        KeyboardFocusResult::Moved(first)
    );
    assert_eq!(
        runtime.handle_keyboard_activation(&KeyboardEvent::new(
            KeyPhase::Pressed,
            Key::Enter,
            KeyModifiers::NONE,
            None
        )),
        KeyboardActivationResult::Handled(ActivationResult::Dispatched)
    );
    assert_eq!(*runtime.state(), 1);

    let second = node(&runtime, "second")?;
    let point = LogicalPoint::new(1.0, 1.0).map_err(|_| "point")?;
    let event = InputEvent::Pointer(PointerEvent::new(
        PointerPhase::Pressed,
        point,
        Some(PointerButton::Primary),
        KeyModifiers::NONE,
        Some(second),
    ));
    assert_eq!(
        runtime.handle_input_event(&event),
        InputEventResult::Pointer {
            focus: PointerFocusResult::Moved(second),
            activation: PointerActivationResult::Handled(ActivationResult::Dispatched),
        }
    );
    assert_eq!(*runtime.state(), 2);
    Ok(())
}

#[test]
fn non_finite_pointer_positions_are_rejected() {
    assert!(LogicalPoint::new(f32::NAN, 0.0).is_err());
    assert!(LogicalPoint::new(0.0, f32::INFINITY).is_err());
}
