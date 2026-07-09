use runenui_core::{Axis, ElementKind, button, column, row, text};

#[derive(Clone, Debug, Eq, PartialEq)]
enum CounterAction {
    Decrement,
    Increment,
    Reset,
}

fn assert_px(actual: f32, expected: f32) {
    assert!((actual - expected).abs() <= f32::EPSILON);
}

#[test]
fn text_element_stores_content_and_id() -> Result<(), &'static str> {
    let element = text::<CounterAction>("Counter").id("counter.title");

    assert_eq!(
        element.element_id().map(runenui_core::ElementId::as_str),
        Some("counter.title")
    );

    let ElementKind::Text(text) = element.kind() else {
        return Err("expected text element");
    };

    assert_eq!(text.content(), "Counter");
    Ok(())
}

#[test]
fn button_element_stores_label_and_typed_press_action() -> Result<(), &'static str> {
    let element = button("+").on_press(CounterAction::Increment);

    let ElementKind::Button(button) = element.kind() else {
        return Err("expected button element");
    };

    assert_eq!(button.label(), "+");
    assert_eq!(button.on_press(), Some(&CounterAction::Increment));
    Ok(())
}

#[test]
fn row_preserves_child_order_and_axis() -> Result<(), &'static str> {
    let element = row::<CounterAction>((button("-"), button("+"))).gap(8_u16);

    assert_px(element.style().gap().value(), 8.0);

    let ElementKind::Container(container) = element.kind() else {
        return Err("expected container element");
    };

    assert_eq!(container.axis(), Axis::Horizontal);
    assert_eq!(container.children().len(), 2);

    let ElementKind::Button(first) = container.children()[0].kind() else {
        return Err("expected first button");
    };
    let ElementKind::Button(second) = container.children()[1].kind() else {
        return Err("expected second button");
    };

    assert_eq!(first.label(), "-");
    assert_eq!(second.label(), "+");
    Ok(())
}

#[test]
fn column_preserves_nested_structure() -> Result<(), &'static str> {
    let element = column::<CounterAction>((
        text("Counter"),
        text("0").id("counter.value"),
        row((
            button("-").on_press(CounterAction::Decrement),
            button("+").on_press(CounterAction::Increment),
            button("Reset").on_press(CounterAction::Reset),
        ))
        .gap(8_u16),
    ))
    .gap(8_u16);

    let ElementKind::Container(container) = element.kind() else {
        return Err("expected root container");
    };

    assert_eq!(container.axis(), Axis::Vertical);
    assert_eq!(container.children().len(), 3);
    assert_px(element.style().gap().value(), 8.0);

    let ElementKind::Text(value) = container.children()[1].kind() else {
        return Err("expected value text");
    };

    assert_eq!(value.content(), "0");
    assert_eq!(
        container.children()[1]
            .element_id()
            .map(runenui_core::ElementId::as_str),
        Some("counter.value")
    );
    Ok(())
}

#[test]
fn builder_output_is_deterministic() {
    fn screen() -> runenui_core::Element<CounterAction> {
        column((
            text("Counter"),
            button("Reset").on_press(CounterAction::Reset),
        ))
        .gap(8_u16)
    }

    assert_eq!(screen(), screen());
}
