use crate::UiNodeDefinition;
use ui_text::FontId;
use ui_tree::{TableColumn, TableRow, TreeRow, UiNode, UiNodeKind, WidgetId};
use ui_widgets::{
    NumericInputConfig, button_selected, label, numeric_input, select, table, tabs, text_input,
    toggle, tree,
};

use super::FormedUiRoute;
use super::context::UiDefinitionContext;
use super::resolve::{resolve_availability, resolve_item_text, resolve_text, resolve_value};
use super::state::FormationState;

pub(super) fn form_label(
    widget_id: WidgetId,
    node: &UiNodeDefinition,
    context: &UiDefinitionContext,
) -> Option<UiNode> {
    if let UiNodeDefinition::Label { label: text, .. } = node {
        return Some(label(
            widget_id,
            resolve_text(text, context),
            context.theme.body_text_style(FontId(1)),
        ));
    }
    None
}

pub(super) fn form_unsupported_control(
    widget_id: WidgetId,
    node: &UiNodeDefinition,
    path: &crate::AuthoredUiNodePath,
    context: &UiDefinitionContext,
    state: &mut FormationState,
) -> Option<UiNode> {
    if let UiNodeDefinition::Control { kind, .. } = node {
        state.diagnostics.push(
            crate::UiDefinitionDiagnostic::error(
                "ui.definition.retained_form.generic_control_unsupported",
                format!(
                    "generic control '{}' must be formed through UiProgram formation, not retained UiTree formation",
                    kind.as_str()
                ),
            )
            .at_path(path.clone()),
        );

        return Some(label(
            widget_id,
            format!("Unsupported control: {}", kind.as_str()),
            context.theme.body_small_text_style(FontId(1)),
        ));
    }
    None
}

pub(super) fn form_button(
    widget_id: WidgetId,
    node: &UiNodeDefinition,
    context: &UiDefinitionContext,
    state: &mut FormationState,
) -> Option<UiNode> {
    if let UiNodeDefinition::Button {
        label,
        route,
        availability,
        selected,
        ..
    } = node
    {
        let availability = resolve_availability(availability.as_ref(), context);
        let mut formed = button_selected(
            widget_id,
            resolve_text(label, context),
            context.theme.body_text_style(FontId(1)),
            context.theme.clone(),
            selected
                .as_ref()
                .and_then(|value| resolve_value(value, context).as_bool())
                .unwrap_or(false),
        );
        if let UiNodeKind::Button(button) = &mut formed.kind {
            button.enabled = availability.is_enabled();
        }
        state
            .availability_by_widget_id
            .insert(widget_id, availability.clone());
        if availability.is_enabled()
            && let Some(route) = route
        {
            state
                .routes_by_widget_id
                .insert(widget_id, FormedUiRoute::RouteSlot(route.id.clone()));
        }
        return Some(formed);
    }
    None
}

pub(super) fn form_toggle(
    widget_id: WidgetId,
    node: &UiNodeDefinition,
    context: &UiDefinitionContext,
    state: &mut FormationState,
) -> Option<UiNode> {
    if let UiNodeDefinition::Toggle {
        label,
        checked,
        route,
        availability,
        ..
    } = node
    {
        let availability = resolve_availability(availability.as_ref(), context);
        let mut formed = toggle(
            widget_id,
            resolve_text(label, context),
            resolve_value(checked, context).as_bool().unwrap_or(false),
            context.theme.body_text_style(FontId(1)),
            context.theme.clone(),
        );
        if let UiNodeKind::Toggle(toggle) = &mut formed.kind {
            toggle.enabled = availability.is_enabled();
        }
        state
            .availability_by_widget_id
            .insert(widget_id, availability.clone());
        if availability.is_enabled()
            && let Some(route) = route
        {
            state
                .routes_by_widget_id
                .insert(widget_id, FormedUiRoute::RouteSlot(route.id.clone()));
        }
        return Some(formed);
    }
    None
}

pub(super) fn form_text_input(
    widget_id: WidgetId,
    node: &UiNodeDefinition,
    context: &UiDefinitionContext,
    state: &mut FormationState,
) -> Option<UiNode> {
    if let UiNodeDefinition::TextInput {
        value,
        placeholder,
        route,
        availability,
        ..
    } = node
    {
        let availability = resolve_availability(availability.as_ref(), context);
        let mut formed = text_input(
            widget_id,
            resolve_text(value, context),
            placeholder.clone().unwrap_or_default(),
            context.theme.body_text_style(FontId(1)),
            context.theme.clone(),
        );
        if let UiNodeKind::TextInput(input) = &mut formed.kind {
            input.editable = availability.is_enabled();
        }
        state
            .availability_by_widget_id
            .insert(widget_id, availability.clone());
        if availability.is_enabled()
            && let Some(route) = route
        {
            state
                .routes_by_widget_id
                .insert(widget_id, FormedUiRoute::RouteSlot(route.id.clone()));
        }
        return Some(formed);
    }
    None
}

pub(super) fn form_numeric_input(
    widget_id: WidgetId,
    node: &UiNodeDefinition,
    context: &UiDefinitionContext,
    state: &mut FormationState,
) -> Option<UiNode> {
    if let UiNodeDefinition::NumericInput {
        value,
        route,
        availability,
        ..
    } = node
    {
        let availability = resolve_availability(availability.as_ref(), context);
        let mut formed = numeric_input(
            widget_id,
            NumericInputConfig::new(
                resolve_value(value, context)
                    .as_number()
                    .unwrap_or_default(),
                1.0,
                None,
                None,
                2,
            ),
            context.theme.body_text_style(FontId(1)),
            context.theme.clone(),
        );
        if let UiNodeKind::NumericInput(input) = &mut formed.kind {
            input.enabled = availability.is_enabled();
        }
        state
            .availability_by_widget_id
            .insert(widget_id, availability.clone());
        if availability.is_enabled()
            && let Some(route) = route
        {
            state
                .routes_by_widget_id
                .insert(widget_id, FormedUiRoute::RouteSlot(route.id.clone()));
        }
        return Some(formed);
    }
    None
}

pub(super) fn form_select(
    widget_id: WidgetId,
    node: &UiNodeDefinition,
    context: &UiDefinitionContext,
    state: &mut FormationState,
) -> Option<UiNode> {
    if let UiNodeDefinition::Select {
        items,
        selected,
        route,
        availability,
        ..
    } = node
    {
        let availability = resolve_availability(availability.as_ref(), context);
        let collection = context
            .collections
            .get(&items.id)
            .cloned()
            .unwrap_or_default();
        let selected_key = selected
            .as_ref()
            .and_then(|slot| context.selections.get(&slot.id));
        let selected_index = collection
            .iter()
            .position(|item| Some(&item.key) == selected_key);
        let mut formed = select(
            widget_id,
            collection.iter().map(|item| item.label.clone()),
            selected_index,
            "Select",
            context.theme.body_text_style(FontId(1)),
            context.theme.clone(),
        );
        if let UiNodeKind::Select(select) = &mut formed.kind {
            select.enabled = availability.is_enabled();
        }
        state
            .availability_by_widget_id
            .insert(widget_id, availability.clone());
        if availability.is_enabled()
            && let Some(route) = route
        {
            state
                .routes_by_widget_id
                .insert(widget_id, FormedUiRoute::RouteSlot(route.id.clone()));
        }
        return Some(formed);
    }
    None
}

pub(super) fn form_tabs(
    widget_id: WidgetId,
    node: &UiNodeDefinition,
    context: &UiDefinitionContext,
    state: &mut FormationState,
) -> Option<UiNode> {
    if let UiNodeDefinition::Tabs {
        items,
        selected,
        route,
        ..
    } = node
    {
        let collection = context
            .collections
            .get(&items.id)
            .cloned()
            .unwrap_or_default();
        let selected_key = selected
            .as_ref()
            .and_then(|slot| context.selections.get(&slot.id));
        let selected_index = collection
            .iter()
            .position(|item| Some(&item.key) == selected_key)
            .unwrap_or(0);
        if let Some(route) = route {
            state
                .routes_by_widget_id
                .insert(widget_id, FormedUiRoute::RouteSlot(route.id.clone()));
        }
        return Some(tabs(
            widget_id,
            collection.iter().map(|item| item.label.clone()),
            selected_index,
            context.theme.body_small_text_style(FontId(1)),
            context.theme.clone(),
        ));
    }
    None
}

pub(super) fn form_table(
    widget_id: WidgetId,
    node: &UiNodeDefinition,
    context: &UiDefinitionContext,
    state: &mut FormationState,
) -> Option<UiNode> {
    if let UiNodeDefinition::Table {
        rows,
        columns,
        route,
        ..
    } = node
    {
        let items = context
            .collections
            .get(&rows.id)
            .cloned()
            .unwrap_or_default();
        if let Some(route) = route {
            state
                .routes_by_widget_id
                .insert(widget_id, FormedUiRoute::RouteSlot(route.id.clone()));
        }
        let formed_columns = if columns.is_empty() {
            vec![TableColumn::new("Item", 160.0)]
        } else {
            columns
                .iter()
                .map(|column| TableColumn::new(column.label.clone(), column.width))
                .collect()
        };
        return Some(table(
            widget_id,
            formed_columns,
            items.iter().map(|item| {
                let cells = if columns.is_empty() {
                    vec![item.label.clone()]
                } else {
                    columns
                        .iter()
                        .map(|column| resolve_item_text(item, &column.value, context))
                        .collect()
                };
                let mut row = TableRow::new(cells);
                row.selected = item.selected;
                row.enabled = item.enabled;
                row
            }),
            context.theme.body_text_style(FontId(1)),
            context.theme.body_small_text_style(FontId(1)),
            context.theme.clone(),
        ));
    }
    None
}

pub(super) fn form_tree(
    widget_id: WidgetId,
    node: &UiNodeDefinition,
    context: &UiDefinitionContext,
    state: &mut FormationState,
) -> Option<UiNode> {
    if let UiNodeDefinition::Tree { rows, route, .. } = node {
        let items = context
            .collections
            .get(&rows.id)
            .cloned()
            .unwrap_or_default();
        if let Some(route) = route {
            state
                .routes_by_widget_id
                .insert(widget_id, FormedUiRoute::RouteSlot(route.id.clone()));
        }
        return Some(tree(
            widget_id,
            items.iter().map(|item| {
                let depth = item
                    .values
                    .get(&"tree.depth".into())
                    .and_then(crate::UiValue::as_number)
                    .map(|value| value.max(0.0) as usize)
                    .unwrap_or(0);
                let has_children = item
                    .values
                    .get(&"tree.has_children".into())
                    .and_then(crate::UiValue::as_bool)
                    .unwrap_or(false);
                let mut row = TreeRow::new(item.label.clone(), depth, has_children);
                row.selected = item.selected;
                row.enabled = item.enabled;
                row.expanded = item
                    .values
                    .get(&"tree.expanded".into())
                    .and_then(crate::UiValue::as_bool)
                    .unwrap_or(false);
                row
            }),
            context.theme.body_text_style(FontId(1)),
            context.theme.clone(),
        ));
    }
    None
}
