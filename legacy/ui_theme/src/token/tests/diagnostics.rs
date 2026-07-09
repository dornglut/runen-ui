//! Theme token diagnostics tests.

use super::*;

#[test]
fn token_resolution_rejects_malformed_values_with_source_path() {
    let graph = ThemeTokenGraph::new(vec![
        ThemeTokenDeclaration::value(
            "color.accent",
            ThemeTokenFamily::Color,
            ThemeTokenLayer::Primitive,
            ThemeTokenValue::Color(UiColor::new(1.2, 0.0, 0.0, 1.0)),
            "base",
        )
        .with_source_path("themes/base.ron:12"),
    ]);

    let report = resolve_theme_tokens(
        &graph,
        &ThemeTokenResolveRequest::activate("editor.workbench"),
    );

    assert!(report.has_errors());
    let diagnostic = &report.diagnostics[0];
    assert_eq!(diagnostic.code, "ui.theme.token.malformed_value");
    assert_eq!(
        diagnostic
            .source_path
            .as_ref()
            .map(ThemeTokenSourcePath::as_str),
        Some("themes/base.ron:12")
    );
}
