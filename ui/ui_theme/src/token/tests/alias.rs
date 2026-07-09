//! Theme token alias tests.

use super::*;

#[test]
fn token_resolution_rejects_alias_cycles() {
    let graph = ThemeTokenGraph::new(vec![
        ThemeTokenDeclaration::alias(
            "color.a",
            ThemeTokenFamily::Color,
            ThemeTokenLayer::Semantic,
            "color.b",
            "base",
        ),
        ThemeTokenDeclaration::alias(
            "color.b",
            ThemeTokenFamily::Color,
            ThemeTokenLayer::Semantic,
            "color.a",
            "base",
        ),
    ]);

    let report = resolve_theme_tokens(
        &graph,
        &ThemeTokenResolveRequest::activate("editor.workbench"),
    );

    assert!(report.has_errors());
    assert!(
        report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "ui.theme.alias.cycle"),
        "{:?}",
        report.diagnostics
    );
}

#[test]
fn token_resolution_rejects_missing_aliases() {
    let graph = ThemeTokenGraph::new(vec![ThemeTokenDeclaration::alias(
        "color.accent",
        ThemeTokenFamily::Color,
        ThemeTokenLayer::Semantic,
        "color.brand",
        "base",
    )]);

    let report = resolve_theme_tokens(
        &graph,
        &ThemeTokenResolveRequest::activate("editor.workbench"),
    );

    assert!(report.has_errors());
    assert_eq!(report.diagnostics[0].code, "ui.theme.alias.missing");
}

#[test]
fn token_resolution_rejects_alias_family_mismatches() {
    let graph = ThemeTokenGraph::new(vec![
        token_value(
            "spacing.sm",
            ThemeTokenLayer::Primitive,
            ThemeTokenValue::Number(4.0),
            "base",
        ),
        ThemeTokenDeclaration::alias(
            "color.accent",
            ThemeTokenFamily::Color,
            ThemeTokenLayer::Semantic,
            "spacing.sm",
            "semantic",
        ),
    ]);

    let report = resolve_theme_tokens(
        &graph,
        &ThemeTokenResolveRequest::activate("editor.workbench"),
    );

    assert!(report.has_errors());
    assert!(
        report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "ui.theme.alias.family_mismatch"),
        "{:?}",
        report.diagnostics
    );
}
