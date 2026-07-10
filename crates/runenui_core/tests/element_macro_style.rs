use runenui_core::{Color, EdgeInsets, Element, ElementKind, Length, Radius, element};

#[derive(Clone, Debug, Eq, PartialEq)]
enum Action {
    Save,
}

#[test]
fn brace_text_style_attributes_attach_visual_style() {
    let element: Element<()> = element! {
        text "Label"
            foreground = { Color::WHITE }
            background = { Color::BLACK }
            padding = { EdgeInsets::all(Length::px(4.0)) }
            radius = { Radius::all(Length::px(2.0)) }
    };

    let visual_style = *element.visual_style();

    assert_eq!(visual_style.foreground(), Some(Color::WHITE));
    assert_eq!(visual_style.background(), Some(Color::BLACK));
    assert_eq!(
        visual_style.padding(),
        Some(EdgeInsets::all(Length::px(4.0)))
    );
    assert_eq!(visual_style.radius(), Some(Radius::all(Length::px(2.0))));
}

#[test]
fn brace_button_style_attributes_work_inside_container() {
    let root: Element<Action> = element! {
        column gap=8_u16 {
            button "Save"
                action=Action::Save
                background = { Color::BLACK }
                radius = { Radius::all(Length::px(3.0)) }

            text "Ready"
                foreground = { Color::WHITE }
        }
    };

    let children = match root.kind() {
        ElementKind::Container(container) => container.children(),
        ElementKind::Text(_) | ElementKind::Button(_) => &[],
    };

    assert_eq!(children.len(), 2);

    let save_button = &children[0];
    assert_eq!(save_button.visual_style().background(), Some(Color::BLACK));
    assert_eq!(
        save_button.visual_style().radius(),
        Some(Radius::all(Length::px(3.0)))
    );

    let status_text = &children[1];
    assert_eq!(status_text.visual_style().foreground(), Some(Color::WHITE));
}

#[test]
fn function_style_macro_accepts_style_expressions() {
    let element: Element<Action> = element!(button(
        "Save",
        action = Action::Save,
        background = Color::BLACK,
        padding = EdgeInsets::all(Length::px(6.0)),
        radius = Radius::all(Length::px(3.0)),
    ));

    assert_eq!(element.visual_style().background(), Some(Color::BLACK));
    assert_eq!(
        element.visual_style().padding(),
        Some(EdgeInsets::all(Length::px(6.0)))
    );
    assert_eq!(
        element.visual_style().radius(),
        Some(Radius::all(Length::px(3.0)))
    );
}
