use runenui_core::{
    ComputedStyle, PresentationOrigin, PresentationRotation, PresentationScale,
    PresentationTransform, PresentationTranslation, StyleEffects, StyleEnvironment,
    StyleFieldProvenance, StyleIntent, StyleInteractionFacts, StyleResolutionLayer, StyleTokens,
    Typography, UnitInterval, presentation_token, resolve_style_in_environment,
    style_effects_between,
};

fn unit(value: f32) -> UnitInterval {
    UnitInterval::new(value).unwrap_or_else(|_| unreachable!("controlled unit value is valid"))
}

fn presentation() -> PresentationTransform {
    PresentationTransform::new(
        PresentationTranslation::new(8.0, -3.0)
            .unwrap_or_else(|_| unreachable!("controlled translation is finite")),
        PresentationScale::new(1.25, 0.75)
            .unwrap_or_else(|_| unreachable!("controlled scale is finite")),
        PresentationRotation::radians(0.25)
            .unwrap_or_else(|_| unreachable!("controlled rotation is finite")),
        PresentationOrigin::new(unit(0.5), unit(0.25)),
    )
}

#[test]
fn presentation_token_resolves_with_property_local_provenance_and_effect() {
    let token = presentation_token!("presentation.control");
    let expected = presentation();
    let mut tokens = StyleTokens::new();
    tokens
        .define_presentation(token.clone(), expected)
        .unwrap_or_else(|_| unreachable!("controlled token is unique"));

    let resolution = resolve_style_in_environment(
        &StyleIntent::EMPTY.with_presentation(token.clone()),
        &StyleEnvironment::from_tokens(tokens),
        StyleInteractionFacts::NONE,
        None,
    );

    assert_eq!(resolution.computed_style().presentation(), Some(expected));
    assert_eq!(
        resolution.provenance().presentation(),
        &StyleFieldProvenance::ResolvedToken(token)
    );
    assert_eq!(
        resolution.provenance().presentation_layer(),
        Some(&StyleResolutionLayer::AuthoredOverride)
    );

    let baseline = ComputedStyle::EMPTY.with_typography(Typography::default());
    assert_eq!(
        style_effects_between(&baseline, resolution.computed_style()),
        StyleEffects::PRESENTATION
    );
}

#[test]
fn presentation_is_absent_by_default_and_does_not_inherit() {
    let parent = ComputedStyle::EMPTY
        .with_typography(Typography::default())
        .with_presentation(presentation());
    let child = resolve_style_in_environment(
        &StyleIntent::EMPTY,
        &StyleEnvironment::default(),
        StyleInteractionFacts::NONE,
        Some(&parent),
    );

    assert_eq!(child.computed_style().presentation(), None);
    assert_eq!(
        child.provenance().presentation(),
        &StyleFieldProvenance::Absent
    );
    assert_eq!(child.provenance().presentation_layer(), None);
}
