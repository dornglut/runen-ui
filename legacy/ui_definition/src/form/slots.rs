use crate::{FormedUiEmbed, UiNodeDefinition, UiNodeId, UiRouteSlotId};
use ui_layout::SizePolicy;
use ui_render_data::ViewportSurfaceEmbedSlotId;
use ui_text::FontId;
use ui_tree::{UiNode, UiNodeKind, WidgetId};
use ui_widgets::{button_selected, hstack_with_policies, viewport_surface_embed};

use super::FormedUiRoute;
use super::context::UiDefinitionContext;
use super::state::{FormationState, assign_widget_id};

pub(super) fn form_menu_slot(
    widget_id: WidgetId,
    node: &UiNodeDefinition,
    path: &crate::AuthoredUiNodePath,
    context: &mut UiDefinitionContext,
    state: &mut FormationState,
) -> Option<UiNode> {
    if let UiNodeDefinition::MenuSlot { menu, .. } = node {
        let items = context.menus.get(&menu.id).cloned().unwrap_or_default();
        let mut children = Vec::new();
        for item in items {
            let item_path = path.child(&UiNodeId::new(item.key.clone()));
            let child_id = assign_widget_id(&item_path, context, state);
            let mut child = button_selected(
                child_id,
                item.label,
                context.theme.body_small_text_style(FontId(1)),
                context.theme.clone(),
                item.selected,
            );
            if let UiNodeKind::Button(button) = &mut child.kind {
                button.enabled = item.enabled;
            }
            state.paths_by_widget_id.insert(child_id, item_path);
            if item.enabled {
                state.routes_by_widget_id.insert(
                    child_id,
                    FormedUiRoute::RouteSlot(UiRouteSlotId::new(item.key.clone())),
                );
            }
            children.push(child);
        }
        return Some(hstack_with_policies(
            widget_id,
            context.theme.spacing.xs,
            vec![SizePolicy::Auto; children.len()],
            children,
        ));
    }
    None
}

pub(super) fn form_embed_slot(
    widget_id: WidgetId,
    node: &UiNodeDefinition,
    context: &UiDefinitionContext,
    state: &mut FormationState,
) -> Option<UiNode> {
    if let UiNodeDefinition::EmbedSlot { slot, .. } = node {
        let raw_slot = context.embed_slots.get(&slot.id).copied().unwrap_or(1);
        let formed =
            viewport_surface_embed(widget_id, 0, ViewportSurfaceEmbedSlotId::new(raw_slot));
        state.embeds_by_widget_id.insert(
            widget_id,
            FormedUiEmbed {
                slot: slot.id.clone(),
            },
        );
        return Some(formed);
    }
    None
}
