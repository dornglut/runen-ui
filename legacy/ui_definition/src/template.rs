//! Authored and normalized UI templates.

use crate::{
    diagnostic::UiDefinitionDiagnostic, identity::UiTemplateId, menu::UiMenuDefinition,
    node::UiNodeDefinition,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuthoredUiTemplate {
    pub id: UiTemplateId,
    pub root: UiNodeDefinition,
    #[serde(default)]
    pub templates: Vec<AuthoredUiTemplate>,
    #[serde(default)]
    pub menus: Vec<UiMenuDefinition>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NormalizedUiTemplate {
    pub id: UiTemplateId,
    pub root: UiNodeDefinition,
    pub templates: BTreeMap<UiTemplateId, NormalizedUiTemplate>,
    pub menus: BTreeMap<String, UiMenuDefinition>,
    pub diagnostics: Vec<UiDefinitionDiagnostic>,
}

impl NormalizedUiTemplate {
    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(|diagnostic| {
            matches!(
                diagnostic.severity,
                crate::UiDefinitionDiagnosticSeverity::Error
            )
        })
    }
}
