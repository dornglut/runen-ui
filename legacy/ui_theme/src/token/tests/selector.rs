//! Theme token selector tests.

use super::*;

#[test]
fn token_resolution_rejects_duplicate_selectors() {
    let selector = ThemeTokenSelector {
        mode: Some("compact".into()),
        ..ThemeTokenSelector::default()
    };
    let graph = ThemeTokenGraph::new(vec![
        token_value(
            "spacing.sm",
            ThemeTokenLayer::Mode,
            ThemeTokenValue::Number(6.0),
            "compact-a",
        )
        .with_selector(selector.clone()),
        token_value(
            "spacing.sm",
            ThemeTokenLayer::Mode,
            ThemeTokenValue::Number(8.0),
            "compact-b",
        )
        .with_selector(selector),
    ]);
    let mut request = ThemeTokenResolveRequest::activate("editor.workbench");
    request.mode = Some("compact".into());

    let report = resolve_theme_tokens(&graph, &request);

    assert!(report.has_errors());
    let diagnostic = &report.diagnostics[0];
    assert_eq!(diagnostic.code, "ui.theme.selector.duplicate");
    assert_eq!(
        diagnostic
            .winning_source
            .as_ref()
            .map(ThemeTokenSourceId::as_str),
        Some("compact-a")
    );
    assert_eq!(
        diagnostic
            .losing_sources
            .iter()
            .map(ThemeTokenSourceId::as_str)
            .collect::<Vec<_>>(),
        ["compact-b"]
    );
}

#[test]
fn component_selectors_match_requested_component() {
    let graph = ThemeTokenGraph::new(vec![
        token_value(
            "spacing.sm",
            ThemeTokenLayer::Component,
            ThemeTokenValue::Number(6.0),
            "button",
        )
        .with_selector(ThemeTokenSelector {
            component: Some("button".into()),
            ..ThemeTokenSelector::default()
        }),
        token_value(
            "spacing.sm",
            ThemeTokenLayer::Component,
            ThemeTokenValue::Number(10.0),
            "panel",
        )
        .with_selector(ThemeTokenSelector {
            component: Some("panel".into()),
            ..ThemeTokenSelector::default()
        }),
    ]);
    let mut request = ThemeTokenResolveRequest::activate("editor.workbench");
    request.component = Some("button".into());

    let report = resolve_theme_tokens(&graph, &request);

    assert!(!report.has_errors(), "{:?}", report.diagnostics);
    assert_eq!(
        report
            .tokens
            .get(&ThemeTokenId::from("spacing.sm"))
            .map(|token| &token.value),
        Some(&ThemeTokenValue::Number(6.0))
    );
}

#[test]
fn token_resolution_rejects_incompatible_state_mode_selectors() {
    let graph = ThemeTokenGraph::new(vec![
        token_value(
            "spacing.sm",
            ThemeTokenLayer::State,
            ThemeTokenValue::Number(6.0),
            "hover-compact",
        )
        .with_selector(ThemeTokenSelector {
            state: Some("hover".into()),
            mode: Some("compact".into()),
            ..ThemeTokenSelector::default()
        }),
    ]);
    let mut request = ThemeTokenResolveRequest::activate("editor.workbench");
    request.state = Some("hover".into());
    request.mode = Some("compact".into());

    let report = resolve_theme_tokens(&graph, &request);

    assert!(report.has_errors());
    assert_eq!(report.diagnostics[0].code, "ui.theme.selector.incompatible");
}
