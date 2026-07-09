//! UI template normalization.

use crate::{
    AuthoredUiTemplate, NormalizedUiTemplate, UiDefinitionDiagnostic, UiTemplateId,
    validate::validate_authored_template,
};
use std::collections::BTreeMap;

pub fn normalize_authored_template(template: AuthoredUiTemplate) -> NormalizedUiTemplate {
    let mut diagnostics = validate_authored_template(&template);
    let mut templates = BTreeMap::<UiTemplateId, NormalizedUiTemplate>::new();
    for child in template.templates {
        if templates.contains_key(&child.id) {
            diagnostics.push(UiDefinitionDiagnostic::error(
                "ui.definition.template.duplicate",
                format!("duplicate template id '{}'", child.id),
            ));
            continue;
        }
        let normalized = normalize_authored_template(child);
        templates.insert(normalized.id.clone(), normalized);
    }
    let mut menus = BTreeMap::new();
    for menu in template.menus {
        if menus.insert(menu.id.clone(), menu).is_some() {
            diagnostics.push(UiDefinitionDiagnostic::error(
                "ui.definition.menu.duplicate",
                "duplicate menu id",
            ));
        }
    }
    NormalizedUiTemplate {
        id: template.id,
        root: template.root,
        templates,
        menus,
        diagnostics,
    }
}
