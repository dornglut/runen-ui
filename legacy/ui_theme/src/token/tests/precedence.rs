//! Theme token precedence tests.

use super::*;

#[test]
fn token_resolution_applies_deterministic_layer_order() {
    let graph = ThemeTokenGraph::new(vec![
        token_value(
            "spacing.sm",
            ThemeTokenLayer::Primitive,
            ThemeTokenValue::Number(4.0),
            "base",
        ),
        token_value(
            "spacing.sm",
            ThemeTokenLayer::State,
            ThemeTokenValue::Number(6.0),
            "hover",
        )
        .with_selector(ThemeTokenSelector {
            state: Some("hover".into()),
            ..ThemeTokenSelector::default()
        }),
        token_value(
            "spacing.sm",
            ThemeTokenLayer::Mode,
            ThemeTokenValue::Number(8.0),
            "compact",
        )
        .with_selector(ThemeTokenSelector {
            mode: Some("compact".into()),
            ..ThemeTokenSelector::default()
        }),
    ]);
    let mut request = ThemeTokenResolveRequest::activate("editor.workbench");
    request.state = Some("hover".into());
    request.mode = Some("compact".into());

    let report = resolve_theme_tokens(&graph, &request);

    assert!(!report.has_errors(), "{:?}", report.diagnostics);
    let resolved = report
        .tokens
        .get(&ThemeTokenId::from("spacing.sm"))
        .unwrap();
    assert_eq!(resolved.value, ThemeTokenValue::Number(8.0));
    assert_eq!(resolved.winning_source.as_str(), "compact");
    assert_eq!(
        resolved
            .losing_sources
            .iter()
            .map(ThemeTokenSourceId::as_str)
            .collect::<Vec<_>>(),
        ["base", "hover"]
    );
}

#[test]
fn token_resolution_applies_full_precedence_order() {
    let graph = ThemeTokenGraph::new(vec![
        token_value(
            "spacing.sm",
            ThemeTokenLayer::Primitive,
            ThemeTokenValue::Number(2.0),
            "primitive",
        ),
        token_value(
            "spacing.sm",
            ThemeTokenLayer::Component,
            ThemeTokenValue::Number(4.0),
            "button",
        )
        .with_selector(ThemeTokenSelector {
            component: Some("button".into()),
            ..ThemeTokenSelector::default()
        }),
        token_value(
            "spacing.sm",
            ThemeTokenLayer::State,
            ThemeTokenValue::Number(6.0),
            "hover",
        )
        .with_selector(ThemeTokenSelector {
            state: Some("hover".into()),
            ..ThemeTokenSelector::default()
        }),
        token_value(
            "spacing.sm",
            ThemeTokenLayer::Mode,
            ThemeTokenValue::Number(8.0),
            "compact",
        )
        .with_selector(ThemeTokenSelector {
            mode: Some("compact".into()),
            ..ThemeTokenSelector::default()
        }),
        token_value(
            "spacing.sm",
            ThemeTokenLayer::Theme,
            ThemeTokenValue::Number(10.0),
            "theme",
        ),
        token_value(
            "spacing.sm",
            ThemeTokenLayer::Skin,
            ThemeTokenValue::Number(12.0),
            "skin",
        ),
        token_value(
            "spacing.sm",
            ThemeTokenLayer::Platform,
            ThemeTokenValue::Number(14.0),
            "desktop",
        )
        .with_selector(ThemeTokenSelector {
            platform: Some("desktop".into()),
            ..ThemeTokenSelector::default()
        }),
        token_value(
            "spacing.sm",
            ThemeTokenLayer::Accessibility,
            ThemeTokenValue::Number(16.0),
            "high-contrast",
        )
        .with_selector(ThemeTokenSelector {
            accessibility: Some("high_contrast".into()),
            ..ThemeTokenSelector::default()
        }),
    ]);
    let mut request = ThemeTokenResolveRequest::activate("editor.workbench");
    request.component = Some("button".into());
    request.state = Some("hover".into());
    request.mode = Some("compact".into());
    request.platform = Some("desktop".into());
    request.accessibility = Some("high_contrast".into());

    let report = resolve_theme_tokens(&graph, &request);

    assert!(!report.has_errors(), "{:?}", report.diagnostics);
    let resolved = report
        .tokens
        .get(&ThemeTokenId::from("spacing.sm"))
        .unwrap();
    assert_eq!(resolved.value, ThemeTokenValue::Number(16.0));
    assert_eq!(resolved.winning_source.as_str(), "high-contrast");
    assert_eq!(
        resolved
            .losing_sources
            .iter()
            .map(ThemeTokenSourceId::as_str)
            .collect::<Vec<_>>(),
        [
            "primitive",
            "button",
            "hover",
            "compact",
            "theme",
            "skin",
            "desktop"
        ]
    );
}
