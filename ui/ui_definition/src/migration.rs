//! Versioned authored UI definition documents and execution-neutral migrations.

use crate::{AuthoredUiTemplate, UiDefinitionDiagnostic, UiDefinitionDiagnosticSeverity};
use serde::{Deserialize, Serialize};

pub const CURRENT_UI_DEFINITION_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthoredUiDefinitionCategory {
    Editor,
    DebugOverlay,
    RuntimeOverlay,
    GameUi,
    Fixture,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VersionedAuthoredUiTemplate {
    pub schema_version: u32,
    pub category: AuthoredUiDefinitionCategory,
    pub template: AuthoredUiTemplate,
}

impl VersionedAuthoredUiTemplate {
    pub fn current(category: AuthoredUiDefinitionCategory, template: AuthoredUiTemplate) -> Self {
        Self {
            schema_version: CURRENT_UI_DEFINITION_SCHEMA_VERSION,
            category,
            template,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct UiDefinitionMigrationReport {
    pub migrated: VersionedAuthoredUiTemplate,
    pub diagnostics: Vec<UiDefinitionDiagnostic>,
}

impl UiDefinitionMigrationReport {
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == UiDefinitionDiagnosticSeverity::Error)
    }
}

pub fn migrate_authored_ui_template(
    document: VersionedAuthoredUiTemplate,
) -> UiDefinitionMigrationReport {
    if document.schema_version == CURRENT_UI_DEFINITION_SCHEMA_VERSION {
        return UiDefinitionMigrationReport {
            migrated: document,
            diagnostics: Vec::new(),
        };
    }
    let schema_version = document.schema_version;

    UiDefinitionMigrationReport {
        migrated: document,
        diagnostics: vec![UiDefinitionDiagnostic::error(
            "ui.definition.migration.unsupported_version",
            format!(
                "UI definition schema version '{}' is not supported by version '{}'",
                schema_version, CURRENT_UI_DEFINITION_SCHEMA_VERSION
            ),
        )],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{UiNodeDefinition, UiNodeId, UiValueBinding};

    #[test]
    fn current_version_migrates_without_diagnostics() {
        let document = VersionedAuthoredUiTemplate::current(
            AuthoredUiDefinitionCategory::Fixture,
            AuthoredUiTemplate {
                id: "test.template".into(),
                root: UiNodeDefinition::Label {
                    id: UiNodeId::from("root"),
                    label: UiValueBinding::static_text("Root"),
                    availability: None,
                },
                templates: Vec::new(),
                menus: Vec::new(),
            },
        );

        let report = migrate_authored_ui_template(document);

        assert!(!report.has_errors());
        assert_eq!(
            report.migrated.schema_version,
            CURRENT_UI_DEFINITION_SCHEMA_VERSION
        );
    }

    #[test]
    fn unsupported_version_is_blocking() {
        let mut document = VersionedAuthoredUiTemplate::current(
            AuthoredUiDefinitionCategory::Fixture,
            AuthoredUiTemplate {
                id: "test.template".into(),
                root: UiNodeDefinition::Label {
                    id: UiNodeId::from("root"),
                    label: UiValueBinding::static_text("Root"),
                    availability: None,
                },
                templates: Vec::new(),
                menus: Vec::new(),
            },
        );
        document.schema_version = 99;

        let report = migrate_authored_ui_template(document);

        assert!(report.has_errors());
        assert_eq!(
            report.diagnostics[0].code,
            "ui.definition.migration.unsupported_version"
        );
    }
}
