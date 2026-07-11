use runenui_core::{
    Color, ColorToken, ColorValue, ComputedStyle, EdgeInsets, Length, Radius, RadiusToken,
    RadiusValue, SpacingToken, SpacingValue, StyleIntent, StyleTokens, UnresolvedStyleToken,
    resolve_literal_style, resolve_style,
};

#[test]
fn literal_resolution_still_reports_unresolved_tokens_without_token_values() {
    let intent = StyleIntent::EMPTY
        .with_foreground(ColorValue::token("color.text.primary"))
        .with_background(ColorToken::new("color.surface"))
        .with_padding(SpacingToken::new("space.2"))
        .with_radius(RadiusValue::token("radius.control"));

    let resolution = resolve_literal_style(&intent);

    assert_eq!(resolution.computed_style(), ComputedStyle::EMPTY);
    assert!(!resolution.is_fully_resolved());
    assert_eq!(
        resolution.unresolved_tokens(),
        [
            UnresolvedStyleToken::Foreground(ColorToken::new("color.text.primary")),
            UnresolvedStyleToken::Background(ColorToken::new("color.surface")),
            UnresolvedStyleToken::Padding(SpacingToken::new("space.2")),
            UnresolvedStyleToken::Radius(RadiusToken::new("radius.control")),
        ]
    );
}

#[test]
fn token_backed_intent_resolves_from_token_map() {
    let padding = EdgeInsets::all(Length::px(8.0));
    let radius = Radius::all(Length::px(4.0));
    let tokens = StyleTokens::new()
        .with_color("color.text.primary", Color::WHITE)
        .with_color("color.surface", Color::BLACK)
        .with_spacing("space.2", padding)
        .with_radius("radius.control", radius);
    let intent = StyleIntent::EMPTY
        .with_foreground(ColorToken::new("color.text.primary"))
        .with_background(ColorToken::new("color.surface"))
        .with_padding(SpacingToken::new("space.2"))
        .with_radius(RadiusToken::new("radius.control"));

    let resolution = resolve_style(&intent, &tokens);

    assert_eq!(
        resolution.computed_style(),
        ComputedStyle::EMPTY
            .with_foreground(Color::WHITE)
            .with_background(Color::BLACK)
            .with_padding(padding)
            .with_radius(radius)
    );
    assert!(resolution.is_fully_resolved());
    assert_eq!(resolution.unresolved_tokens(), []);
}

#[test]
fn missing_token_map_entries_remain_unresolved() {
    let tokens = StyleTokens::new().with_color("color.text.primary", Color::WHITE);
    let intent = StyleIntent::EMPTY
        .with_foreground(ColorToken::new("color.text.primary"))
        .with_background(ColorToken::new("color.surface"));

    let resolution = resolve_style(&intent, &tokens);

    assert_eq!(
        resolution.computed_style(),
        ComputedStyle::EMPTY.with_foreground(Color::WHITE)
    );
    assert_eq!(
        resolution.unresolved_tokens(),
        [UnresolvedStyleToken::Background(ColorToken::new(
            "color.surface"
        ))]
    );
}

#[test]
fn mixed_intent_resolves_literals_tokens_and_reports_missing_tokens() {
    let padding = EdgeInsets::all(Length::px(12.0));
    let tokens = StyleTokens::new().with_color("color.surface", Color::BLACK);
    let intent = StyleIntent::EMPTY
        .with_foreground(Color::WHITE)
        .with_background(ColorToken::new("color.surface"))
        .with_padding(SpacingValue::literal(padding))
        .with_radius(RadiusToken::new("radius.control"));

    let resolution = resolve_style(&intent, &tokens);

    assert_eq!(
        resolution.computed_style(),
        ComputedStyle::EMPTY
            .with_foreground(Color::WHITE)
            .with_background(Color::BLACK)
            .with_padding(padding)
    );
    assert_eq!(
        resolution.unresolved_tokens(),
        [UnresolvedStyleToken::Radius(RadiusToken::new(
            "radius.control"
        ))]
    );
}
