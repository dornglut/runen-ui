use runenui_core::prelude::{
    Axis, ButtonArgs, ContainerArgs, Element, ElementKind, TextArgs, button, button_with,
    container_with, text, text_with,
};

#[derive(Clone, Debug, Eq, PartialEq)]
enum Action {
    Save,
}

#[test]
fn text_args_match_builder_chain() -> Result<(), &'static str> {
    let from_args = text_with::<Action>(TextArgs::new("Title").id("title").key("title-key"));
    let from_builder = text::<Action>("Title").id("title").key("title-key");

    assert_eq!(from_args, from_builder);

    let ElementKind::Text(text) = from_args.kind() else {
        return Err("expected text element");
    };

    assert_eq!(text.content(), "Title");
    assert_eq!(
        from_args.element_id().map(runenui_core::ElementId::as_str),
        Some("title")
    );
    assert_eq!(
        from_args
            .element_key()
            .map(runenui_core::ElementKey::as_str),
        Some("title-key")
    );

    Ok(())
}

#[test]
fn button_args_match_builder_chain() -> Result<(), &'static str> {
    let from_args = button_with(
        ButtonArgs::new("Save")
            .id("toolbar.save")
            .key("document-42")
            .disabled()
            .on_press(Action::Save),
    );
    let from_builder = button("Save")
        .id("toolbar.save")
        .key("document-42")
        .disabled()
        .on_press(Action::Save);

    assert_eq!(from_args, from_builder);

    let ElementKind::Button(button) = from_args.kind() else {
        return Err("expected button element");
    };

    assert_eq!(button.label(), "Save");
    assert!(!button.enabled());
    assert_eq!(button.on_press(), Some(&Action::Save));

    Ok(())
}

#[test]
fn container_args_match_builder_chain() -> Result<(), &'static str> {
    let children = (text::<Action>("A"), button::<Action>("B"));
    let from_args = container_with(
        ContainerArgs::new(Axis::Horizontal, children)
            .id("row.main")
            .key("row-key")
            .gap(8_u16),
    );
    let from_builder = Element::container(
        Axis::Horizontal,
        (text::<Action>("A"), button::<Action>("B")),
    )
    .id("row.main")
    .key("row-key")
    .gap(8_u16);

    assert_eq!(from_args, from_builder);

    let ElementKind::Container(container) = from_args.kind() else {
        return Err("expected container element");
    };

    assert_eq!(container.axis(), Axis::Horizontal);
    assert_eq!(container.children().len(), 2);

    let gap_delta = (from_args.style().gap().value() - 8.0).abs();
    assert!(
        gap_delta <= f32::EPSILON,
        "expected gap to equal 8.0; delta was {gap_delta}",
    );

    Ok(())
}
