//! Theme token packet tests.

use super::*;

#[test]
fn resolved_tokens_can_update_theme_token_packet() {
    let graph = ThemeTokenGraph::new(vec![token_value(
        "color.accent",
        ThemeTokenLayer::Primitive,
        ThemeTokenValue::Color(UiColor::new(0.1, 0.2, 0.3, 1.0)),
        "base",
    )]);

    let report = resolve_theme_tokens(
        &graph,
        &ThemeTokenResolveRequest::activate("editor.workbench"),
    );
    let formed = report.apply_to_theme_tokens(&ThemeTokens::default());

    assert_eq!(formed.accent, UiColor::new(0.1, 0.2, 0.3, 1.0));
}
