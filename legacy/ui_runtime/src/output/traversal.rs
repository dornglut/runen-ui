//! File: domain/ui/ui_runtime/src/output/traversal.rs
//! Purpose: Retained UI tree traversal for frame emission.

use crate::{ComputedLayoutMap, UiNode, UiNodeKind, UiTree};

use super::emit::containers::{
    emit_panel, emit_popup, emit_radial_menu, emit_scroll_begin, emit_scrollbar,
};
use super::emit::controls::{
    emit_button, emit_label, emit_numeric_input, emit_select, emit_table, emit_tabs,
    emit_text_input, emit_toggle, emit_tree,
};
use super::emit::graph_canvas::emit_graph_canvas;
use super::emit::surface::{
    emit_divider, emit_image, emit_product_surface, emit_viewport_surface_embed,
};
use super::interaction_visual::InteractionVisualState;
use super::primitives::sort_key;
use ui_math::UiSize;
use ui_render_data::{ClipPrimitive, UiLayer, UiPrimitive};
use ui_text::{FontAtlasSource, TextLayouter};

#[expect(
    clippy::too_many_arguments,
    reason = "private render-emission helpers keep explicit render state at each emission boundary"
)]
pub(super) fn emit_node(
    tree: &UiTree,
    node: &UiNode,
    layouts: &ComputedLayoutMap,
    layer: &mut UiLayer,
    surface_size: UiSize,
    atlas_source: &dyn FontAtlasSource,
    layouter: &dyn TextLayouter,
    interaction_state: InteractionVisualState,
    render_layer_order: u32,
    primitive_order: &mut u32,
) {
    let Some(layout) = layouts.get(&node.id) else {
        return;
    };
    let node_layer_order = match &node.kind {
        UiNodeKind::Popup(popup) => popup.layer_order,
        UiNodeKind::RadialMenu(radial) => radial.layer_order,
        UiNodeKind::OverlayAdornment(_) => render_layer_order,
        _ => render_layer_order,
    };

    match &node.kind {
        UiNodeKind::Panel(panel) => emit_panel(
            panel,
            layout.bounds,
            layout.content_bounds,
            layer,
            node_layer_order,
            primitive_order,
        ),
        UiNodeKind::Popup(popup) => emit_popup(
            popup,
            layout.bounds,
            layout.content_bounds,
            layer,
            node_layer_order,
            primitive_order,
        ),
        UiNodeKind::RadialMenu(radial) => emit_radial_menu(
            radial,
            layout.bounds,
            layout.content_bounds,
            layer,
            node_layer_order,
            primitive_order,
        ),
        UiNodeKind::OverlayAdornment(_) => {}
        UiNodeKind::Label(label) => emit_label(
            label,
            layout.bounds,
            layer,
            atlas_source,
            layouter,
            node_layer_order,
            primitive_order,
        ),
        UiNodeKind::Button(button) => emit_button(
            node.id,
            button,
            layout.bounds,
            layout.content_bounds,
            layer,
            atlas_source,
            layouter,
            interaction_state.clone(),
            node_layer_order,
            primitive_order,
        ),
        UiNodeKind::TextInput(text_input) => emit_text_input(
            node.id,
            text_input,
            layout.bounds,
            layout.content_bounds,
            layer,
            atlas_source,
            layouter,
            interaction_state.clone(),
            node_layer_order,
            primitive_order,
        ),
        UiNodeKind::Toggle(toggle) => emit_toggle(
            node.id,
            toggle,
            layout.bounds,
            layout.content_bounds,
            layer,
            atlas_source,
            layouter,
            interaction_state.clone(),
            node_layer_order,
            primitive_order,
        ),
        UiNodeKind::NumericInput(numeric) => emit_numeric_input(
            node.id,
            numeric,
            layout.bounds,
            layout.content_bounds,
            layer,
            atlas_source,
            layouter,
            interaction_state.clone(),
            node_layer_order,
            primitive_order,
        ),
        UiNodeKind::Tabs(tabs) => emit_tabs(
            node.id,
            tabs,
            layout.bounds,
            layout.content_bounds,
            layer,
            atlas_source,
            layouter,
            interaction_state.clone(),
            node_layer_order,
            primitive_order,
        ),
        UiNodeKind::Select(select) => emit_select(
            node.id,
            select,
            layout.bounds,
            layout.content_bounds,
            layer,
            atlas_source,
            layouter,
            interaction_state.clone(),
            node_layer_order,
            primitive_order,
        ),
        UiNodeKind::Table(table) => emit_table(
            node.id,
            table,
            layout.bounds,
            layer,
            atlas_source,
            layouter,
            interaction_state.clone(),
            node_layer_order,
            primitive_order,
        ),
        UiNodeKind::Tree(tree) => emit_tree(
            node.id,
            tree,
            layout.bounds,
            layer,
            atlas_source,
            layouter,
            interaction_state.clone(),
            node_layer_order,
            primitive_order,
        ),
        UiNodeKind::Spacer(_) => {}
        UiNodeKind::Divider(divider) => emit_divider(
            divider,
            layout.bounds,
            layer,
            node_layer_order,
            primitive_order,
        ),
        UiNodeKind::Image(image) => emit_image(
            image,
            layout.bounds,
            layer,
            node_layer_order,
            primitive_order,
        ),
        UiNodeKind::ProductSurface(surface) => emit_product_surface(
            surface,
            layout.bounds,
            layer,
            node_layer_order,
            primitive_order,
        ),
        UiNodeKind::GraphCanvas(graph_canvas) => emit_graph_canvas(
            node.id,
            graph_canvas,
            layout.bounds,
            layer,
            atlas_source,
            layouter,
            interaction_state.clone(),
            node_layer_order,
            primitive_order,
        ),
        UiNodeKind::ViewportSurfaceEmbed(embed) => emit_viewport_surface_embed(
            embed,
            layout.bounds,
            surface_size,
            layer,
            node_layer_order,
            primitive_order,
        ),
        UiNodeKind::Scroll(scroll) => emit_scroll_begin(
            scroll,
            layout.bounds,
            layout.content_bounds,
            layer,
            node_layer_order,
            primitive_order,
        ),
        UiNodeKind::Stack(_) | UiNodeKind::Split(_) => {}
    }

    for child in &node.children {
        emit_node(
            tree,
            child,
            layouts,
            layer,
            surface_size,
            atlas_source,
            layouter,
            interaction_state.clone(),
            node_layer_order,
            primitive_order,
        );
    }

    match &node.kind {
        UiNodeKind::Panel(_)
        | UiNodeKind::Popup(_)
        | UiNodeKind::RadialMenu(_)
        | UiNodeKind::Button(_)
        | UiNodeKind::TextInput(_)
        | UiNodeKind::NumericInput(_)
        | UiNodeKind::Tabs(_)
        | UiNodeKind::Select(_)
        | UiNodeKind::Table(_)
        | UiNodeKind::Tree(_) => {
            layer.push(UiPrimitive::Clip(ClipPrimitive::Pop {
                sort_key: sort_key(node_layer_order, *primitive_order),
            }));
            *primitive_order += 1;
        }
        UiNodeKind::Scroll(scroll) => {
            layer.push(UiPrimitive::Clip(ClipPrimitive::Pop {
                sort_key: sort_key(node_layer_order, *primitive_order),
            }));
            *primitive_order += 1;

            emit_scrollbar(
                tree,
                node,
                scroll,
                layouts,
                layout.bounds,
                layout.content_bounds,
                layer,
                interaction_state.clone(),
                node_layer_order,
                primitive_order,
            );
        }
        UiNodeKind::Label(_)
        | UiNodeKind::OverlayAdornment(_)
        | UiNodeKind::Toggle(_)
        | UiNodeKind::Spacer(_)
        | UiNodeKind::Divider(_)
        | UiNodeKind::Image(_)
        | UiNodeKind::ProductSurface(_)
        | UiNodeKind::GraphCanvas(_)
        | UiNodeKind::ViewportSurfaceEmbed(_)
        | UiNodeKind::Stack(_)
        | UiNodeKind::Split(_) => {}
    }
}
