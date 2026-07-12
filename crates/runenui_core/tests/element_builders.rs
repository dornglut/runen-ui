use runenui_core::{Element, View, button, children, column, row, text};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Action {
    Hit,
}

#[test]
fn builders_preserve_nested_structure_and_style() {
    let root: Element<Action> = column(children![
        text("Title"),
        row(children![button("A").on_press(Action::Hit), button("B")]).gap(4_u16),
    ])
    .gap(8_u16)
    .into_element();
    assert_eq!(root.children().len(), 2);
    assert!((root.children()[1].layout().gap().get() - 4.0).abs() <= f32::EPSILON);
}
