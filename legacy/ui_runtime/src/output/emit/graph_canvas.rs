//! File: domain/ui/ui_runtime/src/output/emit/graph_canvas.rs
//! Purpose: Graph-canvas primitive emission for UI frame output.

use crate::{GraphCanvasNode, LabelNode, WidgetId};
use ui_math::UiRect;
use ui_render_data::{
    BorderPrimitive, ClipPrimitive, GraphCanvasPrimitiveBatch, GraphCanvasPrimitiveRole,
    RectPrimitive, StrokePrimitive, UiLayer, UiPrimitive,
};
use ui_text::{FontAtlasSource, TextHorizontalAlign, TextLayouter};
use ui_theme::UiColor;

use super::super::build_ui_frame::InteractionVisualState;
use super::super::primitives::{
    brighten, darken, default_draw_key, paint_from_color, sort_key, with_alpha,
};
use super::super::text::ellipsis_text_layout;
use super::controls::emit_label;

#[expect(
    clippy::too_many_arguments,
    reason = "graph canvas emission maps a retained graph view model into concrete render primitives"
)]
pub(crate) fn emit_graph_canvas(
    widget_id: WidgetId,
    graph_canvas: &GraphCanvasNode,
    bounds: UiRect,
    layer: &mut UiLayer,
    atlas_source: &dyn FontAtlasSource,
    layouter: &dyn TextLayouter,
    interaction_state: InteractionVisualState,
    depth: u32,
    primitive_order: &mut u32,
) {
    let interaction = interaction_state.for_widget(widget_id);
    let viewport = interaction_state
        .graph_canvas_viewports
        .get(&widget_id)
        .copied()
        .unwrap_or(graph_canvas.canvas.viewport);
    let mut background = graph_canvas.theme.background_panel;
    if interaction.hovered {
        background = brighten(background, 1.03);
    }
    let mut border = graph_canvas.theme.border;
    if interaction.focused {
        border = brighten(graph_canvas.theme.accent, 1.04);
    }

    layer.push(UiPrimitive::Rect(RectPrimitive::new(
        bounds,
        graph_canvas.theme.radius.sm,
        paint_from_color(background),
        default_draw_key(),
        sort_key(depth, *primitive_order),
    )));
    *primitive_order += 1;
    layer.push(UiPrimitive::Border(BorderPrimitive::new(
        bounds,
        graph_canvas.theme.radius.sm,
        graph_canvas.theme.border_width,
        paint_from_color(border),
        default_draw_key(),
        sort_key(depth, *primitive_order),
    )));
    *primitive_order += 1;

    if graph_canvas.clip {
        layer.push(UiPrimitive::Clip(ClipPrimitive::Push {
            rect: bounds,
            sort_key: sort_key(depth, *primitive_order),
        }));
        *primitive_order += 1;
    }

    let mut batch = GraphCanvasPrimitiveBatch::new();
    emit_graph_grid(
        &mut batch,
        bounds,
        viewport,
        with_alpha(graph_canvas.theme.border, 0.28),
        with_alpha(graph_canvas.theme.accent, 0.18),
        depth,
        primitive_order,
    );
    for edge in &graph_canvas.canvas.edges {
        batch.push(
            GraphCanvasPrimitiveRole::Edge,
            StrokePrimitive::new(
                [
                    graph_point_to_ui(bounds, viewport, edge.from),
                    graph_point_to_ui(bounds, viewport, edge.to),
                ],
                if edge.selected { 3.0 } else { 2.0 },
                paint_from_color(if edge.selected {
                    graph_canvas.theme.accent
                } else {
                    graph_canvas.theme.border
                }),
                default_draw_key(),
                sort_key(depth, *primitive_order),
            )
            .with_clip(bounds),
        );
        *primitive_order += 1;
    }

    if let Some(gesture) = interaction_state.graph_canvas_gestures.get(&widget_id)
        && let Some(ui_graph_editor::GraphActiveGesture::ConnectionPreview(connection)) =
            gesture.active
    {
        batch.push(
            GraphCanvasPrimitiveRole::ConnectionPreview,
            StrokePrimitive::new(
                [
                    graph_point_to_ui(bounds, viewport, connection.start),
                    graph_point_to_ui(bounds, viewport, connection.current),
                ],
                2.0,
                paint_from_color(graph_canvas.theme.accent),
                default_draw_key(),
                sort_key(depth, *primitive_order),
            )
            .with_clip(bounds),
        );
        *primitive_order += 1;
    }

    for node in &graph_canvas.canvas.nodes {
        let rect = graph_rect_to_ui(bounds, viewport, node.rect);
        let header_rect = graph_rect_to_ui(
            bounds,
            viewport,
            ui_graph_editor::GraphRect::new(node.rect.x, node.rect.y, node.rect.width, 30),
        );
        batch.push(
            GraphCanvasPrimitiveRole::NodeBox,
            RectPrimitive::new(
                rect,
                graph_canvas.theme.radius.sm,
                paint_from_color(darken(graph_canvas.theme.background_panel, 0.92)),
                default_draw_key(),
                sort_key(depth, *primitive_order),
            ),
        );
        *primitive_order += 1;
        batch.push(
            GraphCanvasPrimitiveRole::NodeBox,
            RectPrimitive::new(
                header_rect,
                graph_canvas.theme.radius.sm,
                paint_from_color(with_alpha(graph_canvas.theme.accent, 0.28)),
                default_draw_key(),
                sort_key(depth, *primitive_order),
            ),
        );
        *primitive_order += 1;
        batch.push(
            GraphCanvasPrimitiveRole::NodeBox,
            BorderPrimitive::new(
                rect,
                graph_canvas.theme.radius.sm,
                graph_canvas.theme.border_width.max(1.0),
                paint_from_color(brighten(graph_canvas.theme.border, 1.12)),
                default_draw_key(),
                sort_key(depth, *primitive_order),
            ),
        );
        *primitive_order += 1;
        if node.selected || graph_canvas.canvas.selection.nodes.contains(&node.node) {
            batch.push(
                GraphCanvasPrimitiveRole::SelectionOutline,
                BorderPrimitive::new(
                    rect,
                    graph_canvas.theme.radius.sm,
                    graph_canvas.theme.border_width.max(1.0),
                    paint_from_color(graph_canvas.theme.accent),
                    default_draw_key(),
                    sort_key(depth, *primitive_order),
                ),
            );
            *primitive_order += 1;
        }
        let label_rect = UiRect::new(
            rect.x + graph_canvas.theme.spacing.xs,
            rect.y + graph_canvas.theme.spacing.xs,
            (rect.width - graph_canvas.theme.spacing.xs * 2.0).max(0.0),
            graph_canvas.text_style.font_size * 1.4,
        );
        let label = LabelNode {
            text: node.title.clone(),
            text_style: graph_canvas.text_style.clone(),
            text_layout: ellipsis_text_layout(TextHorizontalAlign::Start),
            constraints: ui_layout::LayoutConstraints::tight(label_rect.size()),
        };
        emit_label(
            &label,
            label_rect,
            layer,
            atlas_source,
            layouter,
            depth,
            primitive_order,
        );
    }

    for port in &graph_canvas.canvas.ports {
        let port_rect = graph_rect_to_ui(bounds, viewport, port.rect);
        batch.push(
            GraphCanvasPrimitiveRole::Port,
            RectPrimitive::new(
                port_rect,
                4.0,
                paint_from_color(match port.direction {
                    ui_graph_editor::GraphPortDirection::Input => {
                        brighten(graph_canvas.theme.border, 1.45)
                    }
                    ui_graph_editor::GraphPortDirection::Output => graph_canvas.theme.accent,
                }),
                default_draw_key(),
                sort_key(depth, *primitive_order),
            ),
        );
        *primitive_order += 1;
        let mut port_text_style = graph_canvas.text_style.clone();
        port_text_style.color[3] = (port_text_style.color[3] * 0.82).clamp(0.0, 1.0);
        let port_text_align = match port.direction {
            ui_graph_editor::GraphPortDirection::Input => TextHorizontalAlign::Start,
            ui_graph_editor::GraphPortDirection::Output => TextHorizontalAlign::End,
        };
        let label_width = 150.0;
        let label_rect = match port.direction {
            ui_graph_editor::GraphPortDirection::Input => UiRect::new(
                port_rect.x + port_rect.width + graph_canvas.theme.spacing.xs,
                port_rect.y - 3.0,
                label_width,
                graph_canvas.text_style.font_size * 1.4,
            ),
            ui_graph_editor::GraphPortDirection::Output => UiRect::new(
                port_rect.x - label_width - graph_canvas.theme.spacing.xs,
                port_rect.y - 3.0,
                label_width,
                graph_canvas.text_style.font_size * 1.4,
            ),
        };
        let label = LabelNode {
            text: port.label.clone(),
            text_style: port_text_style,
            text_layout: ellipsis_text_layout(port_text_align),
            constraints: ui_layout::LayoutConstraints::tight(label_rect.size()),
        };
        emit_label(
            &label,
            label_rect,
            layer,
            atlas_source,
            layouter,
            depth,
            primitive_order,
        );
    }

    for overlay in &graph_canvas.canvas.overlays {
        batch.push(
            GraphCanvasPrimitiveRole::Overlay,
            BorderPrimitive::new(
                graph_rect_to_ui(bounds, viewport, overlay.rect),
                graph_canvas.theme.radius.sm,
                if overlay.active {
                    graph_canvas.theme.border_width.max(2.0)
                } else {
                    graph_canvas.theme.border_width.max(1.0)
                },
                paint_from_color(match overlay.severity {
                    ui_graph_editor::GraphOverlaySeverity::Info => graph_canvas.theme.accent,
                    ui_graph_editor::GraphOverlaySeverity::Warning => {
                        brighten(graph_canvas.theme.accent, 1.20)
                    }
                    ui_graph_editor::GraphOverlaySeverity::Error => {
                        UiColor::new(1.0, 0.12, 0.18, 1.0)
                    }
                }),
                default_draw_key(),
                sort_key(depth, *primitive_order),
            ),
        );
        *primitive_order += 1;
    }

    for primitive in batch.into_ui_primitives() {
        layer.push(primitive);
    }

    if graph_canvas.clip {
        layer.push(UiPrimitive::Clip(ClipPrimitive::Pop {
            sort_key: sort_key(depth, *primitive_order),
        }));
        *primitive_order += 1;
    }
}

fn emit_graph_grid(
    batch: &mut GraphCanvasPrimitiveBatch,
    bounds: UiRect,
    viewport: ui_graph_editor::GraphViewport,
    minor_color: UiColor,
    major_color: UiColor,
    depth: u32,
    primitive_order: &mut u32,
) {
    const GRID_STEP: i32 = 64;
    const MAJOR_EVERY: i32 = 4;
    const MAX_LINES_PER_AXIS: i32 = 96;

    let zoom = viewport.zoom_milli.max(1) as f32 / 1000.0;
    let min_x = ((-viewport.pan_x as f32) / zoom).floor() as i32;
    let max_x = (((bounds.width - viewport.pan_x as f32) / zoom).ceil() as i32).max(min_x);
    let min_y = ((-viewport.pan_y as f32) / zoom).floor() as i32;
    let max_y = (((bounds.height - viewport.pan_y as f32) / zoom).ceil() as i32).max(min_y);
    let first_x = floor_to_grid(min_x, GRID_STEP);
    let first_y = floor_to_grid(min_y, GRID_STEP);

    let mut x = first_x;
    let mut x_count = 0;
    while x <= max_x && x_count < MAX_LINES_PER_AXIS {
        let major = (x / GRID_STEP).rem_euclid(MAJOR_EVERY) == 0;
        batch.push(
            GraphCanvasPrimitiveRole::Edge,
            StrokePrimitive::new(
                [
                    graph_point_to_ui(bounds, viewport, ui_graph_editor::GraphPoint::new(x, min_y)),
                    graph_point_to_ui(bounds, viewport, ui_graph_editor::GraphPoint::new(x, max_y)),
                ],
                if major { 1.1 } else { 0.7 },
                paint_from_color(if major { major_color } else { minor_color }),
                default_draw_key(),
                sort_key(depth, *primitive_order),
            )
            .with_clip(bounds),
        );
        *primitive_order += 1;
        x += GRID_STEP;
        x_count += 1;
    }

    let mut y = first_y;
    let mut y_count = 0;
    while y <= max_y && y_count < MAX_LINES_PER_AXIS {
        let major = (y / GRID_STEP).rem_euclid(MAJOR_EVERY) == 0;
        batch.push(
            GraphCanvasPrimitiveRole::Edge,
            StrokePrimitive::new(
                [
                    graph_point_to_ui(bounds, viewport, ui_graph_editor::GraphPoint::new(min_x, y)),
                    graph_point_to_ui(bounds, viewport, ui_graph_editor::GraphPoint::new(max_x, y)),
                ],
                if major { 1.1 } else { 0.7 },
                paint_from_color(if major { major_color } else { minor_color }),
                default_draw_key(),
                sort_key(depth, *primitive_order),
            )
            .with_clip(bounds),
        );
        *primitive_order += 1;
        y += GRID_STEP;
        y_count += 1;
    }
}

fn floor_to_grid(value: i32, step: i32) -> i32 {
    value.div_euclid(step) * step
}

fn graph_rect_to_ui(
    canvas_bounds: UiRect,
    viewport: ui_graph_editor::GraphViewport,
    rect: ui_graph_editor::GraphRect,
) -> UiRect {
    let origin = graph_point_to_ui(
        canvas_bounds,
        viewport,
        ui_graph_editor::GraphPoint::new(rect.x, rect.y),
    );
    let zoom = viewport.zoom_milli.max(1) as f32 / 1000.0;
    UiRect::new(
        origin.x,
        origin.y,
        rect.width.max(0) as f32 * zoom,
        rect.height.max(0) as f32 * zoom,
    )
}

fn graph_point_to_ui(
    canvas_bounds: UiRect,
    viewport: ui_graph_editor::GraphViewport,
    point: ui_graph_editor::GraphPoint,
) -> ui_math::UiPoint {
    let zoom = viewport.zoom_milli.max(1) as f32 / 1000.0;
    ui_math::UiPoint::new(
        canvas_bounds.x + viewport.pan_x as f32 + point.x as f32 * zoom,
        canvas_bounds.y + viewport.pan_y as f32 + point.y as f32 * zoom,
    )
}
