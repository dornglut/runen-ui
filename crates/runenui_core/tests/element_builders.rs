use runenui_core::{
    Element, FontFamily, GenericFontFamily, LogicalLength, StyleRecipeId, StyleVariantId,
    Typography, TypographyToken, TypographyValue, View, Widget, button, children, column, row,
    text,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Action {
    Hit,
}

#[derive(Debug)]
struct Probe;

impl Widget<Action> for Probe {
    type State = ();

    fn create_state(&self) -> Self::State {}
}

#[test]
fn builders_preserve_nested_structure_and_style() {
    let root: Element<Action> = column(children![
        text("Title"),
        row(children![
            button("A").on_activate(|| Action::Hit),
            button("B")
        ])
        .gap(4_u16),
    ])
    .gap(8_u16)
    .into_element();
    assert_eq!(root.children().len(), 2);
    assert!((root.children()[1].layout().gap().get() - 4.0).abs() <= f32::EPSILON);
}

#[test]
fn recipes_and_variants_are_publicly_authored_in_stable_order()
-> Result<(), Box<dyn std::error::Error>> {
    let recipe = StyleRecipeId::from_static("control.button")?;
    let compact = StyleVariantId::from_static("compact")?;
    let danger = StyleVariantId::from_static("danger")?;

    let custom: Element<Action> = Element::new(Probe)
        .recipe(recipe.clone())
        .variant(compact.clone())
        .variant(danger.clone());
    assert_eq!(custom.style().recipe(), Some(&recipe));
    assert_eq!(
        custom.style().variants(),
        &[compact.clone(), danger.clone()]
    );

    let root: Element<Action> = column(children![
        text("Title")
            .recipe(recipe.clone())
            .variant(compact.clone()),
        button("A").recipe(recipe.clone()).variant(danger.clone())
    ])
    .recipe(recipe.clone())
    .variant(compact.clone())
    .variant(danger.clone())
    .into_element();

    assert_eq!(root.style().recipe(), Some(&recipe));
    assert_eq!(root.style().variants(), &[compact.clone(), danger.clone()]);
    assert_eq!(root.children()[0].style().recipe(), Some(&recipe));
    assert_eq!(
        root.children()[0].style().variants(),
        std::slice::from_ref(&compact)
    );
    assert_eq!(root.children()[1].style().recipe(), Some(&recipe));
    assert_eq!(root.children()[1].style().variants(), &[danger]);
    Ok(())
}

#[test]
fn typography_is_publicly_authored_on_elements_and_builtins()
-> Result<(), Box<dyn std::error::Error>> {
    let typography = Typography::new(
        FontFamily::generic(GenericFontFamily::SansSerif),
        LogicalLength::from(18_u8),
    );
    let token = TypographyToken::parse("text.body")?;

    let custom: Element<Action> = Element::new(Probe).typography(typography.clone());
    assert_eq!(
        custom
            .style()
            .typography()
            .and_then(TypographyValue::as_literal),
        Some(&typography)
    );

    let root: Element<Action> = column(children![
        text("Title").typography(token.clone()),
        button("A").typography(typography.clone())
    ])
    .typography(typography.clone())
    .into_element();

    assert_eq!(
        root.style()
            .typography()
            .and_then(TypographyValue::as_literal),
        Some(&typography)
    );
    assert_eq!(
        root.children()[0]
            .style()
            .typography()
            .and_then(TypographyValue::as_token),
        Some(&token)
    );
    assert_eq!(
        root.children()[1]
            .style()
            .typography()
            .and_then(TypographyValue::as_literal),
        Some(&typography)
    );
    Ok(())
}
