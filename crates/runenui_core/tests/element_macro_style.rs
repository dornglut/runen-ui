use runenui_core::{
    Color, ColorToken, ColorValue, EdgeInsets, Element, ElementKind, Length, Radius, RadiusToken,
    RadiusValue, SpacingToken, SpacingValue, button, element,
};

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

    let visual_style = element.visual_style();

    assert_eq!(
        visual_style.foreground(),
        Some(&ColorValue::literal(Color::WHITE))
    );
    assert_eq!(
        visual_style.background(),
        Some(&ColorValue::literal(Color::BLACK))
    );
    assert_eq!(
        visual_style.padding(),
        Some(&SpacingValue::literal(EdgeInsets::all(Length::px(4.0))))
    );
    assert_eq!(
        visual_style.radius(),
        Some(&RadiusValue::literal(Radius::all(Length::px(2.0))))
    );
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
    assert_eq!(
        save_button.visual_style().background(),
        Some(&ColorValue::literal(Color::BLACK))
    );
    assert_eq!(
        save_button.visual_style().radius(),
        Some(&RadiusValue::literal(Radius::all(Length::px(3.0))))
    );

    let status_text = &children[1];
    assert_eq!(
        status_text.visual_style().foreground(),
        Some(&ColorValue::literal(Color::WHITE))
    );
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

    assert_eq!(
        element.visual_style().background(),
        Some(&ColorValue::literal(Color::BLACK))
    );
    assert_eq!(
        element.visual_style().padding(),
        Some(&SpacingValue::literal(EdgeInsets::all(Length::px(6.0))))
    );
    assert_eq!(
        element.visual_style().radius(),
        Some(&RadiusValue::literal(Radius::all(Length::px(3.0))))
    );
}

#[test]
fn builder_style_methods_accept_token_backed_values() {
    let element = button::<Action>("Save")
        .foreground(ColorToken::new("color.text.primary"))
        .background(ColorValue::token("color.surface"))
        .padding(SpacingToken::new("space.2"))
        .radius(RadiusToken::new("radius.control"));

    assert_eq!(
        element.visual_style().foreground(),
        Some(&ColorValue::token("color.text.primary"))
    );
    assert_eq!(
        element.visual_style().background(),
        Some(&ColorValue::token("color.surface"))
    );
    assert_eq!(
        element.visual_style().padding(),
        Some(&SpacingValue::token("space.2"))
    );
    assert_eq!(
        element.visual_style().radius(),
        Some(&RadiusValue::token("radius.control"))
    );
}

#[test]
fn brace_macro_accepts_token_backed_style_expressions() {
    let element: Element<()> = element! {
        text "Token label"
            foreground = { ColorToken::new("color.text.primary") }
            background = { ColorValue::token("color.surface") }
            padding = { SpacingToken::new("space.2") }
            radius = { RadiusToken::new("radius.control") }
    };

    assert_eq!(
        element.visual_style().foreground(),
        Some(&ColorValue::token("color.text.primary"))
    );
    assert_eq!(
        element.visual_style().background(),
        Some(&ColorValue::token("color.surface"))
    );
    assert_eq!(
        element.visual_style().padding(),
        Some(&SpacingValue::token("space.2"))
    );
    assert_eq!(
        element.visual_style().radius(),
        Some(&RadiusValue::token("radius.control"))
    );
}

#[test]
fn function_style_macro_accepts_token_backed_style_expressions() {
    let element: Element<Action> = element!(button(
        "Save",
        action = Action::Save,
        background = ColorToken::new("color.surface"),
        padding = SpacingToken::new("space.2"),
        radius = RadiusToken::new("radius.control"),
    ));

    assert_eq!(
        element.visual_style().background(),
        Some(&ColorValue::token("color.surface"))
    );
    assert_eq!(
        element.visual_style().padding(),
        Some(&SpacingValue::token("space.2"))
    );
    assert_eq!(
        element.visual_style().radius(),
        Some(&RadiusValue::token("radius.control"))
    );
}
