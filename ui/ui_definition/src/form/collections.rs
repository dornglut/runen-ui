use crate::{
    NormalizedUiTemplate, UiAxisDefinition, UiCollectionItem, UiCollectionSlotId, UiNodeDefinition,
    UiSelectionSlotId, UiValue, UiValueSlotId,
};
use std::collections::BTreeMap;
use ui_layout::SizePolicy;
use ui_tree::{UiNode, WidgetId};
use ui_widgets::{hstack_with_policies, vstack_with_policies};

use super::context::UiDefinitionContext;
use super::dispatch::form_node;
use super::state::FormationState;

pub(super) fn form_repeat(
    widget_id: WidgetId,
    node: &UiNodeDefinition,
    path: &crate::AuthoredUiNodePath,
    template: &NormalizedUiTemplate,
    context: &mut UiDefinitionContext,
    state: &mut FormationState,
) -> Option<UiNode> {
    if let UiNodeDefinition::Repeat {
        id,
        items,
        template: template_id,
        axis,
    } = node
    {
        let collection = context
            .collections
            .get(&items.id)
            .cloned()
            .unwrap_or_default();
        let Some(child_template) = template.templates.get(template_id) else {
            state.diagnostics.push(
                crate::UiDefinitionDiagnostic::error(
                    "ui.definition.template.unresolved",
                    format!("unresolved template ref '{}'", template_id),
                )
                .at_path(path.clone()),
            );
            return Some(vstack_with_policies(
                widget_id,
                context.theme.spacing.xs,
                Vec::new(),
                Vec::new(),
            ));
        };
        let mut children = Vec::new();
        for item in &collection {
            let child_path = path.repeated_child(id, &item.key, child_template.root.id());
            let previous_values = install_repeat_item_values(item, context);
            children.push(form_node(
                &child_template.root,
                &child_path,
                child_template,
                context,
                state,
            ));
            restore_repeat_item_values(previous_values, context);
        }
        return Some(match axis.unwrap_or(UiAxisDefinition::Vertical) {
            UiAxisDefinition::Horizontal => hstack_with_policies(
                widget_id,
                context.theme.spacing.xs,
                vec![SizePolicy::Auto; children.len()],
                children,
            ),
            UiAxisDefinition::Vertical => vstack_with_policies(
                widget_id,
                context.theme.spacing.xs,
                vec![SizePolicy::Auto; children.len()],
                children,
            ),
        });
    }
    None
}

pub(super) fn form_template_ref(
    widget_id: WidgetId,
    node: &UiNodeDefinition,
    path: &crate::AuthoredUiNodePath,
    template: &NormalizedUiTemplate,
    context: &mut UiDefinitionContext,
    state: &mut FormationState,
) -> Option<UiNode> {
    if let UiNodeDefinition::TemplateRef {
        template: template_id,
        ..
    } = node
    {
        let Some(child_template) = template.templates.get(template_id) else {
            state.diagnostics.push(
                crate::UiDefinitionDiagnostic::error(
                    "ui.definition.template.unresolved",
                    format!("unresolved template ref '{}'", template_id),
                )
                .at_path(path.clone()),
            );
            return Some(vstack_with_policies(
                widget_id,
                context.theme.spacing.xs,
                Vec::new(),
                Vec::new(),
            ));
        };
        return Some(form_node(
            &child_template.root,
            path,
            child_template,
            context,
            state,
        ));
    }
    None
}

#[derive(Default)]
pub(super) struct RepeatItemBindings {
    values: BTreeMap<UiValueSlotId, Option<UiValue>>,
    collections: BTreeMap<UiCollectionSlotId, Option<Vec<UiCollectionItem>>>,
    selections: BTreeMap<UiSelectionSlotId, Option<String>>,
}

pub(super) fn install_repeat_item_values(
    item: &UiCollectionItem,
    context: &mut UiDefinitionContext,
) -> RepeatItemBindings {
    let mut values = item.values.clone();
    values.insert("item.key".into(), UiValue::Text(item.key.clone()));
    values.insert("item.label".into(), UiValue::Text(item.label.clone()));
    values.insert("item.selected".into(), UiValue::Bool(item.selected));
    values.insert("item.enabled".into(), UiValue::Bool(item.enabled));

    let mut previous = RepeatItemBindings::default();
    for (slot, value) in values {
        previous
            .values
            .insert(slot.clone(), context.values.insert(slot, value));
    }
    for (slot, collection) in &item.collections {
        previous.collections.insert(
            slot.clone(),
            context.collections.insert(slot.clone(), collection.clone()),
        );
    }
    for (slot, selection) in &item.selections {
        previous.selections.insert(
            slot.clone(),
            context.selections.insert(slot.clone(), selection.clone()),
        );
    }
    previous
}

pub(super) fn restore_repeat_item_values(
    previous_values: RepeatItemBindings,
    context: &mut UiDefinitionContext,
) {
    for (slot, previous) in previous_values.values {
        match previous {
            Some(value) => {
                context.values.insert(slot, value);
            }
            None => {
                context.values.remove(&slot);
            }
        }
    }
    for (slot, previous) in previous_values.collections {
        match previous {
            Some(collection) => {
                context.collections.insert(slot, collection);
            }
            None => {
                context.collections.remove(&slot);
            }
        }
    }
    for (slot, previous) in previous_values.selections {
        match previous {
            Some(selection) => {
                context.selections.insert(slot, selection);
            }
            None => {
                context.selections.remove(&slot);
            }
        }
    }
}
