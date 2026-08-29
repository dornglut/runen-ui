use runenui_core::{
    Color, ColorToken, ComputedStyle, EdgeInsets, LogicalLength, StyleEffects, StyleEnvironment,
    StyleFieldProvenance, StyleInteractionFacts, StyleInteractionState, StylePreferenceKind,
    StylePreferencePolicy, StylePreferences, StyleProperties, StyleRecipe, StyleRecipeId,
    StyleResolutionDiagnostic, StyleResolutionLayer, StyleTheme, StyleTokens, StyleVariantId,
    TokenId, resolve_style_in_environment, style_effects_between,
};

fn recipe_id(value: &str) -> StyleRecipeId {
    StyleRecipeId::new(value).expect("valid recipe id")
}

fn variant_id(value: &str) -> StyleVariantId {
    StyleVariantId::new(value).expect("valid variant id")
}

#[test]
fn precedence_and_provenance_are_property_local_and_deterministic()
-> Result<(), Box<dyn std::error::Error>> {
    let recipe_id = recipe_id("control.button");
    let compact = variant_id("compact");
    let danger = variant_id("danger");
    let mut recipe = StyleRecipe::new(
        StyleProperties::EMPTY
            .with_foreground(Color::rgb(10, 10, 10))
            .with_background(Color::rgb(20, 20, 20)),
    );
    recipe.define_variant(
        compact.clone(),
        StyleProperties::EMPTY.with_foreground(Color::rgb(30, 30, 30)),
    )?;
    recipe.define_variant(
        danger.clone(),
        StyleProperties::EMPTY.with_foreground(Color::rgb(40, 40, 40)),
    )?;
    recipe.define_interaction(
        StyleInteractionState::Hover,
        StyleProperties::EMPTY.with_foreground(Color::rgb(50, 50, 50)),
    )?;
    recipe.define_interaction(
        StyleInteractionState::Active,
        StyleProperties::EMPTY.with_foreground(Color::rgb(60, 60, 60)),
    )?;
    recipe.define_interaction(
        StyleInteractionState::Disabled,
        StyleProperties::EMPTY.with_foreground(Color::rgb(70, 70, 70)),
    )?;

    let mut theme = StyleTheme::new(StyleTokens::new());
    theme.define_recipe(recipe_id.clone(), recipe)?;
    let environment = StyleEnvironment::new(theme)
        .with_framework_defaults(
            StyleProperties::EMPTY.with_foreground(Color::rgb(1, 1, 1)),
        )
        .with_preferences(StylePreferences::new(true, false))
        .with_preference_policy(
            StylePreferencePolicy::new().with_high_contrast(
                StyleProperties::EMPTY.with_foreground(Color::WHITE),
            ),
        );
    let intent = runenui_core::StyleIntent::EMPTY
        .with_recipe(recipe_id)
        .with_variant(compact)
        .with_variant(danger)
        .with_foreground(Color::rgb(80, 80, 80));
    let resolution = resolve_style_in_environment(
        &intent,
        &environment,
        StyleInteractionFacts::new(true, false, true, true),
        None,
    );

    assert_eq!(resolution.computed_style().foreground(), Some(Color::WHITE));
    assert_eq!(
        resolution.provenance().foreground_layer(),
        Some(&StyleResolutionLayer::Preference(
            StylePreferenceKind::HighContrast
        ))
    );
    assert_eq!(
        resolution.provenance().foreground(),
        &StyleFieldProvenance::Literal
    );
    assert_eq!(
        resolution.computed_style().background(),
        Some(Color::rgb(20, 20, 20))
    );
    Ok(())
}

#[test]
fn ordered_variants_and_framework_interaction_order_are_stable()
-> Result<(), Box<dyn std::error::Error>> {
    let recipe_id = recipe_id("control.button");
    let first = variant_id("first");
    let second = variant_id("second");
    let mut recipe = StyleRecipe::new(StyleProperties::EMPTY);
    recipe.define_variant(
        first.clone(),
        StyleProperties::EMPTY.with_foreground(Color::rgb(10, 0, 0)),
    )?;
    recipe.define_variant(
        second.clone(),
        StyleProperties::EMPTY.with_foreground(Color::rgb(20, 0, 0)),
    )?;
    recipe.define_interaction(
        StyleInteractionState::Hover,
        StyleProperties::EMPTY.with_foreground(Color::rgb(30, 0, 0)),
    )?;
    recipe.define_interaction(
        StyleInteractionState::Active,
        StyleProperties::EMPTY.with_foreground(Color::rgb(40, 0, 0)),
    )?;
    let mut theme = StyleTheme::new(StyleTokens::new());
    theme.define_recipe(recipe_id.clone(), recipe)?;
    let environment = StyleEnvironment::new(theme);
    let intent = runenui_core::StyleIntent::EMPTY
        .with_recipe(recipe_id)
        .with_variant(first)
        .with_variant(second);
    let resolution = resolve_style_in_environment(
        &intent,
        &environment,
        StyleInteractionFacts::new(true, false, true, false),
        None,
    );
    assert_eq!(
        resolution.computed_style().foreground(),
        Some(Color::rgb(40, 0, 0))
    );
    assert_eq!(
        resolution.provenance().foreground_layer(),
        Some(&StyleResolutionLayer::Interaction(
            StyleInteractionState::Active
        ))
    );
    Ok(())
}

#[test]
fn missing_higher_precedence_token_masks_lower_value()
-> Result<(), Box<dyn std::error::Error>> {
    let recipe_id = recipe_id("control.button");
    let mut theme = StyleTheme::new(StyleTokens::new());
    theme.define_recipe(
        recipe_id.clone(),
        StyleRecipe::new(
            StyleProperties::EMPTY.with_foreground(Color::rgb(12, 12, 12)),
        ),
    )?;
    let missing = ColorToken::new(TokenId::new("color.missing")?);
    let environment = StyleEnvironment::new(theme);
    let intent = runenui_core::StyleIntent::EMPTY
        .with_recipe(recipe_id)
        .with_foreground(missing.clone());
    let resolution = resolve_style_in_environment(
        &intent,
        &environment,
        StyleInteractionFacts::default(),
        None,
    );
    assert_eq!(resolution.computed_style().foreground(), None);
    assert_eq!(
        resolution.provenance().foreground(),
        &StyleFieldProvenance::MissingToken(missing.clone())
    );
    assert_eq!(
        resolution.provenance().foreground_layer(),
        Some(&StyleResolutionLayer::AuthoredOverride)
    );
    assert!(resolution.diagnostics().iter().any(|diagnostic| matches!(
        diagnostic,
        StyleResolutionDiagnostic::MissingToken(
            runenui_core::UnresolvedStyleToken::Foreground(token)
        ) if token == &missing
    )));
    Ok(())
}

#[test]
fn inheritance_is_bounded_to_foreground_in_m8a() -> Result<(), Box<dyn std::error::Error>> {
    let padding = EdgeInsets::all(LogicalLength::new(8.0)?);
    let parent = ComputedStyle::EMPTY
        .with_foreground(Color::WHITE)
        .with_padding(padding);
    let resolution = resolve_style_in_environment(
        &runenui_core::StyleIntent::EMPTY,
        &StyleEnvironment::default(),
        StyleInteractionFacts::default(),
        Some(parent),
    );
    assert_eq!(resolution.computed_style().foreground(), Some(Color::WHITE));
    assert_eq!(resolution.computed_style().padding(), None);
    assert_eq!(
        resolution.provenance().foreground(),
        &StyleFieldProvenance::Inherited
    );
    assert_eq!(
        resolution.provenance().foreground_layer(),
        Some(&StyleResolutionLayer::Inherited)
    );
    Ok(())
}

#[test]
fn property_effects_distinguish_paint_from_layout() -> Result<(), Box<dyn std::error::Error>> {
    let paint = style_effects_between(
        ComputedStyle::EMPTY,
        ComputedStyle::EMPTY.with_foreground(Color::WHITE),
    );
    assert_eq!(paint, StyleEffects::PAINT);
    let layout = style_effects_between(
        ComputedStyle::EMPTY,
        ComputedStyle::EMPTY.with_padding(EdgeInsets::all(LogicalLength::new(4.0)?)),
    );
    assert_eq!(layout, StyleEffects::LAYOUT);
    Ok(())
}
