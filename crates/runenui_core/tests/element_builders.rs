use runenui_core::{
    Element, ElementId, ElementKey, FlexContainerStyle, FlexDirection, FontFamily, GenericFontFamily, LayoutContainer,
    LayoutDimension, LayoutStyle, LogicalLength, MotionTarget, PresentationOrigin,
    PresentationRotation, PresentationScale, PresentationTransform, PresentationTranslation,
    PresentationValue, StyleRecipeId, StyleVariantId, TransitionPolicy, Typography,
    TypographyToken, TypographyValue, UnitInterval, View, Widget, button, children, column, row,
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
    let gap = root.children()[1].layout().gap();
    assert!((gap.horizontal().get() - 4.0).abs() <= f32::EPSILON);
    assert!((gap.vertical().get() - 4.0).abs() <= f32::EPSILON);
}

#[test]
fn public_layout_authoring_is_available_on_elements_and_builtin_builders() {
    let flex = LayoutStyle::default().with_container(LayoutContainer::Flex(
        FlexContainerStyle::default().with_direction(FlexDirection::Row),
    ));
    let sized = flex
        .clone()
        .with_width(LayoutDimension::length(LogicalLength::from(120_u16)));

    let custom: Element<Action> = Element::new(Probe).with_layout(sized.clone());
    assert_eq!(custom.layout(), &sized);

    let root = column::<Action>(children![
        text("Title").with_layout(sized.clone()),
        button::<Action>("Save").with_layout(sized.clone()),
    ])
    .with_layout(flex.clone())
    .into_element();
    assert_eq!(root.layout(), &flex);
    assert_eq!(root.children()[0].layout(), &sized);
    assert_eq!(root.children()[1].layout(), &sized);
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


fn presentation_transform() -> PresentationTransform {
    PresentationTransform::new(
        PresentationTranslation::new(8.0, -3.0)
            .unwrap_or_else(|_| unreachable!("controlled translation is finite")),
        PresentationScale::new(1.25, 0.75)
            .unwrap_or_else(|_| unreachable!("controlled scale is finite")),
        PresentationRotation::radians(0.25)
            .unwrap_or_else(|_| unreachable!("controlled rotation is finite")),
        PresentationOrigin::new(
            UnitInterval::new(0.5).unwrap_or_else(|_| unreachable!("controlled unit is valid")),
            UnitInterval::new(0.25).unwrap_or_else(|_| unreachable!("controlled unit is valid")),
        ),
    )
}

#[test]
fn common_presentation_and_transition_authoring_has_element_builtin_parity()
-> Result<(), Box<dyn std::error::Error>> {
    let presentation = presentation_transform();
    let custom: Element<Action> = Element::new(Probe)
        .id("custom")
        .key("custom-key")
        .presentation(presentation)
        .transition_disabled(MotionTarget::Opacity);
    let builtin: Element<Action> = text("Title")
        .id("builtin")
        .key("builtin-key")
        .presentation(presentation)
        .transition_disabled(MotionTarget::Opacity)
        .into_element();

    assert_eq!(custom.element_id(), Some(&ElementId::from_static("custom")?));
    assert_eq!(
        custom.element_key(),
        Some(&ElementKey::from_static("custom-key")?)
    );
    assert_eq!(builtin.element_id(), Some(&ElementId::from_static("builtin")?));
    assert_eq!(
        builtin.element_key(),
        Some(&ElementKey::from_static("builtin-key")?)
    );
    assert_eq!(
        custom
            .style()
            .presentation()
            .and_then(PresentationValue::as_literal),
        Some(presentation)
    );
    assert_eq!(
        builtin
            .style()
            .presentation()
            .and_then(PresentationValue::as_literal),
        Some(presentation)
    );
    assert_eq!(
        custom.style().transition_policy(MotionTarget::Opacity),
        Some(&TransitionPolicy::Disabled)
    );
    assert_eq!(
        builtin.style().transition_policy(MotionTarget::Opacity),
        Some(&TransitionPolicy::Disabled)
    );
    Ok(())
}
