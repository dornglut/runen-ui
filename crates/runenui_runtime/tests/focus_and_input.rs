use runenui_core::{Element, View, button, children, row};
use runenui_runtime::{
    AppRuntime, FocusTargetResult, Key, KeyModifiers, KeyPhase, KeyboardEvent, LogicalPoint,
    PointerButton, PointerEvent, PointerPhase, PumpBudget, UiApp,
};

#[derive(Debug)]
enum Action {
    A,
    B,
}
struct App;
impl UiApp for App {
    type State = usize;
    type Action = Action;
    fn root(_: &usize) -> Element<Action> {
        row(children![
            button("A").id("a").key("a").on_activate(|| Action::A),
            button("B").id("b").key("b").on_activate(|| Action::B)
        ])
        .key("root")
        .into_element()
    }
    fn update(state: &mut usize, _: Action) {
        *state += 1;
    }
}

#[test]
fn mounted_focus_traversal_and_input_policy_work() {
    let mut runtime = AppRuntime::<App>::mount(0);
    let a = runtime.index().nodes()[1].id().clone();
    assert_eq!(runtime.set_focus(a.clone()), FocusTargetResult::Focused);
    let tab = KeyboardEvent::new(KeyPhase::Pressed, Key::Tab, KeyModifiers::NONE, None);
    runtime.handle_keyboard_focus(&tab);
    assert_ne!(runtime.focus().focused_node(), Some(&a));
    let enter = KeyboardEvent::new(KeyPhase::Pressed, Key::Enter, KeyModifiers::NONE, None);
    runtime.handle_keyboard_activation(&enter);
    assert_eq!(runtime.state(), &0);
    runtime.pump(PumpBudget::new(1));
    assert_eq!(runtime.state(), &1);
    let pointer = PointerEvent::new(
        PointerPhase::Pressed,
        LogicalPoint::new(1.0, 1.0).unwrap_or_else(|_| unreachable!()),
        Some(PointerButton::Primary),
        KeyModifiers::NONE,
        Some(a),
    );
    runtime.handle_pointer_focus(&pointer);
}

#[test]
fn non_finite_pointer_positions_are_rejected() {
    assert!(LogicalPoint::new(f32::NAN, 0.0).is_err());
    assert!(LogicalPoint::new(0.0, f32::INFINITY).is_err());
}
