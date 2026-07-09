//! Resolved theme token packet and application helpers.

use std::collections::BTreeMap;

use crate::ThemeTokens;

use super::{
    ThemeTokenDiagnostic, ThemeTokenDiagnosticSeverity, ThemeTokenFamily, ThemeTokenId,
    ThemeTokenSourceId, ThemeTokenValue,
};

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedThemeToken {
    pub id: ThemeTokenId,
    pub family: ThemeTokenFamily,
    pub value: ThemeTokenValue,
    pub winning_source: ThemeTokenSourceId,
    pub losing_sources: Vec<ThemeTokenSourceId>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ThemeTokenResolutionReport {
    pub tokens: BTreeMap<ThemeTokenId, ResolvedThemeToken>,
    pub diagnostics: Vec<ThemeTokenDiagnostic>,
}

impl ThemeTokenResolutionReport {
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == ThemeTokenDiagnosticSeverity::Error)
    }

    pub fn apply_to_theme_tokens(&self, base: &ThemeTokens) -> ThemeTokens {
        let mut formed = base.clone();
        for token in self.tokens.values() {
            apply_known_token(&mut formed, token);
        }
        formed
    }
}

fn apply_known_token(theme: &mut ThemeTokens, token: &ResolvedThemeToken) {
    match (token.id.as_str(), &token.value) {
        ("color.background", ThemeTokenValue::Color(value)) => theme.background = *value,
        ("color.background_panel", ThemeTokenValue::Color(value)) => {
            theme.background_panel = *value
        }
        ("color.foreground", ThemeTokenValue::Color(value)) => theme.foreground = *value,
        ("color.foreground_muted", ThemeTokenValue::Color(value)) => {
            theme.foreground_muted = *value
        }
        ("color.accent", ThemeTokenValue::Color(value)) => theme.accent = *value,
        ("color.border", ThemeTokenValue::Color(value)) => theme.border = *value,
        ("spacing.xs", ThemeTokenValue::Number(value)) => theme.spacing.xs = *value,
        ("spacing.sm", ThemeTokenValue::Number(value)) => theme.spacing.sm = *value,
        ("spacing.md", ThemeTokenValue::Number(value)) => theme.spacing.md = *value,
        ("spacing.lg", ThemeTokenValue::Number(value)) => theme.spacing.lg = *value,
        ("spacing.xl", ThemeTokenValue::Number(value)) => theme.spacing.xl = *value,
        ("radius.sm", ThemeTokenValue::Number(value)) => theme.radius.sm = *value,
        ("radius.md", ThemeTokenValue::Number(value)) => theme.radius.md = *value,
        ("radius.lg", ThemeTokenValue::Number(value)) => theme.radius.lg = *value,
        ("typography.body", ThemeTokenValue::Number(value)) => theme.typography.body = *value,
        ("typography.body_small", ThemeTokenValue::Number(value)) => {
            theme.typography.body_small = *value
        }
        ("typography.heading", ThemeTokenValue::Number(value)) => theme.typography.heading = *value,
        ("typography.monospace", ThemeTokenValue::Number(value)) => {
            theme.typography.monospace = *value
        }
        ("border.width", ThemeTokenValue::Number(value)) => theme.border_width = *value,
        _ => {}
    }
}
