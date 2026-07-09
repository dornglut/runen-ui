//! Theme token activation tests.

use super::*;

#[test]
fn state_variants_preview_across_target_profiles() {
    let graph = ThemeTokenGraph::new(vec![
        token_value(
            "spacing.sm",
            ThemeTokenLayer::Primitive,
            ThemeTokenValue::Number(4.0),
            "base",
        )
        .for_target_profiles(["editor.workbench".into(), "game.runtime".into()]),
        token_value(
            "spacing.sm",
            ThemeTokenLayer::State,
            ThemeTokenValue::Number(6.0),
            "hover",
        )
        .for_target_profiles(["editor.workbench".into(), "game.runtime".into()])
        .with_selector(ThemeTokenSelector {
            state: Some("hover".into()),
            ..ThemeTokenSelector::default()
        }),
    ]);
    let mut editor_request = ThemeTokenResolveRequest::preview("editor.workbench");
    editor_request.state = Some("hover".into());
    let mut runtime_request = ThemeTokenResolveRequest::preview("game.runtime");
    runtime_request.state = Some("hover".into());

    let editor_report = resolve_theme_tokens(&graph, &editor_request);
    let runtime_report = resolve_theme_tokens(&graph, &runtime_request);

    assert!(
        !editor_report.has_errors(),
        "{:?}",
        editor_report.diagnostics
    );
    assert!(
        !runtime_report.has_errors(),
        "{:?}",
        runtime_report.diagnostics
    );
    assert_eq!(
        editor_report
            .tokens
            .get(&ThemeTokenId::from("spacing.sm"))
            .map(|token| &token.value),
        Some(&ThemeTokenValue::Number(6.0))
    );
    assert_eq!(
        runtime_report
            .tokens
            .get(&ThemeTokenId::from("spacing.sm"))
            .map(|token| &token.value),
        Some(&ThemeTokenValue::Number(6.0))
    );
}

#[test]
fn token_resolution_rejects_unsupported_target_profiles() {
    let graph = ThemeTokenGraph::new(vec![
        token_value(
            "color.accent",
            ThemeTokenLayer::Primitive,
            ThemeTokenValue::Color(UiColor::new(0.0, 0.2, 1.0, 1.0)),
            "base",
        )
        .for_target_profiles(["editor.workbench".into()]),
    ]);

    let report = resolve_theme_tokens(&graph, &ThemeTokenResolveRequest::activate("game.runtime"));

    assert!(report.has_errors());
    assert_eq!(
        report.diagnostics[0].code,
        "ui.theme.target_profile.unsupported"
    );
}

#[test]
fn token_resolution_rejects_accessibility_conflicts() {
    let selector = ThemeTokenSelector {
        accessibility: Some("high_contrast".into()),
        ..ThemeTokenSelector::default()
    };
    let graph = ThemeTokenGraph::new(vec![
        token_value(
            "color.accent",
            ThemeTokenLayer::Accessibility,
            ThemeTokenValue::Color(UiColor::new(1.0, 1.0, 0.0, 1.0)),
            "a11y-a",
        )
        .with_selector(selector.clone()),
        token_value(
            "color.accent",
            ThemeTokenLayer::Accessibility,
            ThemeTokenValue::Color(UiColor::new(0.0, 1.0, 1.0, 1.0)),
            "a11y-b",
        )
        .with_selector(selector),
    ]);
    let mut request = ThemeTokenResolveRequest::activate("editor.workbench");
    request.accessibility = Some("high_contrast".into());

    let report = resolve_theme_tokens(&graph, &request);

    assert!(report.has_errors());
    assert_eq!(
        report.diagnostics[0].code,
        "ui.theme.accessibility.conflict"
    );
}

#[test]
fn token_resolution_rejects_preview_only_activation() {
    let graph = ThemeTokenGraph::new(vec![
        token_value(
            "color.accent",
            ThemeTokenLayer::Preview,
            ThemeTokenValue::Color(UiColor::new(1.0, 0.0, 0.0, 1.0)),
            "preview",
        )
        .preview_only(),
    ]);

    let report = resolve_theme_tokens(
        &graph,
        &ThemeTokenResolveRequest::activate("editor.workbench"),
    );

    assert!(report.has_errors());
    assert_eq!(
        report.diagnostics[0].code,
        "ui.theme.preview_only_activation"
    );
}
