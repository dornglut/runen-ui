use runenui_core::{Axis, Element, ElementKind, element};

#[derive(Clone, Debug, Eq, PartialEq)]
enum Action {
    Decrement,
    Increment,
    Reset,
}

fn assert_gap(element: &Element<Action>, expected: f32) {
    let delta = (element.style().gap().value() - expected).abs();
    assert!(
        delta <= f32::EPSILON,
        "expected gap to equal {expected}; delta was {delta}",
    );
}

#[test]
fn element_macro_builds_text_with_identity() -> Result<(), &'static str> {
    let element: Element<Action> =
        element! { text("Counter", id = "counter.title", key = "title-key") };

    assert_eq!(
        element.element_id().map(runenui_core::ElementId::as_str),
        Some("counter.title"),
    );
    assert_eq!(
        element.element_key().map(runenui_core::ElementKey::as_str),
        Some("title-key"),
    );

    let ElementKind::Text(text) = element.kind() else {
        return Err("expected text element");
    };

    assert_eq!(text.content(), "Counter");
    Ok(())
}

#[test]
fn element_macro_builds_button_with_action_and_enabled_state() -> Result<(), &'static str> {
    let element = element! {
        button(
            "+",
            id = "counter.increment",
            key = "increment-key",
            action = Action::Increment,
            enabled = false,
        )
    };

    let ElementKind::Button(button) = element.kind() else {
        return Err("expected button element");
    };

    assert_eq!(button.label(), "+");
    assert!(!button.enabled());
    assert_eq!(button.on_press(), Some(&Action::Increment));
    assert_eq!(
        element.element_id().map(runenui_core::ElementId::as_str),
        Some("counter.increment"),
    );
    assert_eq!(
        element.element_key().map(runenui_core::ElementKey::as_str),
        Some("increment-key"),
    );

    Ok(())
}

#[test]
fn element_macro_builds_nested_counter_tree() -> Result<(), &'static str> {
    let element = element! {
        column(gap = 8_u16, [
            text("Counter"),
            text("0", id = "counter.value"),
            row(gap = 8_u16, [
                button("-", id = "counter.decrement", action = Action::Decrement),
                button("+", id = "counter.increment", action = Action::Increment),
                button("Reset", id = "counter.reset", action = Action::Reset),
            ]),
        ])
    };

    assert_gap(&element, 8.0);

    let ElementKind::Container(root) = element.kind() else {
        return Err("expected root container");
    };

    assert_eq!(root.axis(), Axis::Vertical);
    assert_eq!(root.children().len(), 3);

    let ElementKind::Container(controls) = root.children()[2].kind() else {
        return Err("expected controls row");
    };

    assert_eq!(controls.axis(), Axis::Horizontal);
    assert_eq!(controls.children().len(), 3);
    assert_gap(&root.children()[2], 8.0);

    Ok(())
}
