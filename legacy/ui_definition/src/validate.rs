//! UI definition validation.

use crate::{
    AuthoredId, AuthoredUiNodePath, AuthoredUiTemplate, UiDefinitionDiagnostic, UiMenuDefinition,
    UiNodeDefinition,
};
use std::collections::BTreeSet;

pub fn validate_authored_template(template: &AuthoredUiTemplate) -> Vec<UiDefinitionDiagnostic> {
    let mut diagnostics = Vec::new();
    validate_id(&template.id, "template", &mut diagnostics);
    let mut ids = BTreeSet::new();
    validate_node(
        &template.root,
        AuthoredUiNodePath::root(template.root.id()),
        &mut ids,
        &mut diagnostics,
    );
    for menu in &template.menus {
        validate_menu(menu, &mut diagnostics);
    }
    diagnostics
}

fn validate_menu(menu: &UiMenuDefinition, diagnostics: &mut Vec<UiDefinitionDiagnostic>) {
    if menu.id.trim().is_empty() {
        diagnostics.push(UiDefinitionDiagnostic::error(
            "ui.definition.menu.id.empty",
            "menu id must not be empty",
        ));
    }
    if !menu.items.is_empty() && menu.sizing.is_none() {
        diagnostics.push(UiDefinitionDiagnostic::error(
            "ui.definition.menu.sizing.missing",
            format!(
                "menu '{}' with items must declare menu sizing policy",
                menu.id
            ),
        ));
    }
    let Some(scope) = menu.scope.as_ref() else {
        return;
    };
    if scope.id.trim().is_empty() {
        diagnostics.push(UiDefinitionDiagnostic::error(
            "ui.definition.menu_scope.id.empty",
            format!("menu '{}' stack scope id must not be empty", menu.id),
        ));
    }
    if scope.anchor.as_str().trim().is_empty() {
        diagnostics.push(UiDefinitionDiagnostic::error(
            "ui.definition.menu_scope.anchor.empty",
            format!("menu '{}' stack scope anchor must not be empty", menu.id),
        ));
    }
    if scope.parent.as_deref() == Some(scope.id.as_str()) {
        diagnostics.push(UiDefinitionDiagnostic::error(
            "ui.definition.menu_scope.parent.self",
            format!("menu '{}' stack scope must not parent itself", menu.id),
        ));
    }
}

fn validate_node(
    node: &UiNodeDefinition,
    path: AuthoredUiNodePath,
    ids: &mut BTreeSet<AuthoredId>,
    diagnostics: &mut Vec<UiDefinitionDiagnostic>,
) {
    validate_id(node.id(), "node", diagnostics);
    if !ids.insert(node.id().clone()) {
        diagnostics.push(
            UiDefinitionDiagnostic::error(
                "ui.definition.node.duplicate",
                format!("duplicate node id '{}'", node.id()),
            )
            .at_path(path.clone()),
        );
    }

    if matches!(node, UiNodeDefinition::Repeat { .. }) && node.children().is_empty() {
        // Repeat children are supplied by referenced templates.
    }

    if let UiNodeDefinition::Split { children, .. } = node
        && children.len() != 2
    {
        diagnostics.push(
            UiDefinitionDiagnostic::error(
                "ui.definition.split.child_count",
                "split nodes require exactly two authored children",
            )
            .at_path(path.clone()),
        );
    }

    for child in node.children() {
        validate_node(child, path.child(child.id()), ids, diagnostics);
    }
}

fn validate_id(id: &AuthoredId, kind: &str, diagnostics: &mut Vec<UiDefinitionDiagnostic>) {
    let value = id.as_str();
    if value.trim().is_empty() {
        diagnostics.push(UiDefinitionDiagnostic::error(
            "ui.definition.id.empty",
            format!("{kind} id must not be empty"),
        ));
        return;
    }
    if value.chars().any(char::is_whitespace) {
        diagnostics.push(UiDefinitionDiagnostic::error(
            "ui.definition.id.whitespace",
            format!("{kind} id '{value}' must not contain whitespace"),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        UiMenuDismissPolicyDefinition, UiMenuFocusReturnDefinition, UiMenuStackScopeDefinition,
    };

    #[test]
    fn menu_stack_scope_rejects_self_parent() {
        let template = AuthoredUiTemplate {
            id: "test.menu".into(),
            root: UiNodeDefinition::Panel {
                id: "root".into(),
                children: Vec::new(),
                availability: None,
            },
            templates: Vec::new(),
            menus: vec![UiMenuDefinition {
                id: "file".to_string(),
                scope: Some(UiMenuStackScopeDefinition {
                    id: "file".to_string(),
                    anchor: "file_anchor".into(),
                    parent: Some("file".to_string()),
                    dismiss: UiMenuDismissPolicyDefinition::OutsidePointerDown,
                    focus_return: UiMenuFocusReturnDefinition::Anchor,
                }),
                sizing: None,
                items: Vec::new(),
            }],
        };

        let diagnostics = validate_authored_template(&template);

        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "ui.definition.menu_scope.parent.self"),
            "menu stack validation should reject self-parented scopes: {diagnostics:?}",
        );
    }

    #[test]
    fn menu_items_require_sizing_policy() {
        let template = AuthoredUiTemplate {
            id: "test.menu".into(),
            root: UiNodeDefinition::Panel {
                id: "root".into(),
                children: Vec::new(),
                availability: None,
            },
            templates: Vec::new(),
            menus: vec![UiMenuDefinition {
                id: "file".to_string(),
                scope: None,
                sizing: None,
                items: vec![crate::UiMenuItemDefinition {
                    id: "save".into(),
                    label: crate::UiValueBinding::Static(crate::UiValue::Text("Save".to_string())),
                    route: None,
                    availability: None,
                }],
            }],
        };

        let diagnostics = validate_authored_template(&template);

        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "ui.definition.menu.sizing.missing"),
            "menu validation should reject item menus without sizing policy: {diagnostics:?}",
        );
    }
}
