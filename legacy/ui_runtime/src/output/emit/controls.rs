//! File: domain/ui/ui_runtime/src/output/emit/controls.rs
//! Purpose: Interactive control, table, tree, and text-label emission for UI frame output.

use crate::{
    ButtonNode, LabelNode, NumericInputNode, SelectNode, TableNode, TabsNode, TextInputNode,
    ToggleNode, TreeNode, WidgetId,
};
use ui_math::UiRect;
use ui_render_data::{
    BorderPrimitive, ClipPrimitive, GlyphRunPrimitive, RectPrimitive, UiDrawKey, UiLayer, UiPaint,
    UiPrimitive,
};
use ui_text::{FontAtlasSource, TextBlockId, TextHorizontalAlign, TextLayoutPolicy, TextLayouter};

use super::super::build_ui_frame::InteractionVisualState;
use super::super::primitives::{
    brighten, darken, default_draw_key, paint_from_color, sort_key, with_alpha,
};
use super::super::text::{clipped_text_layout, ellipsis_text_layout};

#[expect(
    clippy::too_many_arguments,
    reason = "private render-emission helpers keep explicit render state at each emission boundary"
)]
pub(crate) fn emit_button(
    widget_id: WidgetId,
    button: &ButtonNode,
    bounds: UiRect,
    content_bounds: UiRect,
    layer: &mut UiLayer,
    atlas_source: &dyn FontAtlasSource,
    layouter: &dyn TextLayouter,
    interaction_state: InteractionVisualState,
    depth: u32,
    primitive_order: &mut u32,
) {
    let interaction = interaction_state.for_widget(widget_id);
    if let Some(anchor) = button.reveal_on_hover_anchor {
        let anchor_interaction = interaction_state.for_widget(anchor);
        if !(interaction.hovered
            || interaction.pressed
            || anchor_interaction.hovered
            || anchor_interaction.pressed)
        {
            return;
        }
    }
    let mut background = if button.enabled {
        if button.selected {
            button.selected_fill.unwrap_or(button.theme.accent)
        } else {
            button.theme.background_panel
        }
    } else {
        with_alpha(button.theme.border, 0.35)
    };
    if interaction.hovered {
        background = brighten(background, 1.08);
    }
    if interaction.pressed {
        background = darken(background, 0.88);
    }

    let mut border = if button.selected {
        button.selected_border.unwrap_or(button.theme.accent)
    } else {
        button.theme.border
    };
    if interaction.focused {
        border = brighten(button.theme.accent, 1.04);
    } else if interaction.hovered {
        border = brighten(border, 1.08);
    }

    let radius = button_radius(button, bounds);
    layer.push(UiPrimitive::Rect(RectPrimitive::new(
        bounds,
        radius,
        paint_from_color(background),
        default_draw_key(),
        sort_key(depth, *primitive_order),
    )));
    *primitive_order += 1;

    layer.push(UiPrimitive::Border(BorderPrimitive::new(
        bounds,
        radius,
        button.theme.border_width,
        paint_from_color(border),
        default_draw_key(),
        sort_key(depth, *primitive_order),
    )));
    *primitive_order += 1;

    layer.push(UiPrimitive::Clip(ClipPrimitive::Push {
        rect: content_bounds,
        sort_key: sort_key(depth, *primitive_order),
    }));
    *primitive_order += 1;

    let text_rect = UiRect::new(
        content_bounds.x,
        content_bounds.y,
        content_bounds.width,
        content_bounds.height,
    );

    emit_button_label(
        &button.label,
        &button_text_style(button, interaction),
        button.text_layout,
        text_rect,
        layer,
        atlas_source,
        layouter,
        depth,
        primitive_order,
    );
}

fn button_radius(button: &ButtonNode, bounds: UiRect) -> f32 {
    button
        .corner_radius
        .unwrap_or(button.theme.radius.sm)
        .min(bounds.width.max(0.0) * 0.5)
        .min(bounds.height.max(0.0) * 0.5)
        .max(0.0)
}

#[expect(
    clippy::too_many_arguments,
    reason = "button label emission mirrors the surrounding primitive emission boundary"
)]
fn emit_button_label(
    text: &str,
    text_style: &ui_text::TextStyle,
    text_layout: TextLayoutPolicy,
    bounds: UiRect,
    layer: &mut UiLayer,
    atlas_source: &dyn FontAtlasSource,
    layouter: &dyn TextLayouter,
    depth: u32,
    primitive_order: &mut u32,
) {
    let layout = crate::text_emission::layout_text_in_bounds(
        atlas_source,
        layouter,
        TextBlockId(u64::from(*primitive_order) + 1),
        text,
        text_style,
        bounds,
        text_layout,
    );

    layer.push(UiPrimitive::GlyphRun(GlyphRunPrimitive::new(
        layout,
        Some(bounds),
        UiPaint::rgba(
            text_style.color[0],
            text_style.color[1],
            text_style.color[2],
            text_style.color[3],
        ),
        UiDrawKey::new(0, Some(text_style.font_id.0)),
        sort_key(depth, *primitive_order),
    )));
    *primitive_order += 1;
}

#[expect(
    clippy::too_many_arguments,
    reason = "private render-emission helpers keep explicit render state at each emission boundary"
)]
pub(crate) fn emit_text_input(
    widget_id: WidgetId,
    text_input: &TextInputNode,
    bounds: UiRect,
    content_bounds: UiRect,
    layer: &mut UiLayer,
    atlas_source: &dyn FontAtlasSource,
    layouter: &dyn TextLayouter,
    interaction_state: InteractionVisualState,
    depth: u32,
    primitive_order: &mut u32,
) {
    let interaction = interaction_state.for_widget(widget_id);
    let mut background = if text_input.editable {
        text_input.theme.background_panel
    } else {
        with_alpha(text_input.theme.border, 0.30)
    };
    if interaction.hovered {
        background = brighten(background, 1.04);
    }
    if interaction.focused {
        background = brighten(background, 1.06);
    }

    let mut border = text_input.theme.border;
    if interaction.focused {
        border = brighten(text_input.theme.accent, 1.03);
    } else if interaction.hovered {
        border = brighten(border, 1.08);
    }

    layer.push(UiPrimitive::Rect(RectPrimitive::new(
        bounds,
        text_input.theme.radius.sm,
        paint_from_color(background),
        default_draw_key(),
        sort_key(depth, *primitive_order),
    )));
    *primitive_order += 1;
    layer.push(UiPrimitive::Border(BorderPrimitive::new(
        bounds,
        text_input.theme.radius.sm,
        text_input.theme.border_width,
        paint_from_color(border),
        default_draw_key(),
        sort_key(depth, *primitive_order),
    )));
    *primitive_order += 1;
    layer.push(UiPrimitive::Clip(ClipPrimitive::Push {
        rect: content_bounds,
        sort_key: sort_key(depth, *primitive_order),
    }));
    *primitive_order += 1;

    let mut text_style = text_input.text_style.clone();
    let text = if text_input.value.is_empty() {
        text_style.color[3] = (text_style.color[3] * 0.6).clamp(0.0, 1.0);
        text_input.placeholder.clone()
    } else {
        text_input.value.clone()
    };
    let label = LabelNode {
        text,
        text_style,
        text_layout: clipped_text_layout(TextHorizontalAlign::Start),
        constraints: ui_layout::LayoutConstraints::tight(content_bounds.size()),
    };
    emit_label(
        &label,
        content_bounds,
        layer,
        atlas_source,
        layouter,
        depth,
        primitive_order,
    );
}

#[expect(
    clippy::too_many_arguments,
    reason = "private render-emission helpers keep explicit render state at each emission boundary"
)]
pub(crate) fn emit_toggle(
    widget_id: WidgetId,
    toggle: &ToggleNode,
    bounds: UiRect,
    content_bounds: UiRect,
    layer: &mut UiLayer,
    atlas_source: &dyn FontAtlasSource,
    layouter: &dyn TextLayouter,
    interaction_state: InteractionVisualState,
    depth: u32,
    primitive_order: &mut u32,
) {
    let interaction = interaction_state.for_widget(widget_id);
    let mut background = if toggle.enabled {
        toggle.theme.background_panel
    } else {
        with_alpha(toggle.theme.border, 0.30)
    };
    if interaction.hovered {
        background = brighten(background, 1.04);
    }
    let mut border = toggle.theme.border;
    if interaction.focused {
        border = brighten(toggle.theme.accent, 1.03);
    } else if interaction.hovered {
        border = brighten(border, 1.08);
    }

    layer.push(UiPrimitive::Rect(RectPrimitive::new(
        bounds,
        toggle.theme.radius.sm,
        paint_from_color(background),
        default_draw_key(),
        sort_key(depth, *primitive_order),
    )));
    *primitive_order += 1;
    layer.push(UiPrimitive::Border(BorderPrimitive::new(
        bounds,
        toggle.theme.radius.sm,
        toggle.theme.border_width,
        paint_from_color(border),
        default_draw_key(),
        sort_key(depth, *primitive_order),
    )));
    *primitive_order += 1;

    let indicator_size = content_bounds.height.min(content_bounds.width).max(0.0);
    let indicator_rect = UiRect::new(
        content_bounds.x,
        content_bounds.y,
        indicator_size,
        indicator_size,
    );
    let mut indicator_color = if toggle.checked {
        toggle.theme.accent
    } else {
        toggle.theme.border
    };
    if !toggle.enabled {
        indicator_color.a = (indicator_color.a * 0.5).clamp(0.0, 1.0);
    }
    layer.push(UiPrimitive::Rect(RectPrimitive::new(
        indicator_rect,
        toggle.theme.radius.sm.min(indicator_size * 0.4),
        paint_from_color(indicator_color),
        default_draw_key(),
        sort_key(depth, *primitive_order),
    )));
    *primitive_order += 1;

    let text_bounds = UiRect::new(
        indicator_rect.x + indicator_rect.width + toggle.theme.spacing.sm,
        content_bounds.y,
        (content_bounds.width - indicator_rect.width - toggle.theme.spacing.sm).max(0.0),
        content_bounds.height,
    );
    let label = LabelNode {
        text: toggle.label.clone(),
        text_style: toggle.text_style.clone(),
        text_layout: ellipsis_text_layout(TextHorizontalAlign::Start),
        constraints: ui_layout::LayoutConstraints::tight(text_bounds.size()),
    };
    emit_label(
        &label,
        text_bounds,
        layer,
        atlas_source,
        layouter,
        depth,
        primitive_order,
    );
}

#[expect(
    clippy::too_many_arguments,
    reason = "private render-emission helpers keep explicit render state at each emission boundary"
)]
pub(crate) fn emit_numeric_input(
    widget_id: WidgetId,
    numeric: &NumericInputNode,
    bounds: UiRect,
    content_bounds: UiRect,
    layer: &mut UiLayer,
    atlas_source: &dyn FontAtlasSource,
    layouter: &dyn TextLayouter,
    interaction_state: InteractionVisualState,
    depth: u32,
    primitive_order: &mut u32,
) {
    let interaction = interaction_state.for_widget(widget_id);
    let mut background = if numeric.enabled {
        numeric.theme.background_panel
    } else {
        with_alpha(numeric.theme.border, 0.30)
    };
    if interaction.hovered {
        background = brighten(background, 1.04);
    }
    let mut border = numeric.theme.border;
    if interaction.focused {
        border = brighten(numeric.theme.accent, 1.03);
    } else if interaction.hovered {
        border = brighten(border, 1.08);
    }

    layer.push(UiPrimitive::Rect(RectPrimitive::new(
        bounds,
        numeric.theme.radius.sm,
        paint_from_color(background),
        default_draw_key(),
        sort_key(depth, *primitive_order),
    )));
    *primitive_order += 1;
    layer.push(UiPrimitive::Border(BorderPrimitive::new(
        bounds,
        numeric.theme.radius.sm,
        numeric.theme.border_width,
        paint_from_color(border),
        default_draw_key(),
        sort_key(depth, *primitive_order),
    )));
    *primitive_order += 1;
    layer.push(UiPrimitive::Clip(ClipPrimitive::Push {
        rect: content_bounds,
        sort_key: sort_key(depth, *primitive_order),
    }));
    *primitive_order += 1;

    let value_text = format!("{:.*}", usize::from(numeric.precision), numeric.value);
    let label = LabelNode {
        text: value_text,
        text_style: numeric.text_style.clone(),
        text_layout: clipped_text_layout(TextHorizontalAlign::Start),
        constraints: ui_layout::LayoutConstraints::tight(content_bounds.size()),
    };
    emit_label(
        &label,
        content_bounds,
        layer,
        atlas_source,
        layouter,
        depth,
        primitive_order,
    );
}

#[expect(
    clippy::too_many_arguments,
    reason = "private render-emission helpers keep explicit render state at each emission boundary"
)]
pub(crate) fn emit_tabs(
    widget_id: WidgetId,
    tabs: &TabsNode,
    bounds: UiRect,
    content_bounds: UiRect,
    layer: &mut UiLayer,
    atlas_source: &dyn FontAtlasSource,
    layouter: &dyn TextLayouter,
    interaction_state: InteractionVisualState,
    depth: u32,
    primitive_order: &mut u32,
) {
    let interaction = interaction_state.for_widget(widget_id);
    let mut background = tabs.theme.background_panel;
    if interaction.hovered {
        background = brighten(background, 1.04);
    }
    let mut border = tabs.theme.border;
    if interaction.focused {
        border = brighten(tabs.theme.accent, 1.03);
    } else if interaction.hovered {
        border = brighten(border, 1.08);
    }

    layer.push(UiPrimitive::Rect(RectPrimitive::new(
        bounds,
        tabs.theme.radius.sm,
        paint_from_color(background),
        default_draw_key(),
        sort_key(depth, *primitive_order),
    )));
    *primitive_order += 1;
    layer.push(UiPrimitive::Border(BorderPrimitive::new(
        bounds,
        tabs.theme.radius.sm,
        tabs.theme.border_width,
        paint_from_color(border),
        default_draw_key(),
        sort_key(depth, *primitive_order),
    )));
    *primitive_order += 1;
    layer.push(UiPrimitive::Clip(ClipPrimitive::Push {
        rect: content_bounds,
        sort_key: sort_key(depth, *primitive_order),
    }));
    *primitive_order += 1;

    if tabs.labels.is_empty() {
        return;
    }

    let segment_width = content_bounds.width / tabs.labels.len() as f32;
    let selected = tabs.selected_index.min(tabs.labels.len() - 1);
    let selected_rect = UiRect::new(
        content_bounds.x + segment_width * selected as f32,
        content_bounds.y,
        segment_width.max(0.0),
        content_bounds.height.max(0.0),
    );
    layer.push(UiPrimitive::Rect(RectPrimitive::new(
        selected_rect,
        tabs.theme.radius.sm.min(selected_rect.height * 0.5),
        paint_from_color(tabs.theme.accent),
        default_draw_key(),
        sort_key(depth, *primitive_order),
    )));
    *primitive_order += 1;

    for (index, label_text) in tabs.labels.iter().enumerate() {
        let tab_bounds = UiRect::new(
            content_bounds.x + segment_width * index as f32,
            content_bounds.y,
            segment_width.max(0.0),
            content_bounds.height.max(0.0),
        );
        let mut style = tabs.text_style.clone();
        if index != selected {
            style.color[3] = (style.color[3] * 0.7).clamp(0.0, 1.0);
        }
        let label = LabelNode {
            text: label_text.clone(),
            text_style: style,
            text_layout: ellipsis_text_layout(TextHorizontalAlign::Center),
            constraints: ui_layout::LayoutConstraints::tight(tab_bounds.size()),
        };
        emit_label(
            &label,
            tab_bounds,
            layer,
            atlas_source,
            layouter,
            depth,
            primitive_order,
        );
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "private render-emission helpers keep explicit render state at each emission boundary"
)]
pub(crate) fn emit_select(
    widget_id: WidgetId,
    select: &SelectNode,
    bounds: UiRect,
    content_bounds: UiRect,
    layer: &mut UiLayer,
    atlas_source: &dyn FontAtlasSource,
    layouter: &dyn TextLayouter,
    interaction_state: InteractionVisualState,
    depth: u32,
    primitive_order: &mut u32,
) {
    let interaction = interaction_state.for_widget(widget_id);
    let mut background = if select.enabled {
        select.theme.background_panel
    } else {
        with_alpha(select.theme.border, 0.30)
    };
    if interaction.hovered {
        background = brighten(background, 1.04);
    }
    let mut border = select.theme.border;
    if interaction.focused {
        border = brighten(select.theme.accent, 1.03);
    } else if interaction.hovered {
        border = brighten(border, 1.08);
    }
    layer.push(UiPrimitive::Rect(RectPrimitive::new(
        bounds,
        select.theme.radius.sm,
        paint_from_color(background),
        default_draw_key(),
        sort_key(depth, *primitive_order),
    )));
    *primitive_order += 1;
    layer.push(UiPrimitive::Border(BorderPrimitive::new(
        bounds,
        select.theme.radius.sm,
        select.theme.border_width,
        paint_from_color(border),
        default_draw_key(),
        sort_key(depth, *primitive_order),
    )));
    *primitive_order += 1;
    layer.push(UiPrimitive::Clip(ClipPrimitive::Push {
        rect: content_bounds,
        sort_key: sort_key(depth, *primitive_order),
    }));
    *primitive_order += 1;

    let text = select
        .selected_index
        .and_then(|index| select.options.get(index))
        .cloned()
        .unwrap_or_else(|| select.placeholder.clone());
    let label = LabelNode {
        text: format!("{text} v"),
        text_style: select.text_style.clone(),
        text_layout: ellipsis_text_layout(TextHorizontalAlign::Start),
        constraints: ui_layout::LayoutConstraints::tight(content_bounds.size()),
    };
    emit_label(
        &label,
        content_bounds,
        layer,
        atlas_source,
        layouter,
        depth,
        primitive_order,
    );
}

#[expect(
    clippy::too_many_arguments,
    reason = "private render-emission helpers keep explicit render state at each emission boundary"
)]
pub(crate) fn emit_table(
    widget_id: WidgetId,
    table: &TableNode,
    bounds: UiRect,
    layer: &mut UiLayer,
    atlas_source: &dyn FontAtlasSource,
    layouter: &dyn TextLayouter,
    interaction_state: InteractionVisualState,
    depth: u32,
    primitive_order: &mut u32,
) {
    let interaction = interaction_state.for_widget(widget_id);
    let mut background = table.theme.background_panel;
    if interaction.hovered {
        background = brighten(background, 1.03);
    }
    let mut border = table.theme.border;
    if interaction.focused {
        border = brighten(table.theme.accent, 1.03);
    } else if interaction.hovered {
        border = brighten(border, 1.08);
    }
    layer.push(UiPrimitive::Rect(RectPrimitive::new(
        bounds,
        table.theme.radius.sm,
        paint_from_color(background),
        default_draw_key(),
        sort_key(depth, *primitive_order),
    )));
    *primitive_order += 1;
    layer.push(UiPrimitive::Border(BorderPrimitive::new(
        bounds,
        table.theme.radius.sm,
        table.theme.border_width,
        paint_from_color(border),
        default_draw_key(),
        sort_key(depth, *primitive_order),
    )));
    *primitive_order += 1;
    layer.push(UiPrimitive::Clip(ClipPrimitive::Push {
        rect: bounds,
        sort_key: sort_key(depth, *primitive_order),
    }));
    *primitive_order += 1;

    let column_widths = table_column_widths(table, bounds.width);
    let header_rect = UiRect::new(bounds.x, bounds.y, bounds.width, table.row_height);
    let mut header_color = table.theme.border;
    header_color.a = (header_color.a * 0.45).clamp(0.0, 1.0);
    layer.push(UiPrimitive::Rect(RectPrimitive::new(
        header_rect,
        0.0,
        paint_from_color(header_color),
        default_draw_key(),
        sort_key(depth, *primitive_order),
    )));
    *primitive_order += 1;

    emit_table_cells(
        table
            .columns
            .iter()
            .map(|column| column.label.as_str())
            .collect::<Vec<_>>()
            .as_slice(),
        &column_widths,
        header_rect,
        &table.header_text_style,
        layer,
        atlas_source,
        layouter,
        depth,
        primitive_order,
    );

    for (row_index, row) in table.rows.iter().enumerate() {
        let row_rect = UiRect::new(
            bounds.x,
            bounds.y + table.row_height * (row_index as f32 + 1.0),
            bounds.width,
            table.row_height,
        );
        if row.selected {
            let mut selected = table.theme.accent;
            selected.a = (selected.a * 0.55).clamp(0.0, 1.0);
            layer.push(UiPrimitive::Rect(RectPrimitive::new(
                row_rect,
                0.0,
                paint_from_color(selected),
                default_draw_key(),
                sort_key(depth, *primitive_order),
            )));
            *primitive_order += 1;
        }
        let cells = row.cells.iter().map(String::as_str).collect::<Vec<_>>();
        emit_table_cells(
            &cells,
            &column_widths,
            row_rect,
            &table.text_style,
            layer,
            atlas_source,
            layouter,
            depth,
            primitive_order,
        );
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "private render-emission helpers keep explicit render state at each emission boundary"
)]
pub(crate) fn emit_tree(
    widget_id: WidgetId,
    tree: &TreeNode,
    bounds: UiRect,
    layer: &mut UiLayer,
    atlas_source: &dyn FontAtlasSource,
    layouter: &dyn TextLayouter,
    interaction_state: InteractionVisualState,
    depth: u32,
    primitive_order: &mut u32,
) {
    let interaction = interaction_state.for_widget(widget_id);
    let mut background = tree.theme.background_panel;
    if interaction.hovered {
        background = brighten(background, 1.03);
    }
    let mut border = tree.theme.border;
    if interaction.focused {
        border = brighten(tree.theme.accent, 1.03);
    } else if interaction.hovered {
        border = brighten(border, 1.08);
    }
    layer.push(UiPrimitive::Rect(RectPrimitive::new(
        bounds,
        tree.theme.radius.sm,
        paint_from_color(background),
        default_draw_key(),
        sort_key(depth, *primitive_order),
    )));
    *primitive_order += 1;
    layer.push(UiPrimitive::Border(BorderPrimitive::new(
        bounds,
        tree.theme.radius.sm,
        tree.theme.border_width,
        paint_from_color(border),
        default_draw_key(),
        sort_key(depth, *primitive_order),
    )));
    *primitive_order += 1;
    layer.push(UiPrimitive::Clip(ClipPrimitive::Push {
        rect: bounds,
        sort_key: sort_key(depth, *primitive_order),
    }));
    *primitive_order += 1;

    for (index, row) in tree.rows.iter().enumerate() {
        let row_rect = UiRect::new(
            bounds.x,
            bounds.y + tree.row_height * index as f32,
            bounds.width,
            tree.row_height,
        );
        if row.selected {
            let mut selected = tree.theme.accent;
            selected.a = (selected.a * 0.55).clamp(0.0, 1.0);
            layer.push(UiPrimitive::Rect(RectPrimitive::new(
                row_rect,
                0.0,
                paint_from_color(selected),
                default_draw_key(),
                sort_key(depth, *primitive_order),
            )));
            *primitive_order += 1;
        }
        let marker = if row.has_children {
            if row.expanded { "v" } else { ">" }
        } else {
            " "
        };
        let text = format!(
            "{}{marker} {}",
            " ".repeat(row.depth.saturating_mul(2)),
            row.label
        );
        let label_bounds = UiRect::new(
            row_rect.x + tree.theme.spacing.xs,
            row_rect.y,
            (row_rect.width - tree.theme.spacing.xs * 2.0).max(0.0),
            row_rect.height,
        );
        let label = LabelNode {
            text,
            text_style: tree.text_style.clone(),
            text_layout: ellipsis_text_layout(TextHorizontalAlign::Start),
            constraints: ui_layout::LayoutConstraints::tight(label_bounds.size()),
        };
        emit_label(
            &label,
            label_bounds,
            layer,
            atlas_source,
            layouter,
            depth,
            primitive_order,
        );
    }
}

fn table_column_widths(table: &TableNode, available_width: f32) -> Vec<f32> {
    if table.columns.is_empty() {
        return Vec::new();
    }
    let minimum = table
        .columns
        .iter()
        .map(|column| column.min_width)
        .collect::<Vec<_>>();
    let minimum_sum = minimum.iter().sum::<f32>().max(1.0);
    let scale = (available_width.max(minimum_sum)) / minimum_sum;
    minimum.into_iter().map(|width| width * scale).collect()
}

#[expect(
    clippy::too_many_arguments,
    reason = "private render-emission helpers keep explicit render state at each emission boundary"
)]
fn emit_table_cells(
    cells: &[&str],
    column_widths: &[f32],
    row_rect: UiRect,
    text_style: &ui_text::TextStyle,
    layer: &mut UiLayer,
    atlas_source: &dyn FontAtlasSource,
    layouter: &dyn TextLayouter,
    depth: u32,
    primitive_order: &mut u32,
) {
    let mut x = row_rect.x;
    for (column_index, width) in column_widths.iter().copied().enumerate() {
        let cell_text = cells.get(column_index).copied().unwrap_or("");
        let cell_bounds = UiRect::new(x + 4.0, row_rect.y, (width - 8.0).max(0.0), row_rect.height);
        let label = LabelNode {
            text: cell_text.to_string(),
            text_style: text_style.clone(),
            text_layout: ellipsis_text_layout(TextHorizontalAlign::Start),
            constraints: ui_layout::LayoutConstraints::tight(cell_bounds.size()),
        };
        emit_label(
            &label,
            cell_bounds,
            layer,
            atlas_source,
            layouter,
            depth,
            primitive_order,
        );
        x += width;
    }
}

pub(crate) fn emit_label(
    label: &LabelNode,
    bounds: UiRect,
    layer: &mut UiLayer,
    atlas_source: &dyn FontAtlasSource,
    layouter: &dyn TextLayouter,
    depth: u32,
    primitive_order: &mut u32,
) {
    let layout = crate::text_emission::layout_text_in_bounds(
        atlas_source,
        layouter,
        TextBlockId(u64::from(*primitive_order) + 1),
        &label.text,
        &label.text_style,
        bounds,
        label.text_layout,
    );

    layer.push(UiPrimitive::GlyphRun(GlyphRunPrimitive::new(
        layout,
        Some(bounds),
        UiPaint::rgba(
            label.text_style.color[0],
            label.text_style.color[1],
            label.text_style.color[2],
            label.text_style.color[3],
        ),
        UiDrawKey::new(0, Some(label.text_style.font_id.0)),
        sort_key(depth, *primitive_order),
    )));
    *primitive_order += 1;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct WidgetInteraction {
    pub(crate) hovered: bool,
    pub(crate) pressed: bool,
    pub(crate) focused: bool,
}

impl InteractionVisualState {
    pub(crate) fn for_widget(&self, widget_id: WidgetId) -> WidgetInteraction {
        WidgetInteraction {
            hovered: self.hovered_widget == Some(widget_id),
            pressed: self.pressed_widget == Some(widget_id),
            focused: self.focused_widget == Some(widget_id),
        }
    }
}

fn button_text_style(button: &ButtonNode, interaction: WidgetInteraction) -> ui_text::TextStyle {
    let mut text_style = button.text_style.clone();
    if !button.enabled {
        text_style.color[3] = (text_style.color[3] * 0.55).clamp(0.0, 1.0);
        return text_style;
    }
    if button.selected || interaction.pressed {
        text_style.color[0] = (text_style.color[0] + 0.08).clamp(0.0, 1.0);
        text_style.color[1] = (text_style.color[1] + 0.08).clamp(0.0, 1.0);
        text_style.color[2] = (text_style.color[2] + 0.08).clamp(0.0, 1.0);
    } else if interaction.hovered {
        text_style.color[3] = (text_style.color[3] * 0.95).clamp(0.0, 1.0);
    }
    text_style
}
