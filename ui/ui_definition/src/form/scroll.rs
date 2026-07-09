use crate::{
    UiScrollAxisDefinition, UiScrollInputDefinition, UiScrollInputPolicyDefinition,
    UiScrollOwnershipDefinition,
};
use ui_math::Axis;
use ui_tree::{UiNode, UiNodeKind, WidgetId};
use ui_widgets::{ScrollInputPolicies, ScrollInputPolicy, hscroll, vscroll, xy_scroll};

use super::context::UiDefinitionContext;

pub(super) fn form_scroll(
    widget_id: WidgetId,
    axis: UiScrollAxisDefinition,
    input: UiScrollInputDefinition,
    context: &UiDefinitionContext,
    formed: Vec<UiNode>,
) -> UiNode {
    let mut node = match axis {
        UiScrollAxisDefinition::Horizontal => hscroll(widget_id, context.theme.clone(), formed),
        UiScrollAxisDefinition::Vertical => vscroll(widget_id, context.theme.clone(), formed),
        UiScrollAxisDefinition::Both => xy_scroll(widget_id, context.theme.clone(), formed),
    };
    if let UiNodeKind::Scroll(scroll) = &mut node.kind {
        scroll.input_policies = scroll_input_policies(input);
    }
    node
}

pub(super) fn scroll_input_policies(input: UiScrollInputDefinition) -> ScrollInputPolicies {
    ScrollInputPolicies::new(
        scroll_input_policy(input.horizontal),
        scroll_input_policy(input.vertical),
    )
}

pub(super) fn scroll_input_policy(input: UiScrollInputPolicyDefinition) -> ScrollInputPolicy {
    match input {
        UiScrollInputPolicyDefinition::WheelOnly => ScrollInputPolicy::WheelOnly,
        UiScrollInputPolicyDefinition::MiddleDragOnly => ScrollInputPolicy::MiddleDragOnly,
        UiScrollInputPolicyDefinition::WheelAndMiddleDrag => ScrollInputPolicy::WheelAndMiddleDrag,
    }
}

pub(super) fn formed_scroll_owner(
    widget_id: WidgetId,
    axis: UiScrollAxisDefinition,
    ownership: UiScrollOwnershipDefinition,
) -> crate::FormedScrollOwner {
    crate::FormedScrollOwner {
        widget_id,
        axes: match axis {
            UiScrollAxisDefinition::Horizontal => vec![Axis::Horizontal],
            UiScrollAxisDefinition::Vertical => vec![Axis::Vertical],
            UiScrollAxisDefinition::Both => vec![Axis::Horizontal, Axis::Vertical],
        },
        boundary: ownership.boundary,
    }
}
