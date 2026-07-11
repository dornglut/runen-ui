use runenui_core::{
    Color, ColorToken, ColorValue, ComputedStyle, EdgeInsets, Length, Radius, RadiusToken,
    RadiusValue, SpacingToken, StyleFieldProvenance, StyleIntent, StyleProvenance, StyleTokens,
    UnresolvedStyleToken, resolve_literal_style, resolve_style,
};

#[test]
fn empty_intent_records_absent_provenance() {
    let resolution = resolve_style(&StyleIntent::EMPTY, &StyleTokens::new());

    assert_eq!(resolution.computed_style(), ComputedStyle::EMPTY);
    assert_eq!(resolution.provenance(), &StyleProvenance::EMPTY);
    assert_eq!(resolution.unresolved_tokens(), []);
    assert!(resolution.is_fully_resolved());
}

#[test]
fn literal_style_records_literal_provenance_for_each_field() {
    let padding = EdgeInsets::all(Length::px(8.0));
    let radius = Radius::all(Length::px(4.0));
    let intent = StyleIntent::EMPTY
        .with_foreground(Color::WHITE)
        .with_background(Color::BLACK)
        .with_padding(padding)
        .with_radius(radius);

    let resolution = resolve_literal_style(&intent);

    assert_eq!(
        resolution.computed_style(),
        ComputedStyle::EMPTY
            .with_foreground(Color::WHITE)
            .with_background(Color::BLACK)
            .with_padding(padding)
            .with_radius(radius)
    );
    assert_eq!(
        resolution.provenance(),
        &StyleProvenance::new(
            StyleFieldProvenance::Literal,
            StyleFieldProvenance::Literal,
            StyleFieldProvenance::Literal,
            StyleFieldProvenance::Literal,
        )
    );
    assert_eq!(resolution.unresolved_tokens(), []);
}

#[test]
fn literal_resolution_reports_missing_token_provenance_without_token_values() {
    let intent = StyleIntent::EMPTY
        .with_foreground(ColorValue::token("color.text.primary"))
        .with_background(ColorToken::new("color.surface"))
        .with_padding(SpacingToken::new("space.2"))
        .with_radius(RadiusValue::token("radius.control"));

    let resolution = resolve_literal_style(&intent);

    assert_eq!(resolution.computed_style(), ComputedStyle::EMPTY);
    assert!(!resolution.is_fully_resolved());
    assert_eq!(
        resolution.provenance(),
        &StyleProvenance::new(
            StyleFieldProvenance::MissingToken(ColorToken::new("color.text.primary")),
            StyleFieldProvenance::MissingToken(ColorToken::new("color.surface")),
            StyleFieldProvenance::MissingToken(SpacingToken::new("space.2")),
            StyleFieldProvenance::MissingToken(RadiusToken::new("radius.control")),
        )
    );
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
fn token_backed_intent_records_resolved_token_provenance() {
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
    assert_eq!(
        resolution.provenance(),
        &StyleProvenance::new(
            StyleFieldProvenance::ResolvedToken(ColorToken::new("color.text.primary")),
            StyleFieldProvenance::ResolvedToken(ColorToken::new("color.surface")),
            StyleFieldProvenance::ResolvedToken(SpacingToken::new("space.2")),
            StyleFieldProvenance::ResolvedToken(RadiusToken::new("radius.control")),
        )
    );
    assert!(resolution.is_fully_resolved());
    assert_eq!(resolution.unresolved_tokens(), []);
}

#[test]
fn missing_token_map_entries_record_missing_token_provenance() {
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
        resolution.provenance(),
        &StyleProvenance::new(
            StyleFieldProvenance::ResolvedToken(ColorToken::new("color.text.primary")),
            StyleFieldProvenance::MissingToken(ColorToken::new("color.surface")),
            StyleFieldProvenance::Absent,
            StyleFieldProvenance::Absent,
        )
    );
    assert_eq!(
        resolution.unresolved_tokens(),
        [UnresolvedStyleToken::Background(ColorToken::new(
            "color.surface"
        ))]
    );
}

#[test]
fn mixed_intent_records_literal_resolved_missing_and_absent_provenance() {
    let tokens = StyleTokens::new().with_color("color.surface", Color::BLACK);
    let intent = StyleIntent::EMPTY
        .with_foreground(Color::WHITE)
        .with_background(ColorToken::new("color.surface"))
        .with_radius(RadiusToken::new("radius.control"));

    let resolution = resolve_style(&intent, &tokens);

    assert_eq!(
        resolution.computed_style(),
        ComputedStyle::EMPTY
            .with_foreground(Color::WHITE)
            .with_background(Color::BLACK)
    );
    assert_eq!(
        resolution.provenance(),
        &StyleProvenance::new(
            StyleFieldProvenance::Literal,
            StyleFieldProvenance::ResolvedToken(ColorToken::new("color.surface")),
            StyleFieldProvenance::Absent,
            StyleFieldProvenance::MissingToken(RadiusToken::new("radius.control")),
        )
    );
    assert_eq!(
        resolution.unresolved_tokens(),
        [UnresolvedStyleToken::Radius(RadiusToken::new(
            "radius.control"
        ))]
    );
}
