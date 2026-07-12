use runenui_core::{Element, ElementKind, button, children, column, element, text};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Action {
    Hit,
}

#[test]
fn macro_and_builder_are_identical_and_have_no_child_ceiling() -> Result<(), &'static str> {
    let macro_root: Element<Action> = element!(column(children![
        text("0"),
        text("1"),
        text("2"),
        text("3"),
        text("4"),
        text("5"),
        text("6"),
        text("7"),
        text("8"),
        text("9"),
        text("10"),
        text("11"),
        button("12").on_press(Action::Hit),
    ]));
    let builder_root: Element<Action> = runenui_core::IntoElement::into_element(column(children![
        text("0"),
        text("1"),
        text("2"),
        text("3"),
        text("4"),
        text("5"),
        text("6"),
        text("7"),
        text("8"),
        text("9"),
        text("10"),
        text("11"),
        button("12").on_press(Action::Hit),
    ]));
    assert_eq!(macro_root, builder_root);
    let ElementKind::Container(container) = macro_root.kind() else {
        return Err("container");
    };
    assert_eq!(container.children().len(), 13);
    Ok(())
}
