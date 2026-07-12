use runenui_core::{Axis, ChildLayout, View, WidgetMeasure, button, children, column, text};

#[derive(Clone, Debug, Eq, PartialEq)]
enum Action {
    Save,
}

#[test]
fn typed_builders_use_the_open_widget_protocol() {
    let text = text("Title").id("title").into_element();
    let button = button("Save")
        .id("save")
        .disabled()
        .on_press(Action::Save)
        .into_element();
    assert!(matches!(text.measure(), WidgetMeasure::Text { .. }));
    assert_eq!(button.semantics().role(), "button");
    let container = column(children![text, button]).gap(8_u16).into_element();

    assert_eq!(
        container.child_layout(),
        Some(ChildLayout::Linear {
            axis: Axis::Vertical
        })
    );
    assert_eq!(container.measure(), WidgetMeasure::default());
    assert_eq!(container.children().len(), 2);
    assert!((container.layout().gap().get() - 8.0).abs() <= f32::EPSILON);
}
