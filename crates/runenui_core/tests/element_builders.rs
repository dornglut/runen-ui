use std::time::Duration;

use runenui_core::{
    AnimationId, Brush, Color, DropShadow, EdgeInsets, Element, ElementId, ElementKey,
    ExplicitTimeline, FlexContainerStyle, FlexDirection, FontFamily, GenericFontFamily,
    LayoutContainer, LayoutDimension, LayoutStyle, LogicalLength, MotionEasing, MotionKeyframe,
    MotionRepeat, MotionTarget, MotionValue, PresentationOrigin, PresentationRotation,
    PresentationScale, PresentationTransform, PresentationTranslation, PresentationValue, Radius,
    ReducedMotionStrategy, SceneOpacity, StrokeStyle, StyleRecipeId, StyleVariantId, TimelineSpec,
    TransitionPolicy, TransitionSpec, Typography, TypographyToken, TypographyValue, UnitInterval,
    View, Widget, button, children, column, row, text,
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

fn common_transition() -> TransitionSpec {
    TransitionSpec::new(
        Duration::from_millis(120),
        Duration::ZERO,
        MotionEasing::Linear,
        Some(ReducedMotionStrategy::PreserveEssential),
    )
    .unwrap_or_else(|_| unreachable!("controlled transition is valid"))
}

fn common_timeline() -> ExplicitTimeline {
    let spec = TimelineSpec::new(
        vec![
            MotionKeyframe::new(
                UnitInterval::ZERO,
                MotionValue::Opacity(SceneOpacity::TRANSPARENT),
            ),
            MotionKeyframe::new(
                UnitInterval::ONE,
                MotionValue::Opacity(SceneOpacity::OPAQUE),
            ),
        ],
        vec![MotionEasing::Linear],
        Duration::from_millis(200),
        Duration::ZERO,
        MotionRepeat::ONCE,
        Some(ReducedMotionStrategy::PreserveEssential),
    )
    .unwrap_or_else(|_| unreachable!("controlled timeline is valid"));
    ExplicitTimeline::new(
        AnimationId::from_static("common-authoring")
            .unwrap_or_else(|_| unreachable!("controlled animation id is valid")),
        spec,
    )
}

fn authored_outline() -> runenui_core::Outline {
    runenui_core::Outline::new(
        Brush::solid(Color::WHITE),
        StrokeStyle::new(LogicalLength::from(2_u16)),
    )
}

fn authored_shadows() -> Vec<DropShadow> {
    vec![
        DropShadow::new(1.0, 2.0, LogicalLength::from(3_u16), 4.0, Color::BLACK)
            .unwrap_or_else(|_| unreachable!("controlled shadow is finite")),
    ]
}

#[test]
fn common_node_authoring_has_element_builtin_parity() -> Result<(), Box<dyn std::error::Error>> {
    let layout =
        LayoutStyle::default().with_width(LayoutDimension::length(LogicalLength::from(120_u16)));
    let recipe = StyleRecipeId::from_static("common.authoring")?;
    let variant = StyleVariantId::from_static("compact")?;
    let typography = Typography::new(
        FontFamily::generic(GenericFontFamily::SansSerif),
        LogicalLength::from(18_u8),
    );
    let padding = EdgeInsets::all(LogicalLength::from(4_u16));
    let radius = Radius::all(LogicalLength::from(6_u16));
    let outline = authored_outline();
    let shadows = authored_shadows();
    let opacity = SceneOpacity::new(0.75)?;
    let presentation = presentation_transform();
    let transition = common_transition();
    let timeline = common_timeline();

    let custom: Element<Action> = Element::new(Probe)
        .id("common")
        .key("common-key")
        .with_layout(layout.clone())
        .recipe(recipe.clone())
        .variant(variant.clone())
        .foreground(Color::WHITE)
        .background(Color::BLACK)
        .padding(padding)
        .radius(radius)
        .typography(typography.clone())
        .outline(outline.clone())
        .shadows(shadows.clone())
        .opacity(opacity)
        .presentation(presentation)
        .transition(MotionTarget::Opacity, transition.clone())
        .transition_disabled(MotionTarget::Foreground)
        .timeline(timeline.clone());

    let builtin: Element<Action> = text("Title")
        .id("common")
        .key("common-key")
        .with_layout(layout.clone())
        .recipe(recipe)
        .variant(variant)
        .foreground(Color::WHITE)
        .background(Color::BLACK)
        .padding(padding)
        .radius(radius)
        .typography(typography)
        .outline(outline)
        .shadows(shadows)
        .opacity(opacity)
        .presentation(presentation)
        .transition(MotionTarget::Opacity, transition)
        .transition_disabled(MotionTarget::Foreground)
        .timeline(timeline)
        .into_element();

    assert_eq!(
        custom.element_id(),
        Some(&ElementId::from_static("common")?)
    );
    assert_eq!(
        custom.element_key(),
        Some(&ElementKey::from_static("common-key")?)
    );
    assert_eq!(custom.element_id(), builtin.element_id());
    assert_eq!(custom.element_key(), builtin.element_key());
    assert_eq!(custom.layout(), builtin.layout());
    assert_eq!(custom.style(), builtin.style());
    assert_eq!(custom.timelines(), builtin.timelines());
    assert!(matches!(
        custom.style().transition_policy(MotionTarget::Opacity),
        Some(TransitionPolicy::Enabled(_))
    ));
    assert_eq!(
        custom.style().transition_policy(MotionTarget::Foreground),
        Some(&TransitionPolicy::Disabled)
    );
    assert_eq!(
        custom
            .style()
            .presentation()
            .and_then(PresentationValue::as_literal),
        Some(presentation)
    );
    Ok(())
}
