use crate::{
    AuthoredUiNodePath, FormedInteractionModel, FormedUiEmbed, UiAvailability,
    UiDefinitionDiagnostic,
};
use std::collections::{BTreeMap, BTreeSet};
use ui_tree::WidgetId;

use super::FormedUiRoute;
use super::context::UiDefinitionContext;

#[derive(Default)]
pub(super) struct FormationState {
    pub(super) routes_by_widget_id: BTreeMap<WidgetId, FormedUiRoute>,
    pub(super) paths_by_widget_id: BTreeMap<WidgetId, AuthoredUiNodePath>,
    pub(super) embeds_by_widget_id: BTreeMap<WidgetId, FormedUiEmbed>,
    pub(super) availability_by_widget_id: BTreeMap<WidgetId, UiAvailability>,
    pub(super) interaction_model: FormedInteractionModel,
    pub(super) used_widget_ids: BTreeSet<WidgetId>,
    pub(super) diagnostics: Vec<UiDefinitionDiagnostic>,
}

pub(super) fn assign_widget_id(
    path: &AuthoredUiNodePath,
    context: &mut UiDefinitionContext,
    state: &mut FormationState,
) -> WidgetId {
    let widget_id = if let Some(widget_id) = context.widget_ids_by_path.get(path).copied() {
        widget_id
    } else {
        let widget_id = context
            .widget_id_scope
            .map(|scope| scope.scoped_widget_id(context.next_widget_id))
            .unwrap_or(WidgetId(context.next_widget_id));
        context.next_widget_id += 1;
        context.widget_ids_by_path.insert(path.clone(), widget_id);
        widget_id
    };
    if !state.used_widget_ids.insert(widget_id) {
        state.diagnostics.push(
            UiDefinitionDiagnostic::error(
                "ui.definition.widget_id.duplicate",
                format!(
                    "formed widget id '{}' is assigned more than once",
                    widget_id.0
                ),
            )
            .at_path(path.clone()),
        );
    }
    widget_id
}
