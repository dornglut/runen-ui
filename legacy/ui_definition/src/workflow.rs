//! Focused public workflow helpers for behavior-free UI definitions.

use crate::{
    AuthoredUiTemplate, NormalizedUiTemplate, UiPersistenceActivationLibrary,
    UiPersistenceActivationValidationReport, UiPersistenceActivationValidationRequest,
    UiPreviewLibrary, UiPreviewValidationReport, UiPreviewValidationRequest, UiReadinessLibrary,
    UiReadinessValidationReport, UiReadinessValidationRequest, UiVisualLayoutActivationMode,
    UiVisualLayoutEditContext, UiVisualLayoutEditReport, UiVisualLayoutOperation,
    apply_visual_layout_operation, normalize_authored_template, validate_authored_template,
    validate_persistence_activation, validate_preview_fixtures, validate_production_readiness,
};

pub use crate::UiDefinitionDiagnostic;

pub fn validate_ui_template(template: &AuthoredUiTemplate) -> Vec<UiDefinitionDiagnostic> {
    validate_authored_template(template)
}

pub fn normalize_ui_template(template: AuthoredUiTemplate) -> NormalizedUiTemplate {
    normalize_authored_template(template)
}

pub fn apply_ui_layout_operation(
    template: AuthoredUiTemplate,
    operation: &UiVisualLayoutOperation,
    context: &UiVisualLayoutEditContext,
) -> UiVisualLayoutEditReport {
    apply_visual_layout_operation(
        template,
        operation,
        UiVisualLayoutActivationMode::Activate,
        context,
    )
}

pub fn preview_ui_layout_operation(
    template: AuthoredUiTemplate,
    operation: &UiVisualLayoutOperation,
    context: &UiVisualLayoutEditContext,
) -> UiVisualLayoutEditReport {
    apply_visual_layout_operation(
        template,
        operation,
        UiVisualLayoutActivationMode::Preview,
        context,
    )
}

pub fn validate_ui_preview_library(
    library: &UiPreviewLibrary,
    request: &UiPreviewValidationRequest,
) -> UiPreviewValidationReport {
    validate_preview_fixtures(library, request)
}

pub fn validate_ui_persistence_flow(
    library: &UiPersistenceActivationLibrary,
    request: &UiPersistenceActivationValidationRequest,
) -> UiPersistenceActivationValidationReport {
    validate_persistence_activation(library, request)
}

pub fn validate_ui_readiness(
    library: &UiReadinessLibrary,
    request: &UiReadinessValidationRequest,
) -> UiReadinessValidationReport {
    validate_production_readiness(library, request)
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;

    #[test]
    fn prelude_supports_normal_ui_definition_workflow() {
        let template = template();
        let normalized = normalize_ui_template(template.clone());

        assert!(!normalized.has_errors(), "{:?}", normalized.diagnostics);

        let operation = UiVisualLayoutOperation {
            id: "axis.stack".into(),
            source_document: template.id.clone(),
            target_path: AuthoredUiNodePath("root/stack".to_string()),
            expected_node_id: "stack".into(),
            target_profile: "editor.workbench".into(),
            kind: UiVisualLayoutEditKind::ChangeStackAxis {
                axis: UiAxisDefinition::Horizontal,
            },
            source_location: None,
            preview_only: false,
        };
        let context =
            UiVisualLayoutEditContext::with_supported_target_profiles(["editor.workbench".into()]);

        let edit = apply_ui_layout_operation(template, &operation, &context);

        assert!(!edit.has_errors(), "{:?}", edit.diagnostics);
        assert!(edit.diff.is_some());

        let preview = validate_ui_preview_library(
            &UiPreviewLibrary::default(),
            &UiPreviewValidationRequest::preview("editor.workbench"),
        );
        assert!(preview.has_errors());

        let persistence = validate_ui_persistence_flow(
            &UiPersistenceActivationLibrary::default(),
            &UiPersistenceActivationValidationRequest::dry_run("editor.workbench"),
        );
        assert!(!persistence.has_errors());

        let readiness = validate_ui_readiness(
            &UiReadinessLibrary::default(),
            &UiReadinessValidationRequest::inspect("editor.workbench"),
        );
        assert!(!readiness.has_errors());
    }

    fn template() -> AuthoredUiTemplate {
        AuthoredUiTemplate {
            id: "example.template".into(),
            root: UiNodeDefinition::Column {
                id: "root".into(),
                children: vec![UiNodeDefinition::Stack {
                    id: "stack".into(),
                    axis: UiAxisDefinition::Vertical,
                    children: vec![UiNodeDefinition::Label {
                        id: "title".into(),
                        label: UiValueBinding::static_text("Title"),
                        availability: None,
                    }],
                }],
            },
            templates: Vec::new(),
            menus: Vec::new(),
        }
    }
}
