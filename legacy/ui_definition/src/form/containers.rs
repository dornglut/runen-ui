use crate::UiAxisDefinition;
use ui_layout::SizePolicy;
use ui_math::UiSize;
use ui_tree::{UiNode, WidgetId};
use ui_widgets::{hdivider, hstack_with_policies, panel, split, vdivider, vstack_with_policies};

use super::context::UiDefinitionContext;
use super::resolve::axis_to_runtime;

pub(super) fn form_panel(
    widget_id: WidgetId,
    context: &UiDefinitionContext,
    children: Vec<UiNode>,
) -> UiNode {
    panel(widget_id, context.theme.clone(), children)
}

pub(super) fn form_row(
    widget_id: WidgetId,
    child_count: usize,
    context: &UiDefinitionContext,
    children: Vec<UiNode>,
) -> UiNode {
    hstack_with_policies(
        widget_id,
        context.theme.spacing.sm,
        vec![SizePolicy::Auto; child_count],
        children,
    )
}

pub(super) fn form_column(
    widget_id: WidgetId,
    child_count: usize,
    context: &UiDefinitionContext,
    children: Vec<UiNode>,
) -> UiNode {
    vstack_with_policies(
        widget_id,
        context.theme.spacing.xs,
        vec![SizePolicy::Auto; child_count],
        children,
    )
}

pub(super) fn form_horizontal_stack(
    widget_id: WidgetId,
    child_count: usize,
    context: &UiDefinitionContext,
    children: Vec<UiNode>,
) -> UiNode {
    hstack_with_policies(
        widget_id,
        context.theme.spacing.xs,
        vec![SizePolicy::Auto; child_count],
        children,
    )
}

pub(super) fn form_split(
    widget_id: WidgetId,
    axis: UiAxisDefinition,
    ratio: f32,
    context: &UiDefinitionContext,
    children: Vec<UiNode>,
) -> UiNode {
    split(
        widget_id,
        axis_to_runtime(axis),
        ratio,
        context.theme.spacing.sm,
        children,
    )
}

pub(super) fn form_spacer(widget_id: WidgetId) -> UiNode {
    ui_widgets::spacer(widget_id, UiSize::new(12.0, 4.0))
}

pub(super) fn form_separator(
    widget_id: WidgetId,
    axis: UiAxisDefinition,
    length: Option<f32>,
    thickness: f32,
    context: &UiDefinitionContext,
) -> UiNode {
    let length_policy = length.map(SizePolicy::Fixed).unwrap_or(SizePolicy::Auto);
    match axis {
        UiAxisDefinition::Horizontal => hdivider(
            widget_id,
            thickness,
            length_policy,
            context.theme.foreground,
        ),
        UiAxisDefinition::Vertical => vdivider(
            widget_id,
            thickness,
            length_policy,
            context.theme.foreground,
        ),
    }
}
