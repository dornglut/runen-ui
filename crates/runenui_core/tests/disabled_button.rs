use runenui_core::{ElementKind, button, text};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Action {
    Submit,
}

#[test]
fn buttons_are_enabled_by_default() -> Result<(), &'static str> {
    let element = button("Submit").on_press(Action::Submit);

    let ElementKind::Button(button) = element.kind() else {
        return Err("expected button element");
    };

    assert!(button.enabled());
    assert_eq!(button.on_press(), Some(&Action::Submit));
    Ok(())
}

#[test]
fn disabled_marks_button_inactive_without_removing_action() -> Result<(), &'static str> {
    let element = button("Submit").on_press(Action::Submit).disabled();

    let ElementKind::Button(button) = element.kind() else {
        return Err("expected button element");
    };

    assert!(!button.enabled());
    assert_eq!(button.on_press(), Some(&Action::Submit));
    Ok(())
}

#[test]
fn enabled_builder_can_restore_a_disabled_button() -> Result<(), &'static str> {
    let element = button("Submit")
        .on_press(Action::Submit)
        .disabled()
        .enabled(true);

    let ElementKind::Button(button) = element.kind() else {
        return Err("expected button element");
    };

    assert!(button.enabled());
    Ok(())
}

#[test]
fn disabled_builder_is_noop_for_non_buttons() {
    let element = text::<Action>("Static").disabled();

    assert!(matches!(element.kind(), ElementKind::Text(_)));
}
