use runenui_core::{
    Color, ColorToken, ComputedStyle, EdgeInsets, IdentifierError, LogicalLength, StyleEffects,
    StyleEnvironment, StyleFieldProvenance, StyleIntent, StyleInteractionFacts,
    StyleInteractionState, StylePreferenceKind, StylePreferencePolicy, StylePreferences,
    StyleProperties, StyleRecipe, StyleRecipeId, StyleResolutionDiagnostic, StyleResolutionLayer,
    StyleTheme, StyleTokens, StyleVariantId, TokenId, resolve_style_in_environment,
    style_effects_between,
};

fn recipe_id(value: &str) -> Result<StyleRecipeId, IdentifierError> {
    StyleRecipeId::new(value)
}

fn variant_id(value: &str) -> Result<StyleVariantId, IdentifierError> {
    StyleVariantId::new(value)
}

fn assert_foreground_resolution(
    intent: &StyleIntent,
    environment: &StyleEnvironment,
    interaction: StyleInteractionFacts,
    expected_color: Color,
    expected_layer: StyleResolutionLayer,
) {
    let resolution = resolve_style_in_environment(intent, environment, interaction, None);
    assert_eq!(resolution.computed_style().foreground(), Some(expected_color));
    assert_eq!(
        resolution.provenance().foreground_layer(),
        Some(&expected_layer)
    );
}

#[test]
fn precedence_and_provenance_are_property_local_and_deterministic()
-> Result<(), Box<dyn std::error::Error>> {
    let recipe_id = recipe_id("control.button")?;
    let compact = variant_id("compact")?;
    let danger = variant_id("danger")?;
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
        .with_framework_defaults(StyleProperties::EMPTY.with_foreground(Color::rgb(1, 1, 1)))
        .with_preferences(StylePreferences::new(true, false))
        .with_preference_policy(
            StylePreferencePolicy::new()
                .with_high_contrast(StyleProperties::EMPTY.with_foreground(Color::WHITE)),
        );
    let intent = StyleIntent::EMPTY
        .with_recipe(recipe_id)
        .with_variant(compact)
        .with_variant(danger)
        .with_foreground(Color::rgb(80, 80, 80));
    let interaction = StyleInteractionFacts::NONE
        .with(StyleInteractionState::Hover, true)
        .with(StyleInteractionState::Active, true)
        .with(StyleInteractionState::Disabled, true);
    let resolution = resolve_style_in_environment(&intent, &environment, interaction, None);

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
fn every_precedence_edge_has_an_exact_winner() -> Result<(), Box<dyn std::error::Error>> {
    let recipe_id = recipe_id("control.precedence")?;
    let first = variant_id("first")?;
    let second = variant_id("second")?;
    let mut recipe =
        StyleRecipe::new(StyleProperties::EMPTY.with_foreground(Color::rgb(2, 0, 0)));
    recipe.define_variant(
        first.clone(),
        StyleProperties::EMPTY.with_foreground(Color::rgb(3, 0, 0)),
    )?;
    recipe.define_variant(
        second.clone(),
        StyleProperties::EMPTY.with_foreground(Color::rgb(4, 0, 0)),
    )?;
    for (state, value) in [
        (StyleInteractionState::Hover, 5),
        (StyleInteractionState::Focus, 6),
        (StyleInteractionState::Active, 7),
        (StyleInteractionState::Disabled, 8),
    ] {
        recipe.define_interaction(
            state,
            StyleProperties::EMPTY.with_foreground(Color::rgb(value, 0, 0)),
        )?;
    }
    let mut theme = StyleTheme::new(StyleTokens::new());
    theme.define_recipe(recipe_id.clone(), recipe)?;
    let environment = StyleEnvironment::new(theme)
        .with_framework_defaults(StyleProperties::EMPTY.with_foreground(Color::rgb(1, 0, 0)));

    assert_foreground_resolution(
        &StyleIntent::EMPTY,
        &environment,
        StyleInteractionFacts::NONE,
        Color::rgb(1, 0, 0),
        StyleResolutionLayer::FrameworkDefault,
    );

    let recipe_intent = StyleIntent::EMPTY.with_recipe(recipe_id.clone());
    assert_foreground_resolution(
        &recipe_intent,
        &environment,
        StyleInteractionFacts::NONE,
        Color::rgb(2, 0, 0),
        StyleResolutionLayer::ThemeRecipe(recipe_id.clone()),
    );

    let first_variant_intent = recipe_intent.clone().with_variant(first.clone());
    assert_foreground_resolution(
        &first_variant_intent,
        &environment,
        StyleInteractionFacts::NONE,
        Color::rgb(3, 0, 0),
        StyleResolutionLayer::Variant(first.clone()),
    );

    let ordered_variants_intent = first_variant_intent.with_variant(second.clone());
    assert_foreground_resolution(
        &ordered_variants_intent,
        &environment,
        StyleInteractionFacts::NONE,
        Color::rgb(4, 0, 0),
        StyleResolutionLayer::Variant(second),
    );

    let hover = StyleInteractionFacts::NONE.with(StyleInteractionState::Hover, true);
    assert_foreground_resolution(
        &ordered_variants_intent,
        &environment,
        hover,
        Color::rgb(5, 0, 0),
        StyleResolutionLayer::Interaction(StyleInteractionState::Hover),
    );

    let focus = hover.with(StyleInteractionState::Focus, true);
    assert_foreground_resolution(
        &ordered_variants_intent,
        &environment,
        focus,
        Color::rgb(6, 0, 0),
        StyleResolutionLayer::Interaction(StyleInteractionState::Focus),
    );

    let active = focus.with(StyleInteractionState::Active, true);
    assert_foreground_resolution(
        &ordered_variants_intent,
        &environment,
        active,
        Color::rgb(7, 0, 0),
        StyleResolutionLayer::Interaction(StyleInteractionState::Active),
    );

    let disabled = active.with(StyleInteractionState::Disabled, true);
    assert_foreground_resolution(
        &ordered_variants_intent,
        &environment,
        disabled,
        Color::rgb(8, 0, 0),
        StyleResolutionLayer::Interaction(StyleInteractionState::Disabled),
    );

    let authored = ordered_variants_intent.with_foreground(Color::rgb(9, 0, 0));
    assert_foreground_resolution(
        &authored,
        &environment,
        disabled,
        Color::rgb(9, 0, 0),
        StyleResolutionLayer::AuthoredOverride,
    );

    let preference_environment = environment
        .with_preferences(StylePreferences::new(true, false))
        .with_preference_policy(
            StylePreferencePolicy::new()
                .with_high_contrast(StyleProperties::EMPTY.with_foreground(Color::rgb(10, 0, 0))),
        );
    assert_foreground_resolution(
        &authored,
        &preference_environment,
        disabled,
        Color::rgb(10, 0, 0),
        StyleResolutionLayer::Preference(StylePreferenceKind::HighContrast),
    );
    Ok(())
}

#[test]
fn ordered_variants_and_framework_interaction_order_are_stable()
-> Result<(), Box<dyn std::error::Error>> {
    let recipe_id = recipe_id("control.button")?;
    let first = variant_id("first")?;
    let second = variant_id("second")?;
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
    let intent = StyleIntent::EMPTY
        .with_recipe(recipe_id)
        .with_variant(first)
        .with_variant(second);
    let interaction = StyleInteractionFacts::NONE
        .with(StyleInteractionState::Hover, true)
        .with(StyleInteractionState::Active, true);
    let resolution = resolve_style_in_environment(&intent, &environment, interaction, None);
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
fn missing_higher_precedence_token_masks_lower_value() -> Result<(), Box<dyn std::error::Error>> {
    let recipe_id = recipe_id("control.button")?;
    let mut theme = StyleTheme::new(StyleTokens::new());
    theme.define_recipe(
        recipe_id.clone(),
        StyleRecipe::new(StyleProperties::EMPTY.with_foreground(Color::rgb(12, 12, 12))),
    )?;
    let missing = ColorToken::new(TokenId::new("color.missing")?);
    let environment = StyleEnvironment::new(theme);
    let intent = StyleIntent::EMPTY
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
        &StyleIntent::EMPTY,
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
