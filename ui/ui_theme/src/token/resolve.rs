//! Theme token graph resolution.

use std::collections::BTreeMap;

use super::{
    ResolvedThemeToken, ThemeTokenActivationImpact, ThemeTokenActivationMode,
    ThemeTokenDeclaration, ThemeTokenDiagnostic, ThemeTokenGraph, ThemeTokenId, ThemeTokenLayer,
    ThemeTokenResolutionReport, ThemeTokenResolveRequest, ThemeTokenSelector, ThemeTokenSourceId,
    ThemeTokenValueSource,
    diagnostics::{error, error_for_declaration},
};

pub fn resolve_theme_tokens(
    graph: &ThemeTokenGraph,
    request: &ThemeTokenResolveRequest,
) -> ThemeTokenResolutionReport {
    let mut diagnostics = Vec::new();
    let mut active = BTreeMap::<ThemeTokenId, ActiveThemeToken>::new();
    let mut active_selectors = BTreeMap::<
        (ThemeTokenId, ThemeTokenLayer, ThemeTokenSelector),
        ThemeTokenDeclaration,
    >::new();
    let mut declarations = graph.declarations.clone();
    declarations.sort_by(|left, right| {
        left.layer
            .cmp(&right.layer)
            .then_with(|| left.id.cmp(&right.id))
            .then_with(|| left.source.cmp(&right.source))
    });

    for declaration in declarations {
        if (declaration.preview_only || declaration.layer == ThemeTokenLayer::Preview)
            && request.activation == ThemeTokenActivationMode::Activate
        {
            let diagnostic = error_for_declaration(
                "ui.theme.preview_only_activation",
                format!(
                    "preview-only token '{}' cannot be activated",
                    declaration.id
                ),
                &declaration,
                request,
                "convert the preview override into a deterministic theme package before activation",
            )
            .with_activation_impact(ThemeTokenActivationImpact::PreviewOnly);
            diagnostics.push(diagnostic);
            continue;
        }
        if !declaration.supports_profile(&request.target_profile) {
            diagnostics.push(error_for_declaration(
                "ui.theme.target_profile.unsupported",
                format!(
                    "token '{}' does not support target profile '{}'",
                    declaration.id, request.target_profile
                ),
                &declaration,
                request,
                "add an explicit compatible token declaration for the requested target profile",
            ));
            continue;
        }
        if let Some(diagnostic) = validate_selector_compatibility(&declaration, request) {
            diagnostics.push(diagnostic);
            continue;
        }
        if !declaration.selector.matches(request) {
            continue;
        }
        if declaration.layer == ThemeTokenLayer::Accessibility
            && active
                .get(&declaration.id)
                .is_some_and(|token| token.declaration.layer == ThemeTokenLayer::Accessibility)
        {
            diagnostics.push(error_for_declaration(
                "ui.theme.accessibility.conflict",
                format!(
                    "multiple accessibility overrides resolve token '{}'",
                    declaration.id
                ),
                &declaration,
                request,
                "make accessibility overrides mutually exclusive or assign a deterministic layer",
            ));
            continue;
        }
        if let ThemeTokenValueSource::Value(value) = &declaration.value
            && let Some(code) = value.validation_error_code(declaration.family)
        {
            diagnostics.push(error_for_declaration(
                code,
                format!(
                    "token '{}' value does not match family {:?}",
                    declaration.id, declaration.family
                ),
                &declaration,
                request,
                "use a token value compatible with the declared family",
            ));
            continue;
        }
        let selector_key = (
            declaration.id.clone(),
            declaration.layer,
            declaration.selector.clone(),
        );
        if let Some(previous) = active_selectors.insert(selector_key, declaration.clone()) {
            diagnostics.push(
                error_for_declaration(
                    "ui.theme.selector.duplicate",
                    format!(
                        "token '{}' has duplicate selectors at layer {:?}",
                        declaration.id, declaration.layer
                    ),
                    &declaration,
                    request,
                    "make duplicate selectors mutually exclusive or move one declaration to a higher precedence layer",
                )
                .with_sources(Some(previous.source), vec![declaration.source.clone()]),
            );
            continue;
        }
        let losing_sources = active
            .remove(&declaration.id)
            .map(|previous| {
                let mut sources = previous.losing_sources;
                sources.push(previous.declaration.source);
                sources
            })
            .unwrap_or_default();

        active.insert(
            declaration.id.clone(),
            ActiveThemeToken {
                losing_sources,
                declaration,
            },
        );
    }

    let mut resolved = BTreeMap::new();
    for id in active.keys() {
        let mut chain = Vec::new();
        if let Some(token) = resolve_token(id, &active, request, &mut chain, &mut diagnostics) {
            resolved.insert(id.clone(), token);
        }
    }

    ThemeTokenResolutionReport {
        tokens: resolved,
        diagnostics,
    }
}

fn resolve_token(
    id: &ThemeTokenId,
    active: &BTreeMap<ThemeTokenId, ActiveThemeToken>,
    request: &ThemeTokenResolveRequest,
    chain: &mut Vec<ThemeTokenId>,
    diagnostics: &mut Vec<ThemeTokenDiagnostic>,
) -> Option<ResolvedThemeToken> {
    if chain.contains(id) {
        let active_token = active.get(id);
        let alias_path = chain
            .iter()
            .cloned()
            .chain(std::iter::once(id.clone()))
            .collect();
        let diagnostic = match active_token {
            Some(active_token) => error_for_declaration(
                "ui.theme.alias.cycle",
                format!("theme token alias cycle includes '{id}'"),
                &active_token.declaration,
                request,
                "break the alias cycle with a concrete token value",
            ),
            None => error(
                "ui.theme.alias.cycle",
                format!("theme token alias cycle includes '{id}'"),
                Some(id.clone()),
                request,
                None,
                "break the alias cycle with a concrete token value",
            ),
        };
        diagnostics.push(diagnostic.with_alias_path(alias_path));
        return None;
    }
    let active_token = active.get(id)?;
    match &active_token.declaration.value {
        ThemeTokenValueSource::Value(value) => {
            if let Some(code) = value.validation_error_code(active_token.declaration.family) {
                diagnostics.push(error_for_declaration(
                    code,
                    format!(
                        "token '{}' value does not match family {:?}",
                        id, active_token.declaration.family
                    ),
                    &active_token.declaration,
                    request,
                    "use a token value compatible with the declared family",
                ));
                return None;
            }
            Some(ResolvedThemeToken {
                id: id.clone(),
                family: active_token.declaration.family,
                value: value.clone(),
                winning_source: active_token.declaration.source.clone(),
                losing_sources: active_token.losing_sources.clone(),
            })
        }
        ThemeTokenValueSource::Alias(alias) => {
            chain.push(id.clone());
            let resolved = match resolve_token(alias, active, request, chain, diagnostics) {
                Some(resolved) => resolved,
                None => {
                    if !active.contains_key(alias) {
                        diagnostics.push(error_for_declaration(
                            "ui.theme.alias.missing",
                            format!("token '{}' aliases missing token '{}'", id, alias),
                            &active_token.declaration,
                            request,
                            "declare the alias target or replace the alias with a concrete value",
                        ).with_alias_path(vec![id.clone(), alias.clone()]));
                    }
                    chain.pop();
                    return None;
                }
            };
            chain.pop();
            if resolved.family != active_token.declaration.family {
                diagnostics.push(
                    error_for_declaration(
                        "ui.theme.alias.family_mismatch",
                        format!(
                            "token '{}' family {:?} does not match alias '{}' family {:?}",
                            id, active_token.declaration.family, alias, resolved.family
                        ),
                        &active_token.declaration,
                        request,
                        "alias only to a token in the same family",
                    )
                    .with_alias_path(vec![id.clone(), alias.clone()]),
                );
                return None;
            }
            Some(ResolvedThemeToken {
                id: id.clone(),
                family: active_token.declaration.family,
                value: resolved.value,
                winning_source: active_token.declaration.source.clone(),
                losing_sources: {
                    let mut sources = active_token.losing_sources.clone();
                    sources.push(resolved.winning_source);
                    sources.extend(resolved.losing_sources);
                    sources
                },
            })
        }
    }
}

fn validate_selector_compatibility(
    declaration: &ThemeTokenDeclaration,
    request: &ThemeTokenResolveRequest,
) -> Option<ThemeTokenDiagnostic> {
    let selector = &declaration.selector;
    let has_context_selector = selector.state.is_some()
        || selector.mode.is_some()
        || selector.platform.is_some()
        || selector.accessibility.is_some();
    let code = match declaration.layer {
        ThemeTokenLayer::Primitive | ThemeTokenLayer::Semantic => (selector.component.is_some()
            || has_context_selector)
            .then_some("ui.theme.selector.incompatible"),
        ThemeTokenLayer::Component => (selector.component.is_none() || has_context_selector)
            .then_some("ui.theme.selector.incompatible"),
        ThemeTokenLayer::State => (selector.state.is_none() || selector.mode.is_some())
            .then_some("ui.theme.selector.incompatible"),
        ThemeTokenLayer::Mode => (selector.mode.is_none() || selector.state.is_some())
            .then_some("ui.theme.selector.incompatible"),
        ThemeTokenLayer::Theme | ThemeTokenLayer::Skin => {
            has_context_selector.then_some("ui.theme.selector.incompatible")
        }
        ThemeTokenLayer::Platform => {
            (selector.platform.is_none()).then_some("ui.theme.selector.incompatible")
        }
        ThemeTokenLayer::Accessibility => {
            (selector.accessibility.is_none()).then_some("ui.theme.selector.incompatible")
        }
        ThemeTokenLayer::Preview => None,
    };

    code.map(|code| {
        error_for_declaration(
            code,
            format!(
                "token '{}' has selector fields incompatible with layer {:?}",
                declaration.id, declaration.layer
            ),
            declaration,
            request,
            "move selectors to the matching precedence layer or split the token declaration",
        )
    })
}

#[derive(Debug, Clone, PartialEq)]
struct ActiveThemeToken {
    declaration: ThemeTokenDeclaration,
    losing_sources: Vec<ThemeTokenSourceId>,
}
