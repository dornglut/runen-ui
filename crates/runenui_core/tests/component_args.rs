use runenui_core::{Axis, ElementKind, IntoElement, button, children, column, text};

#[derive(Clone, Debug, Eq, PartialEq)]
enum Action {
    Save,
}

#[test]
fn typed_builders_erase_to_the_expected_element_kinds() -> Result<(), &'static str> {
    let text = text("Title").id("title").into_element();
    let button = button("Save")
        .id("save")
        .disabled()
        .on_press(Action::Save)
        .into_element();
    let container = column(children![text.clone(), button.clone()])
        .gap(8_u16)
        .into_element();

    assert!(matches!(text.kind(), ElementKind::Text(_)));
    assert!(matches!(button.kind(), ElementKind::Button(_)));
    let ElementKind::Container(container_kind) = container.kind() else {
        return Err("container");
    };
    assert_eq!(container_kind.axis(), Axis::Vertical);
    assert_eq!(container_kind.children().len(), 2);
    assert!((container.layout().gap().get() - 8.0).abs() <= f32::EPSILON);
    Ok(())
}
