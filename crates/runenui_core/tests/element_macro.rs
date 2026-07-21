use runenui_core::{Element, button, children, column, element, text};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Action {
    Hit,
}

#[test]
fn macro_and_builder_are_identical_and_have_no_child_ceiling() {
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
        button("12").on_activate(|| Action::Hit),
    ]));
    let builder_root: Element<Action> = runenui_core::View::into_element(column(children![
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
        button("12").on_activate(|| Action::Hit),
    ]));
    assert_eq!(format!("{macro_root:?}"), format!("{builder_root:?}"));
    assert_eq!(macro_root.children().len(), 13);
}
