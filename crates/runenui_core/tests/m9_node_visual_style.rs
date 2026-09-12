use runenui_core::{
    Brush, Color, ComputedStyle, DropShadow, LogicalLength, OpacityToken, Outline, OutlineToken,
    SceneOpacity, ShadowToken, StrokeStyle, StyleEffects, StyleEnvironment, StyleFieldProvenance,
    StyleIntent, StyleInteractionFacts, StyleProperties, StyleResolutionLayer, StyleTheme,
    StyleTokens, TokenId, Typography, UnresolvedStyleToken, opacity_token, outline_token,
    resolve_style_in_environment, shadow_token, style_effects_between,
};

fn outline() -> Outline {
    Outline::new(
        Brush::solid(Color::WHITE),
        StrokeStyle::new(LogicalLength::from(2_u16)),
    )
}

fn shadows() -> Result<Vec<DropShadow>, Box<dyn std::error::Error>> {
    Ok(vec![
        DropShadow::new(1.0, 2.0, LogicalLength::from(3_u16), 4.0, Color::BLACK)?,
        DropShadow::new(-5.0, 6.0, LogicalLength::from(7_u16), -2.0, Color::WHITE)?,
    ])
}

#[test]
fn visual_tokens_resolve_exact_values_order_and_property_local_provenance()
-> Result<(), Box<dyn std::error::Error>> {
    let outline_token = outline_token!("outline.control");
    let shadow_token = shadow_token!("shadow.control");
    let opacity_token = opacity_token!("opacity.control");
    let outline = outline();
    let shadows = shadows()?;
    let opacity = SceneOpacity::new(0.5)?;

    let mut tokens = StyleTokens::new();
    tokens.define_outline(outline_token.clone(), outline.clone())?;
    tokens.define_shadows(shadow_token.clone(), shadows.clone())?;
    tokens.define_opacity(opacity_token.clone(), opacity)?;

    let environment = StyleEnvironment::from_tokens(tokens);
    let resolution = resolve_style_in_environment(
        &StyleIntent::EMPTY
            .with_outline(outline_token.clone())
            .with_shadows(shadow_token.clone())
            .with_opacity(opacity_token.clone()),
        &environment,
        StyleInteractionFacts::NONE,
        None,
    );

    assert_eq!(resolution.computed_style().outline(), Some(&outline));
    assert_eq!(resolution.computed_style().shadows(), shadows.as_slice());
    assert_eq!(resolution.computed_style().opacity(), opacity);
    assert_eq!(
        resolution.provenance().outline(),
        &StyleFieldProvenance::ResolvedToken(outline_token)
    );
    assert_eq!(
        resolution.provenance().shadows(),
        &StyleFieldProvenance::ResolvedToken(shadow_token)
    );
    assert_eq!(
        resolution.provenance().opacity(),
        &StyleFieldProvenance::ResolvedToken(opacity_token)
    );
    for layer in [
        resolution.provenance().outline_layer(),
        resolution.provenance().shadows_layer(),
        resolution.provenance().opacity_layer(),
    ] {
        assert_eq!(layer, Some(&StyleResolutionLayer::AuthoredOverride));
    }

    let baseline = ComputedStyle::EMPTY.with_typography(Typography::default());
    assert_eq!(
        style_effects_between(&baseline, resolution.computed_style()),
        StyleEffects::PAINT
    );
    Ok(())
}

#[test]
fn visual_defaults_are_normalized_and_report_initial_provenance() {
    let resolution = resolve_style_in_environment(
        &StyleIntent::EMPTY,
        &StyleEnvironment::default(),
        StyleInteractionFacts::NONE,
        None,
    );

    assert_eq!(
        resolution.computed_style(),
        &ComputedStyle::EMPTY.with_typography(Typography::default())
    );
    assert!(resolution.computed_style().shadows().is_empty());
    assert_eq!(resolution.computed_style().opacity(), SceneOpacity::OPAQUE);
    assert_eq!(
        resolution.provenance().shadows(),
        &StyleFieldProvenance::Literal
    );
    assert_eq!(
        resolution.provenance().shadows_layer(),
        Some(&StyleResolutionLayer::Initial)
    );
    assert_eq!(
        resolution.provenance().opacity(),
        &StyleFieldProvenance::Literal
    );
    assert_eq!(
        resolution.provenance().opacity_layer(),
        Some(&StyleResolutionLayer::Initial)
    );
}

#[test]
fn missing_higher_visual_tokens_mask_lower_values_to_normalized_defaults()
-> Result<(), Box<dyn std::error::Error>> {
    let lower_shadows = shadows()?;
    let lower_opacity = SceneOpacity::new(0.25)?;
    let missing_outline = OutlineToken::new(TokenId::new("outline.missing")?);
    let missing_shadows = ShadowToken::new(TokenId::new("shadow.missing")?);
    let missing_opacity = OpacityToken::new(TokenId::new("opacity.missing")?);

    let environment = StyleEnvironment::new(StyleTheme::new(StyleTokens::new()))
        .with_framework_defaults(
            StyleProperties::EMPTY
                .with_outline(outline())
                .with_shadows(lower_shadows)
                .with_opacity(lower_opacity),
        );
    let resolution = resolve_style_in_environment(
        &StyleIntent::EMPTY
            .with_outline(missing_outline.clone())
            .with_shadows(missing_shadows.clone())
            .with_opacity(missing_opacity.clone()),
        &environment,
        StyleInteractionFacts::NONE,
        None,
    );

    assert_eq!(resolution.computed_style().outline(), None);
    assert!(resolution.computed_style().shadows().is_empty());
    assert_eq!(resolution.computed_style().opacity(), SceneOpacity::OPAQUE);
    assert_eq!(
        resolution.provenance().outline(),
        &StyleFieldProvenance::MissingToken(missing_outline.clone())
    );
    assert_eq!(
        resolution.provenance().shadows(),
        &StyleFieldProvenance::MissingToken(missing_shadows.clone())
    );
    assert_eq!(
        resolution.provenance().opacity(),
        &StyleFieldProvenance::MissingToken(missing_opacity.clone())
    );
    assert_eq!(
        resolution.unresolved_tokens(),
        &[
            UnresolvedStyleToken::Outline(missing_outline),
            UnresolvedStyleToken::Shadows(missing_shadows),
            UnresolvedStyleToken::Opacity(missing_opacity),
        ]
    );
    Ok(())
}

#[test]
fn visual_node_properties_do_not_inherit() -> Result<(), Box<dyn std::error::Error>> {
    let parent = ComputedStyle::EMPTY
        .with_outline(outline())
        .with_shadows(shadows()?)
        .with_opacity(SceneOpacity::new(0.5)?);
    let child = resolve_style_in_environment(
        &StyleIntent::EMPTY,
        &StyleEnvironment::default(),
        StyleInteractionFacts::NONE,
        Some(&parent),
    );

    assert_eq!(child.computed_style().outline(), None);
    assert!(child.computed_style().shadows().is_empty());
    assert_eq!(child.computed_style().opacity(), SceneOpacity::OPAQUE);
    Ok(())
}
