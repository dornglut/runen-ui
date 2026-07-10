use runenui_core::prelude::{button, column};

#[derive(Clone, Debug, Eq, PartialEq)]
enum Action {
    Press,
}

#[test]
fn element_key_is_separate_from_authored_id() {
    let element = button("Open")
        .id("toolbar.open")
        .key("document-42")
        .on_press(Action::Press);

    assert_eq!(
        element.element_id().map(runenui_core::ElementId::as_str),
        Some("toolbar.open")
    );
    assert_eq!(
        element.element_key().map(runenui_core::ElementKey::as_str),
        Some("document-42")
    );
}

#[test]
fn container_children_preserve_keys() -> Result<(), &'static str> {
    let list = column((
        button::<Action>("First").key("item-a"),
        button::<Action>("Second").key("item-b"),
    ));

    let runenui_core::ElementKind::Container(container) = list.kind() else {
        return Err("expected container");
    };

    assert_eq!(
        container.children()[0]
            .element_key()
            .map(runenui_core::ElementKey::as_str),
        Some("item-a")
    );
    assert_eq!(
        container.children()[1]
            .element_key()
            .map(runenui_core::ElementKey::as_str),
        Some("item-b")
    );

    Ok(())
}
