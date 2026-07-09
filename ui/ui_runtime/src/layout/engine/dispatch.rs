//! File: domain/ui/ui_runtime/src/layout/engine/dispatch.rs
//! Purpose: Dispatch retained node layout to responsibility-owned modules.

use ui_math::{UiRect, UiSize};

use crate::{ComputedLayoutMap, UiNode, UiNodeKind, UiRuntimeState};

use super::containers::{layout_divider, layout_panel, layout_spacer, layout_split, layout_stack};
use super::controls::{
    layout_button, layout_label, layout_numeric_input, layout_select, layout_table, layout_tabs,
    layout_text_input, layout_toggle, layout_tree,
};
use super::overlay::layout_overlay_adornment;
use super::popup::{layout_popup, layout_radial_menu};
use super::scroll::layout_scroll;
use super::surface::{
    layout_graph_canvas, layout_image, layout_product_surface, layout_viewport_surface_embed,
};

pub(super) fn layout_node(
    node: &UiNode,
    bounds: UiRect,
    state: &UiRuntimeState,
    out: &mut ComputedLayoutMap,
) -> UiSize {
    match &node.kind {
        UiNodeKind::Panel(panel) => layout_panel(node, panel, bounds, state, out),
        UiNodeKind::Popup(popup) => layout_popup(node, popup, bounds, state, out),
        UiNodeKind::RadialMenu(radial) => layout_radial_menu(node, radial, bounds, state, out),
        UiNodeKind::OverlayAdornment(adornment) => {
            layout_overlay_adornment(node, adornment, bounds, state, out)
        }
        UiNodeKind::Label(label) => layout_label(node, label, bounds, out),
        UiNodeKind::Button(button) => layout_button(node, button, bounds, out),
        UiNodeKind::TextInput(text_input) => layout_text_input(node, text_input, bounds, out),
        UiNodeKind::Toggle(toggle) => layout_toggle(node, toggle, bounds, out),
        UiNodeKind::NumericInput(numeric) => layout_numeric_input(node, numeric, bounds, out),
        UiNodeKind::Tabs(tabs) => layout_tabs(node, tabs, bounds, out),
        UiNodeKind::Select(select) => layout_select(node, select, bounds, out),
        UiNodeKind::Table(table) => layout_table(node, table, bounds, out),
        UiNodeKind::Tree(tree) => layout_tree(node, tree, bounds, out),
        UiNodeKind::Spacer(spacer) => layout_spacer(node, spacer, bounds, out),
        UiNodeKind::Divider(divider) => layout_divider(node, divider, bounds, out),
        UiNodeKind::Image(image) => layout_image(node, image, bounds, out),
        UiNodeKind::ProductSurface(surface) => layout_product_surface(node, surface, bounds, out),
        UiNodeKind::GraphCanvas(graph_canvas) => {
            layout_graph_canvas(node, graph_canvas, bounds, out)
        }
        UiNodeKind::ViewportSurfaceEmbed(embed) => {
            layout_viewport_surface_embed(node, embed, bounds, out)
        }
        UiNodeKind::Scroll(scroll) => layout_scroll(node, scroll, bounds, state, out),
        UiNodeKind::Stack(stack) => layout_stack(node, stack, bounds, state, out),
        UiNodeKind::Split(split) => layout_split(node, split, bounds, state, out),
    }
}
