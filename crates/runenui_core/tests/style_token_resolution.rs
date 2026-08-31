use std::collections::{BTreeMap, HashMap, hash_map::DefaultHasher};
use std::hash::{Hash, Hasher};

use runenui_core::{
    Color, ColorToken, ComputedStyle, DuplicateTokenDefinition, EdgeInsets, FontFamily,
    GenericFontFamily, IdentifierError, LogicalLength, Radius, RadiusToken, SpacingToken,
    StyleEnvironment, StyleIntent, StyleInteractionFacts, StyleTokens, TokenId, Typography,
    TypographyToken, UnresolvedStyleToken, color_token, radius_token,
    resolve_style_in_environment, spacing_token, token_id, typography_token,
};

fn hash(value: &impl Hash) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

fn typography(size: u8) -> Typography {
    Typography::new(
        FontFamily::generic(GenericFontFamily::SansSerif),
        LogicalLength::from(size),
    )
}

#[test]
fn token_definitions_do_not_silently_overwrite() -> Result<(), Box<dyn std::error::Error>> {
    let token = color_token!("color.text.primary");
    let mut tokens = StyleTokens::new();
    tokens.define_color(token.clone(), Color::WHITE)?;
    let duplicate = match tokens.define_color(token.clone(), Color::BLACK) {
        Ok(()) => return Err("expected duplicate token error".into()),
        Err(error) => error,
    };
    assert_eq!(duplicate.token().as_str(), "color.text.primary");
    assert_eq!(tokens.color(&token), Some(Color::WHITE));
    Ok(())
}

#[test]
fn token_identity_and_lookup_are_textual_across_constructor_forms()
-> Result<(), Box<dyn std::error::Error>> {
    let dynamic = TokenId::new("color.primary")?;
    let static_id = TokenId::from_static("color.primary")?;
    assert_eq!(dynamic, static_id);
    assert_eq!(dynamic.cmp(&static_id), std::cmp::Ordering::Equal);
    assert_eq!(hash(&dynamic), hash(&static_id));
    let mut ordered = BTreeMap::new();
    ordered.insert(static_id.clone(), "static");
    assert_eq!(ordered.get(&dynamic), Some(&"static"));
    let mut hashed = HashMap::new();
    hashed.insert(dynamic.clone(), "dynamic");
    assert_eq!(hashed.get(&static_id), Some(&"dynamic"));

    let dynamic_color = ColorToken::new(dynamic);
    let static_color = color_token!("color.primary");
    assert_eq!(dynamic_color, static_color);
    assert_eq!(
        SpacingToken::new(TokenId::new("space.content")?),
        spacing_token!("space.content")
    );
    assert_eq!(
        RadiusToken::new(TokenId::new("radius.control")?),
        radius_token!("radius.control")
    );
    assert_eq!(
        TypographyToken::new(TokenId::new("type.body")?),
        typography_token!("type.body")
    );

    let mut tokens = StyleTokens::new();
    tokens.define_color(dynamic_color, Color::WHITE)?;
    assert_eq!(tokens.color(&static_color), Some(Color::WHITE));

    let mut reverse = StyleTokens::new();
    reverse.define_spacing(
        spacing_token!("space.content"),
        EdgeInsets::all(LogicalLength::new(4.0)?),
    )?;
    assert_eq!(
        reverse.spacing(&SpacingToken::new(TokenId::new("space.content")?)),
        Some(EdgeInsets::all(LogicalLength::new(4.0)?))
    );

    let mut typography_tokens = StyleTokens::new();
    let dynamic_typography = TypographyToken::new(TokenId::new("type.body")?);
    typography_tokens.define_typography(dynamic_typography, typography(16))?;
    assert_eq!(
        typography_tokens.typography(&typography_token!("type.body")),
        Some(&typography(16))
    );

    let mut duplicate = StyleTokens::new();
    duplicate.define_radius(
        RadiusToken::new(TokenId::new("radius.control")?),
        Radius::ZERO,
    )?;
    let error = match duplicate.define_radius(radius_token!("radius.control"), Radius::ZERO) {
        Ok(()) => return Err("mixed storage duplicate was accepted".into()),
        Err(error) => error,
    };
    assert_eq!(error.token(), &token_id!("radius.control"));
    Ok(())
}

#[test]
fn token_ids_use_the_unicode_identifier_grammar() {
    for (value, expected) in [
        ("\u{00A0}", IdentifierError::WhitespaceOnly),
        ("\u{2003}", IdentifierError::WhitespaceOnly),
        ("\u{00A0}name", IdentifierError::SurroundingWhitespace),
        ("name\u{2003}", IdentifierError::SurroundingWhitespace),
        ("name\u{0085}value", IdentifierError::ControlCharacter),
    ] {
        assert_eq!(TokenId::new(value), Err(expected));
        assert_eq!(TokenId::from_static(value), Err(expected));
    }
    for value in ["fenster.öffnen", "画面.開始", "контрол.кнопка"] {
        assert_eq!(TokenId::new(value).as_ref().map(TokenId::as_str), Ok(value));
        assert_eq!(
            TokenId::from_static(value).as_ref().map(TokenId::as_str),
            Ok(value)
        );
    }
    assert_eq!(token_id!("画面.開始").as_str(), "画面.開始");
}

#[test]
fn style_resolution_preserves_provenance() -> Result<(), DuplicateTokenDefinition> {
    let token = color_token!("color.text.primary");
    let mut tokens = StyleTokens::new();
    tokens.define_color(token.clone(), Color::WHITE)?;
    let environment = StyleEnvironment::from_tokens(tokens);
    let resolution = resolve_style_in_environment(
        &StyleIntent::EMPTY.with_foreground(token),
        &environment,
        StyleInteractionFacts::NONE,
        None,
    );
    assert_eq!(
        resolution.computed_style(),
        &ComputedStyle::EMPTY.with_foreground(Color::WHITE)
    );
    assert!(resolution.is_fully_resolved());
    Ok(())
}

#[test]
fn typography_tokens_resolve_through_the_production_style_environment()
-> Result<(), Box<dyn std::error::Error>> {
    let token = typography_token!("type.body");
    let expected = typography(18);
    let mut tokens = StyleTokens::new();
    tokens.define_typography(token.clone(), expected.clone())?;
    let environment = StyleEnvironment::from_tokens(tokens);
    let resolution = resolve_style_in_environment(
        &StyleIntent::EMPTY.with_typography(token),
        &environment,
        StyleInteractionFacts::NONE,
        None,
    );

    assert_eq!(resolution.computed_style().typography(), Some(&expected));
    assert!(resolution.is_fully_resolved());
    Ok(())
}

#[test]
fn missing_tokens_diagnose_every_current_property() {
    let foreground = color_token!("color.missing.foreground");
    let background = color_token!("color.missing.background");
    let padding = spacing_token!("space.missing.padding");
    let radius = radius_token!("radius.missing");
    let typography = typography_token!("type.missing");
    let intent = StyleIntent::EMPTY
        .with_foreground(foreground.clone())
        .with_background(background.clone())
        .with_padding(padding.clone())
        .with_radius(radius.clone())
        .with_typography(typography.clone());
    let environment = StyleEnvironment::default();
    let resolution =
        resolve_style_in_environment(&intent, &environment, StyleInteractionFacts::NONE, None);

    assert_eq!(resolution.computed_style(), &ComputedStyle::EMPTY);
    assert_eq!(
        resolution.unresolved_tokens(),
        &[
            UnresolvedStyleToken::Foreground(foreground),
            UnresolvedStyleToken::Background(background),
            UnresolvedStyleToken::Padding(padding),
            UnresolvedStyleToken::Radius(radius),
            UnresolvedStyleToken::Typography(typography),
        ]
    );
    assert!(!resolution.is_fully_resolved());
}
