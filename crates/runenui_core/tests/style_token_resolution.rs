use runenui_core::{
    Color, ComputedStyle, DuplicateTokenDefinition, StyleIntent, StyleTokens, color_token,
    resolve_style,
};

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
fn style_resolution_preserves_provenance() -> Result<(), DuplicateTokenDefinition> {
    let token = color_token!("color.text.primary");
    let mut tokens = StyleTokens::new();
    tokens.define_color(token.clone(), Color::WHITE)?;
    let resolution = resolve_style(&StyleIntent::EMPTY.with_foreground(token), &tokens);
    assert_eq!(
        resolution.computed_style(),
        ComputedStyle::EMPTY.with_foreground(Color::WHITE)
    );
    assert!(resolution.is_fully_resolved());
    Ok(())
}
