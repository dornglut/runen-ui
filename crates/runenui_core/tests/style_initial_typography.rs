use runenui_core::{
    FontFamily, GenericFontFamily, LogicalLength, StyleEnvironment, StyleFieldProvenance,
    StyleIntent, StyleInteractionFacts, StyleProperties, StyleResolutionLayer, StyleTheme,
    StyleTokens, Typography, resolve_style_in_environment,
};

fn typography(size: f32) -> Result<Typography, Box<dyn std::error::Error>> {
    Ok(Typography::new(
        FontFamily::generic(GenericFontFamily::SansSerif),
        LogicalLength::new(size)?,
    ))
}

#[test]
fn root_style_uses_canonical_initial_typography() {
    let resolution = resolve_style_in_environment(
        &StyleIntent::EMPTY,
        &StyleEnvironment::default(),
        StyleInteractionFacts::NONE,
        None,
    );
    let initial = Typography::default();

    assert_eq!(resolution.computed_style().typography(), Some(&initial));
    assert_eq!(
        resolution.provenance().typography(),
        &StyleFieldProvenance::Literal
    );
    assert_eq!(
        resolution.provenance().typography_layer(),
        Some(&StyleResolutionLayer::Initial)
    );
}

#[test]
fn framework_typography_overrides_only_the_initial_value() -> Result<(), Box<dyn std::error::Error>>
{
    let framework = typography(12.0)?;
    let environment = StyleEnvironment::new(StyleTheme::new(StyleTokens::new()))
        .with_framework_defaults(StyleProperties::EMPTY.with_typography(framework.clone()));
    let resolution = resolve_style_in_environment(
        &StyleIntent::EMPTY,
        &environment,
        StyleInteractionFacts::NONE,
        None,
    );

    assert_eq!(resolution.computed_style().typography(), Some(&framework));
    assert_eq!(
        resolution.provenance().typography_layer(),
        Some(&StyleResolutionLayer::FrameworkDefault)
    );
    Ok(())
}
