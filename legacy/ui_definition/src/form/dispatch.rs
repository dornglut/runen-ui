use crate::{
    AuthoredUiNodePath, NormalizedUiTemplate, UiAvailability, UiAxisDefinition, UiNodeDefinition,
};
use ui_tree::UiNode;

use super::context::UiDefinitionContext;
use super::resolve::resolve_availability;
use super::scroll::formed_scroll_owner;
use super::state::{FormationState, assign_widget_id};
use super::{collections, containers, controls, scroll, slots};

pub(super) fn form_node(
    node: &UiNodeDefinition,
    path: &AuthoredUiNodePath,
    template: &NormalizedUiTemplate,
    context: &mut UiDefinitionContext,
    state: &mut FormationState,
) -> UiNode {
    let widget_id = assign_widget_id(path, context, state);
    state.paths_by_widget_id.insert(widget_id, path.clone());

    match node {
        UiNodeDefinition::Panel { children, .. } => {
            let formed = form_children(children, path, template, context, state);
            containers::form_panel(widget_id, context, formed)
        }
        UiNodeDefinition::Row { children, .. } => {
            let formed = form_children(children, path, template, context, state);
            containers::form_row(widget_id, children.len(), context, formed)
        }
        UiNodeDefinition::Column { children, .. }
        | UiNodeDefinition::Stack {
            axis: UiAxisDefinition::Vertical,
            children,
            ..
        } => {
            let formed = form_children(children, path, template, context, state);
            containers::form_column(widget_id, children.len(), context, formed)
        }
        UiNodeDefinition::Stack {
            axis: UiAxisDefinition::Horizontal,
            children,
            ..
        } => {
            let formed = form_children(children, path, template, context, state);
            containers::form_horizontal_stack(widget_id, children.len(), context, formed)
        }
        UiNodeDefinition::Scroll {
            axis,
            input,
            ownership,
            children,
            ..
        } => {
            let formed = form_children(children, path, template, context, state);
            let node = scroll::form_scroll(widget_id, *axis, *input, context, formed);
            state
                .interaction_model
                .push_scroll_owner(formed_scroll_owner(widget_id, *axis, *ownership));
            node
        }
        UiNodeDefinition::Split {
            axis,
            ratio,
            children,
            ..
        } => {
            let formed = form_children(children, path, template, context, state);
            containers::form_split(widget_id, *axis, *ratio, context, formed)
        }
        UiNodeDefinition::Spacer { .. } => containers::form_spacer(widget_id),
        UiNodeDefinition::Separator {
            axis,
            length,
            thickness,
            ..
        } => containers::form_separator(
            widget_id,
            axis.unwrap_or(UiAxisDefinition::Horizontal),
            *length,
            thickness.unwrap_or(1.0),
            context,
        ),
        UiNodeDefinition::Label { .. } => {
            controls::form_label(widget_id, node, context).expect("label node should form as label")
        }
        UiNodeDefinition::Control { .. } => {
            controls::form_unsupported_control(widget_id, node, path, context, state)
                .expect("control node should form as unsupported control placeholder")
        }
        UiNodeDefinition::Button { .. } => controls::form_button(widget_id, node, context, state)
            .expect("button node should form as button"),
        UiNodeDefinition::Toggle { .. } => controls::form_toggle(widget_id, node, context, state)
            .expect("toggle node should form as toggle"),
        UiNodeDefinition::TextInput { .. } => {
            controls::form_text_input(widget_id, node, context, state)
                .expect("text input node should form as text input")
        }
        UiNodeDefinition::NumericInput { .. } => {
            controls::form_numeric_input(widget_id, node, context, state)
                .expect("numeric input node should form as numeric input")
        }
        UiNodeDefinition::Select { .. } => controls::form_select(widget_id, node, context, state)
            .expect("select node should form as select"),
        UiNodeDefinition::Tabs { .. } => controls::form_tabs(widget_id, node, context, state)
            .expect("tabs node should form as tabs"),
        UiNodeDefinition::Table { .. } => controls::form_table(widget_id, node, context, state)
            .expect("table node should form as table"),
        UiNodeDefinition::Tree { .. } => controls::form_tree(widget_id, node, context, state)
            .expect("tree node should form as tree"),
        UiNodeDefinition::Repeat { .. } => {
            collections::form_repeat(widget_id, node, path, template, context, state)
                .expect("repeat node should form as repeat")
        }
        UiNodeDefinition::TemplateRef { .. } => {
            collections::form_template_ref(widget_id, node, path, template, context, state)
                .expect("template ref node should form as template ref")
        }
        UiNodeDefinition::MenuSlot { .. } => {
            slots::form_menu_slot(widget_id, node, path, context, state)
                .expect("menu slot node should form as menu slot")
        }
        UiNodeDefinition::EmbedSlot { .. } => {
            slots::form_embed_slot(widget_id, node, context, state)
                .expect("embed slot node should form as embed slot")
        }
    }
}

pub(super) fn form_children(
    children: &[UiNodeDefinition],
    parent_path: &AuthoredUiNodePath,
    template: &NormalizedUiTemplate,
    context: &mut UiDefinitionContext,
    state: &mut FormationState,
) -> Vec<UiNode> {
    children
        .iter()
        .filter_map(|child| {
            if node_is_unavailable(child, context) {
                return None;
            }
            Some(form_node(
                child,
                &parent_path.child(child.id()),
                template,
                context,
                state,
            ))
        })
        .collect()
}

pub(super) fn node_is_unavailable(node: &UiNodeDefinition, context: &UiDefinitionContext) -> bool {
    let availability = match node {
        UiNodeDefinition::Panel { availability, .. }
        | UiNodeDefinition::Label { availability, .. }
        | UiNodeDefinition::Button { availability, .. }
        | UiNodeDefinition::Toggle { availability, .. }
        | UiNodeDefinition::TextInput { availability, .. }
        | UiNodeDefinition::NumericInput { availability, .. }
        | UiNodeDefinition::Select { availability, .. } => availability.as_ref(),
        _ => None,
    };
    matches!(
        resolve_availability(availability, context),
        UiAvailability::Unavailable { .. }
    )
}
