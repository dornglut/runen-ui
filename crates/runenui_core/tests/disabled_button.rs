use runenui_core::{ElementKind, IntoElement, button};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Action {
    Submit,
}

#[test]
fn disabled_button_preserves_action_and_can_be_reenabled() -> Result<(), &'static str> {
    let element = button("Submit")
        .on_press(Action::Submit)
        .disabled()
        .enabled(true)
        .into_element();
    let ElementKind::Button(button) = element.kind() else {
        return Err("button");
    };
    assert!(button.enabled());
    assert_eq!(button.on_press(), Some(&Action::Submit));
    Ok(())
}
